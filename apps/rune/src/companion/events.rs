//! Event-stream parsing and translation, ported from the Swift app's `OpencodeEvents.swift`.

use super::types::*;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

/// One server event: `{ id, type, data }`.
#[derive(Clone, Debug, PartialEq)]
pub struct OpencodeEvent {
    pub id: Option<String>,
    pub kind: String,
    pub data: Map<String, Value>,
}

impl OpencodeEvent {
    pub fn parse(text: &str) -> Option<OpencodeEvent> {
        let json: Value = serde_json::from_str(text).ok()?;
        let kind = json.get("type")?.as_str()?.to_string();
        let data = json.get("data").and_then(Value::as_object).cloned().unwrap_or_default();
        Some(OpencodeEvent {
            id: json.get("id").and_then(Value::as_str).map(Into::into),
            kind,
            data,
        })
    }
}

/// Splits a server-sent event stream into `data:` payloads.
#[derive(Clone, Debug, Default)]
pub struct SseParser {
    buffer: String,
    data: Vec<String>,
}

impl SseParser {
    /// Feeds raw text, which may end mid-line. Returns the payloads it completed.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn feed(&mut self, chunk: &str) -> Vec<String> {
        self.buffer.push_str(chunk);
        let mut payloads = Vec::new();
        while let Some(at) = self.buffer.find('\n') {
            let line: String = self.buffer[..at].to_string();
            self.buffer.drain(..=at);
            if let Some(p) = self.feed_line(&line) {
                payloads.push(p);
            }
        }
        payloads
    }

    /// Feeds one line without its newline. Returns a payload when the line ends an event.
    pub fn feed_line(&mut self, raw: &str) -> Option<String> {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() {
            if self.data.is_empty() {
                return None;
            }
            let payload = self.data.join("\n");
            self.data.clear();
            return Some(payload);
        }
        if let Some(value) = line.strip_prefix("data:") {
            self.data.push(value.strip_prefix(' ').unwrap_or(value).to_string());
        }
        None
    }
}

// MARK: Translation

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    Failed,
    Interrupted,
}

impl Outcome {
    pub fn parse(s: &str) -> Outcome {
        match s {
            "succeeded" => Outcome::Succeeded,
            "failed" => Outcome::Failed,
            _ => Outcome::Interrupted,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnFinish {
    pub outcome: Outcome,
    pub error: Option<String>,
}

impl TurnFinish {
    pub fn new(outcome: Outcome) -> TurnFinish {
        TurnFinish { outcome, error: None }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TranslateResult {
    pub updates: Vec<Update>,
    /// A permission Mdow declines on the user's behalf, such as running shell commands:
    /// (permission id, reason).
    pub auto_reject: Option<(String, String)>,
    pub finish: Option<TurnFinish>,
    /// OpenCode began a run; the run's events and its finish belong to the newest prompt.
    pub started: bool,
}

impl TranslateResult {
    fn updates(updates: Vec<Update>) -> TranslateResult {
        TranslateResult { updates, ..Default::default() }
    }
}

pub fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// `path` made absolute against `directory`, with `.` and `..` resolved.
pub fn resolve(path: &str, directory: &str) -> String {
    if path.starts_with('/') {
        return path.to_string();
    }
    let mut out = PathBuf::from("/");
    for c in Path::new(directory).join(path).components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(n) => out.push(n),
            _ => {}
        }
    }
    out.to_string_lossy().into_owned()
}

const MAX_DETAIL_CHARS: usize = 4_000;

/// Tools that change files. Mdow shows these as reviewable change cards, not tool rows.
const WRITE_TOOLS: [&str; 5] = ["edit", "write", "patch", "apply_patch", "multiedit"];

fn is_write_tool(name: &str) -> bool {
    WRITE_TOOLS.contains(&name)
}

fn clip(text: &str) -> String {
    if text.chars().count() > MAX_DETAIL_CHARS {
        let cut: String = text.chars().take(MAX_DETAIL_CHARS).collect();
        format!("{cut}\n…")
    } else {
        text.to_string()
    }
}

fn format_input(input: Option<&Value>) -> Option<String> {
    match input? {
        Value::String(s) => Some(clip(s)),
        other => Some(clip(&serde_json::to_string_pretty(other).unwrap_or_default())),
    }
}

fn text_from_content(content: Option<&Value>) -> String {
    match content {
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn input_path(input: Option<&Value>) -> Option<String> {
    let input = input?.as_object()?;
    ["path", "filePath", "file"]
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
        .map(Into::into)
}

fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A plain-words title for a tool call, such as "Read notes.md".
pub fn tool_title(name: &str, input: Option<&Value>) -> String {
    let field = |k: &str| input.and_then(|i| i.get(k)).and_then(Value::as_str).map(str::to_string);
    match name {
        "read" => input_path(input)
            .map(|p| format!("Read {}", basename(&p)))
            .unwrap_or_else(|| "Read a document".into()),
        "grep" | "search" => match field("pattern").or_else(|| field("query")) {
            Some(p) => format!("Searched for “{p}”"),
            None => "Searched the folder".into(),
        },
        "glob" | "list" | "ls" | "find" => "Looked through the folder".into(),
        "webfetch" => {
            field("url").map(|u| format!("Fetched {u}")).unwrap_or_else(|| "Fetched a page".into())
        }
        "websearch" => field("query")
            .map(|q| format!("Searched the web for “{q}”"))
            .unwrap_or_else(|| "Searched the web".into()),
        "todowrite" | "todoread" => "Updated its plan".into(),
        _ => capitalize(name),
    }
}

/// The files in an edit permission's or tool result's `metadata.files`.
pub fn file_changes(metadata: Option<&Value>, directory: &str) -> Vec<FileChange> {
    let Some(files) = metadata.and_then(|m| m.get("files")).and_then(Value::as_array) else {
        return Vec::new();
    };
    files
        .iter()
        .filter_map(|entry| {
            let entry = entry.as_object()?;
            let file = entry.get("file").or_else(|| entry.get("path"))?.as_str()?;
            let status = match entry.get("status").and_then(Value::as_str) {
                Some("added") => FileStatus::Added,
                Some("deleted") => FileStatus::Deleted,
                _ => FileStatus::Modified,
            };
            let count = |k: &str| entry.get(k).and_then(Value::as_f64).unwrap_or(0.0) as i64;
            Some(FileChange {
                path: resolve(file, directory),
                display_path: file.to_string(),
                patch: entry.get("patch").and_then(Value::as_str).unwrap_or("").to_string(),
                additions: count("additions"),
                deletions: count("deletions"),
                status,
            })
        })
        .collect()
}

/// Translates the OpenCode event stream for one session into companion updates. One instance
/// lives as long as the session, so tool names and pending permissions carry across turns.
pub struct Translator {
    pub session_id: String,
    pub directory: String,
    tool_names: HashMap<String, String>,
    tool_inputs: HashMap<String, Value>,
    permission_tools: HashMap<String, String>,
    rejected: HashSet<String>,
    cited: HashSet<String>,
    reasoning_open: bool,
}

impl Translator {
    pub fn new(session_id: &str, directory: &str) -> Translator {
        Translator {
            session_id: session_id.into(),
            directory: directory.into(),
            tool_names: HashMap::new(),
            tool_inputs: HashMap::new(),
            permission_tools: HashMap::new(),
            rejected: HashSet::new(),
            cited: HashSet::new(),
            reasoning_open: false,
        }
    }

    /// Clears per-turn state; call before each prompt.
    pub fn begin_turn(&mut self) {
        self.cited.clear();
        self.reasoning_open = false;
    }

    pub fn translate(&mut self, event: &OpencodeEvent) -> TranslateResult {
        let d = &event.data;
        if d.get("sessionID").and_then(Value::as_str) != Some(self.session_id.as_str()) {
            return TranslateResult::default();
        }
        let s = |k: &str| {
            d.get(k).and_then(Value::as_str).filter(|v| !v.is_empty()).map(str::to_string)
        };
        match event.kind.as_str() {
            "session.text.delta" => match s("delta") {
                Some(delta) => {
                    let mut updates = self.close_reasoning();
                    updates.push(Update::Delta(delta));
                    TranslateResult::updates(updates)
                }
                None => TranslateResult::default(),
            },
            "session.reasoning.delta" => match s("delta") {
                Some(delta) => {
                    self.reasoning_open = true;
                    TranslateResult::updates(vec![Update::Thinking(delta)])
                }
                None => TranslateResult::default(),
            },
            "session.reasoning.ended" => TranslateResult::updates(self.close_reasoning()),
            "session.tool.input.started" => self.tool_started(s("id"), s("name")),
            "session.tool.called" => self.tool_called(s("id"), d.get("input")),
            "session.tool.success" => {
                self.tool_succeeded(s("id"), d.get("content"), d.get("metadata"))
            }
            "session.tool.failed" => self.tool_failed(s("id"), d.get("error")),
            "permission.asked" => self.permission_asked(d),
            "permission.replied" => self.permission_replied(s("requestID"), s("reply")),
            "session.execution.started" => TranslateResult { started: true, ..Default::default() },
            "session.execution.succeeded" => TranslateResult {
                updates: self.close_reasoning(),
                finish: Some(TurnFinish::new(Outcome::Succeeded)),
                ..Default::default()
            },
            "session.execution.failed" => {
                let error = d
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                TranslateResult {
                    updates: self.close_reasoning(),
                    finish: Some(TurnFinish {
                        outcome: Outcome::Failed,
                        error: Some(
                            error.unwrap_or_else(|| {
                                "OpenCode could not finish the response.".into()
                            }),
                        ),
                    }),
                    ..Default::default()
                }
            }
            kind if kind.starts_with("session.execution.") => TranslateResult {
                updates: self.close_reasoning(),
                finish: Some(TurnFinish::new(Outcome::Interrupted)),
                ..Default::default()
            },
            _ => TranslateResult::default(),
        }
    }

    fn close_reasoning(&mut self) -> Vec<Update> {
        if !self.reasoning_open {
            return Vec::new();
        }
        self.reasoning_open = false;
        vec![Update::ThinkingDone]
    }

    fn name_of(&self, id: &str) -> String {
        self.tool_names.get(id).cloned().unwrap_or_else(|| "tool".into())
    }

    fn tool_started(&mut self, id: Option<String>, name: Option<String>) -> TranslateResult {
        let (Some(id), Some(name)) = (id, name) else { return TranslateResult::default() };
        self.tool_names.insert(id.clone(), name.clone());
        let mut updates = self.close_reasoning();
        if !is_write_tool(&name) {
            updates.push(Update::Tool(ToolPart::new(
                id,
                tool_title(&name, None),
                ToolState::Pending,
            )));
        }
        TranslateResult::updates(updates)
    }

    fn tool_called(&mut self, id: Option<String>, input: Option<&Value>) -> TranslateResult {
        let Some(id) = id else { return TranslateResult::default() };
        let name = self.name_of(&id);
        if is_write_tool(&name) {
            return TranslateResult::default();
        }
        if let Some(input) = input {
            self.tool_inputs.insert(id.clone(), input.clone());
        }
        let mut updates = vec![Update::Tool(
            ToolPart::new(id.clone(), tool_title(&name, input), ToolState::Running)
                .input(format_input(input)),
        )];
        if name == "read"
            && let Some(path) = input_path(input)
            && crate::files::is_document(Path::new(&path))
        {
            let absolute = resolve(&path, &self.directory);
            if self.cited.insert(absolute.clone()) {
                updates.push(Update::Citation(Citation {
                    source_id: format!("read:{absolute}"),
                    label: basename(&absolute),
                    path: absolute,
                }));
            }
        }
        TranslateResult::updates(updates)
    }

    fn tool_succeeded(
        &mut self,
        id: Option<String>,
        content: Option<&Value>,
        metadata: Option<&Value>,
    ) -> TranslateResult {
        let Some(id) = id else { return TranslateResult::default() };
        let name = self.name_of(&id);
        let files = file_changes(metadata, &self.directory);
        if is_write_tool(&name) || !files.is_empty() {
            return TranslateResult::updates(vec![Update::Change {
                tool_call_id: id,
                permission_id: None,
                status: ChangeStatus::Applied,
                files: (!files.is_empty()).then_some(files),
                error: None,
            }]);
        }
        let title = tool_title(&name, self.tool_inputs.get(&id));
        TranslateResult::updates(vec![Update::Tool(
            ToolPart::new(id, title, ToolState::Completed)
                .output(Some(clip(&text_from_content(content)))),
        )])
    }

    fn tool_failed(&mut self, id: Option<String>, error: Option<&Value>) -> TranslateResult {
        let Some(id) = id else { return TranslateResult::default() };
        let name = self.name_of(&id);
        let message = error
            .and_then(|e| e.get("message").and_then(Value::as_str).or_else(|| e.as_str()))
            .unwrap_or("Tool failed")
            .to_string();
        if is_write_tool(&name) {
            let rejected = self
                .permission_tools
                .iter()
                .any(|(permission, tool)| *tool == id && self.rejected.contains(permission));
            return TranslateResult::updates(vec![if rejected {
                Update::change(id, ChangeStatus::Rejected)
            } else {
                Update::Change {
                    tool_call_id: id,
                    permission_id: None,
                    status: ChangeStatus::Failed,
                    files: None,
                    error: Some(message),
                }
            }]);
        }
        let title = tool_title(&name, self.tool_inputs.get(&id));
        TranslateResult::updates(vec![Update::Tool(
            ToolPart::new(id, title, ToolState::Error).error(Some(message)),
        )])
    }

    fn permission_asked(&mut self, d: &Map<String, Value>) -> TranslateResult {
        let Some(permission_id) =
            d.get("id").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string)
        else {
            return TranslateResult::default();
        };
        let action = d.get("action").and_then(Value::as_str).unwrap_or("unknown").to_string();
        let tool_call_id = d
            .get("source")
            .and_then(|s| s.get("id"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| permission_id.clone());
        self.permission_tools.insert(permission_id.clone(), tool_call_id.clone());

        if action == "edit" || action == "write" {
            let files = file_changes(d.get("metadata"), &self.directory);
            let mut updates = self.close_reasoning();
            updates.push(Update::Change {
                tool_call_id,
                permission_id: Some(permission_id),
                status: ChangeStatus::Pending,
                files: Some(files),
                error: None,
            });
            return TranslateResult::updates(updates);
        }

        // Mdow is a document companion: anything beyond reading and editing documents in the
        // open folder (shell commands, other folders, the network) is declined automatically.
        let resources: Vec<&str> = d
            .get("resources")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let target = if resources.is_empty() {
            String::new()
        } else {
            format!(" ({})", resources.join(", "))
        };
        TranslateResult {
            auto_reject: Some((
                permission_id,
                format!(
                    "Mdow does not allow the {action} permission{target}. Work only with the documents in this folder."
                ),
            )),
            ..Default::default()
        }
    }

    fn permission_replied(
        &mut self,
        permission_id: Option<String>,
        reply: Option<String>,
    ) -> TranslateResult {
        let Some(permission_id) = permission_id else { return TranslateResult::default() };
        let Some(tool_call_id) = self.permission_tools.get(&permission_id).cloned() else {
            return TranslateResult::default();
        };
        if reply.as_deref() != Some("reject") {
            return TranslateResult::default();
        }
        self.rejected.insert(permission_id);
        if let Some(name) = self.tool_names.get(&tool_call_id)
            && !is_write_tool(name)
        {
            return TranslateResult::default();
        }
        TranslateResult::updates(vec![Update::change(tool_call_id, ChangeStatus::Rejected)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SESSION: &str = "ses_test";
    const DIR: &str = "/docs";
    const PATCH: &str = "--- notes.md\n+++ notes.md\n@@ -1 +1 @@\n-teh\n+the\n";

    fn event(kind: &str, data: Value) -> OpencodeEvent {
        let mut data = data.as_object().cloned().unwrap_or_default();
        data.insert("sessionID".into(), json!(SESSION));
        OpencodeEvent { id: None, kind: kind.into(), data }
    }

    fn translator() -> Translator {
        Translator::new(SESSION, DIR)
    }

    #[test]
    fn ignores_events_from_other_sessions() {
        let mut t = translator();
        let e = OpencodeEvent {
            id: None,
            kind: "session.text.delta".into(),
            data: json!({ "sessionID": "ses_other", "delta": "hi" }).as_object().cloned().unwrap(),
        };
        assert!(t.translate(&e).updates.is_empty());
    }

    #[test]
    fn streams_reasoning_then_closes_it_when_text_starts() {
        let mut t = translator();
        assert_eq!(
            t.translate(&event("session.reasoning.delta", json!({ "delta": "Hmm" }))).updates,
            vec![Update::Thinking("Hmm".into())]
        );
        assert_eq!(
            t.translate(&event("session.text.delta", json!({ "delta": "Fixed." }))).updates,
            vec![Update::ThinkingDone, Update::Delta("Fixed.".into())]
        );
    }

    #[test]
    fn shows_reads_as_titled_tool_rows_and_cites_the_document() {
        let mut t = translator();
        t.translate(&event(
            "session.tool.input.started",
            json!({ "id": "call_1", "name": "read" }),
        ));
        let called = t.translate(&event(
            "session.tool.called",
            json!({ "id": "call_1", "input": { "path": "/docs/notes.md" } }),
        ));
        let Some(Update::Tool(tool)) = called.updates.first() else {
            panic!("expected a tool row")
        };
        assert_eq!(tool.tool_call_id, "call_1");
        assert_eq!(tool.name, "Read notes.md");
        assert_eq!(tool.state, ToolState::Running);
        assert_eq!(
            called.updates.last(),
            Some(&Update::Citation(Citation {
                source_id: "read:/docs/notes.md".into(),
                path: "/docs/notes.md".into(),
                label: "notes.md".into()
            }))
        );
        let done = t.translate(&event(
            "session.tool.success",
            json!({ "id": "call_1", "content": [{ "type": "text", "text": "1: # Notes" }] }),
        ));
        assert_eq!(
            done.updates,
            vec![Update::Tool(
                ToolPart::new("call_1", "Read notes.md", ToolState::Completed)
                    .output(Some("1: # Notes".into()))
            )]
        );
    }

    #[test]
    fn relative_reads_are_resolved_and_cited_once() {
        let mut t = translator();
        t.translate(&event("session.tool.input.started", json!({ "id": "a", "name": "read" })));
        let first = t.translate(&event(
            "session.tool.called",
            json!({ "id": "a", "input": { "path": "guide/intro.md" } }),
        ));
        assert_eq!(first.updates.len(), 2);
        assert!(
            matches!(&first.updates[1], Update::Citation(c) if c.path == "/docs/guide/intro.md")
        );
        t.translate(&event("session.tool.input.started", json!({ "id": "b", "name": "read" })));
        let second = t.translate(&event(
            "session.tool.called",
            json!({ "id": "b", "input": { "path": "guide/intro.md" } }),
        ));
        assert_eq!(second.updates.len(), 1);
    }

    #[test]
    fn turns_an_edit_permission_into_a_pending_change_card_then_applies_it() {
        let mut t = translator();
        assert!(
            t.translate(&event(
                "session.tool.input.started",
                json!({ "id": "call_2", "name": "edit" })
            ))
            .updates
            .is_empty()
        );
        let asked = t.translate(&event(
            "permission.asked",
            json!({
                "id": "per_1", "action": "edit", "resources": ["notes.md"],
                "metadata": { "files": [{ "file": "notes.md", "patch": PATCH, "status": "modified", "additions": 1, "deletions": 1 }] },
                "source": { "type": "tool", "id": "call_2" },
            }),
        ));
        assert_eq!(
            asked.updates,
            vec![Update::Change {
                tool_call_id: "call_2".into(),
                permission_id: Some("per_1".into()),
                status: ChangeStatus::Pending,
                files: Some(vec![FileChange {
                    path: "/docs/notes.md".into(),
                    display_path: "notes.md".into(),
                    patch: PATCH.into(),
                    additions: 1,
                    deletions: 1,
                    status: FileStatus::Modified,
                }]),
                error: None,
            }]
        );
        let applied = t.translate(&event(
            "session.tool.success",
            json!({
                "id": "call_2", "content": [{ "type": "text", "text": "Edited notes.md" }],
                "metadata": { "files": [{ "file": "notes.md", "patch": PATCH, "additions": 1, "deletions": 1 }] },
            }),
        ));
        let Some(Update::Change { tool_call_id, status, .. }) = applied.updates.first() else {
            panic!("expected a change")
        };
        assert_eq!(tool_call_id, "call_2");
        assert_eq!(*status, ChangeStatus::Applied);
    }

    #[test]
    fn marks_a_rejected_edit_as_rejected_rather_than_failed() {
        let mut t = translator();
        t.translate(&event(
            "session.tool.input.started",
            json!({ "id": "call_3", "name": "edit" }),
        ));
        t.translate(&event(
            "permission.asked",
            json!({ "id": "per_2", "action": "edit", "source": { "id": "call_3" } }),
        ));
        assert_eq!(
            t.translate(&event(
                "permission.replied",
                json!({ "requestID": "per_2", "reply": "reject" })
            ))
            .updates,
            vec![Update::change("call_3", ChangeStatus::Rejected)]
        );
        assert_eq!(
            t.translate(&event(
                "session.tool.failed",
                json!({ "id": "call_3", "error": { "message": "Rejected" } })
            ))
            .updates,
            vec![Update::change("call_3", ChangeStatus::Rejected)]
        );
    }

    #[test]
    fn reports_failed_edits_with_their_error() {
        let mut t = translator();
        t.translate(&event("session.tool.input.started", json!({ "id": "c", "name": "patch" })));
        assert_eq!(
            t.translate(&event(
                "session.tool.failed",
                json!({ "id": "c", "error": { "message": "No match" } })
            ))
            .updates,
            vec![Update::Change {
                tool_call_id: "c".into(),
                permission_id: None,
                status: ChangeStatus::Failed,
                files: None,
                error: Some("No match".into())
            }]
        );
    }

    #[test]
    fn declines_shell_permissions_automatically() {
        let mut t = translator();
        let result =
            t.translate(&event("permission.asked", json!({ "id": "per_3", "action": "shell", "resources": ["ls"], "source": { "id": "call_4" } })));
        assert!(result.updates.is_empty());
        let (id, reason) = result.auto_reject.unwrap();
        assert_eq!(id, "per_3");
        assert!(reason.contains("shell"));
        assert!(reason.contains("(ls)"));
    }

    #[test]
    fn reports_how_the_turn_ended() {
        let mut t = translator();
        assert_eq!(
            t.translate(&event("session.execution.succeeded", json!({}))).finish,
            Some(TurnFinish::new(Outcome::Succeeded))
        );
        assert_eq!(
            t.translate(&event(
                "session.execution.failed",
                json!({ "error": { "type": "provider.auth", "message": "Sign in again" } })
            ))
            .finish,
            Some(TurnFinish { outcome: Outcome::Failed, error: Some("Sign in again".into()) })
        );
        assert_eq!(
            t.translate(&event("session.execution.interrupted", json!({}))).finish,
            Some(TurnFinish::new(Outcome::Interrupted))
        );
        let started = t.translate(&event("session.execution.started", json!({})));
        assert_eq!(started.finish, None);
        assert!(started.started);
    }

    #[test]
    fn tool_titles_use_plain_words() {
        assert_eq!(
            tool_title("grep", Some(&json!({ "pattern": "roadmap" }))),
            "Searched for “roadmap”"
        );
        assert_eq!(
            tool_title("glob", Some(&json!({ "pattern": "**/*.md" }))),
            "Looked through the folder"
        );
        assert_eq!(
            tool_title("websearch", Some(&json!({ "query": "swift" }))),
            "Searched the web for “swift”"
        );
        assert_eq!(tool_title("mystery", None), "Mystery");
    }

    #[test]
    fn file_changes_resolve_relative_paths_and_skip_malformed_entries() {
        assert_eq!(
            file_changes(
                Some(
                    &json!({ "files": [{ "file": "a.md", "status": "added" }, { "nope": true }, "x"] })
                ),
                DIR
            ),
            vec![FileChange {
                path: "/docs/a.md".into(),
                display_path: "a.md".into(),
                patch: String::new(),
                additions: 0,
                deletions: 0,
                status: FileStatus::Added
            }]
        );
        assert!(file_changes(None, DIR).is_empty());
        assert_eq!(resolve("../b/./c.md", "/docs/x"), "/docs/b/c.md");
    }

    #[test]
    fn sse_parser_joins_data_lines_and_handles_split_chunks() {
        let mut parser = SseParser::default();
        let mut received = parser.feed("event: message\ndata: {\"type\":\"a\",");
        received.extend(parser.feed("\"data\":{}}\n\ndata: one\r\ndata: two\n\n: heartbeat\n\n"));
        assert_eq!(
            received,
            vec!["{\"type\":\"a\",\"data\":{}}".to_string(), "one\ntwo".to_string()]
        );
    }

    #[test]
    fn parses_typed_events_and_drops_malformed_payloads() {
        assert_eq!(
            OpencodeEvent::parse(
                "{\"id\":\"evt_1\",\"type\":\"session.idle\",\"data\":{\"sessionID\":\"s\"}}"
            ),
            Some(OpencodeEvent {
                id: Some("evt_1".into()),
                kind: "session.idle".into(),
                data: json!({ "sessionID": "s" }).as_object().cloned().unwrap()
            })
        );
        assert_eq!(OpencodeEvent::parse("{\"data\":{}}"), None);
        assert_eq!(OpencodeEvent::parse("not json"), None);
    }
}
