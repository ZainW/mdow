//! OpenCode's event stream: server-sent events, and translating one session's events into
//! companion updates.

use super::types::*;
use crate::files;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// One server event: `{ id, type, data }`.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub id: Option<String>,
    pub kind: String,
    pub data: Map<String, Value>,
}

impl Event {
    pub fn parse(text: &str) -> Option<Event> {
        let json: Value = serde_json::from_str(text).ok()?;
        let kind = json.get("type")?.as_str()?.to_string();
        Some(Event {
            id: json.get("id").and_then(Value::as_str).map(str::to_string),
            kind,
            data: json.get("data").and_then(Value::as_object).cloned().unwrap_or_default(),
        })
    }
}

/// Splits a server-sent event stream into `data:` payloads.
#[derive(Default)]
pub struct SseParser {
    data_lines: Vec<String>,
}

impl SseParser {
    /// Feeds one line without its newline. Returns a payload when the line ends an event.
    pub fn feed_line(&mut self, raw: &str) -> Option<String> {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() {
            if self.data_lines.is_empty() {
                return None;
            }
            let payload = self.data_lines.join("\n");
            self.data_lines.clear();
            return Some(payload);
        }
        if let Some(value) = line.strip_prefix("data:") {
            self.data_lines.push(value.strip_prefix(' ').unwrap_or(value).to_string());
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnFinish {
    pub outcome: Outcome,
    pub error: Option<String>,
}

impl TurnFinish {
    fn new(outcome: Outcome) -> TurnFinish {
        TurnFinish { outcome, error: None }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TranslateResult {
    pub updates: Vec<Update>,
    /// A permission Mdow declines on the user's behalf, such as running shell commands.
    pub auto_reject: Option<(String, String)>,
    pub finish: Option<TurnFinish>,
    /// OpenCode began a run; the run's events and its finish belong to the newest prompt.
    pub started: bool,
}

fn updates(updates: Vec<Update>) -> TranslateResult {
    TranslateResult { updates, ..TranslateResult::default() }
}

fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |name| name.to_string_lossy().into_owned())
}

fn resolve(path: &str, directory: &str) -> String {
    if path.starts_with('/') {
        return path.to_string();
    }
    files::normalize(&PathBuf::from(directory).join(path)).to_string_lossy().into_owned()
}

const MAX_DETAIL_CHARS: usize = 4_000;

/// Tools that change files. Mdow shows these as reviewable change cards, not tool rows.
fn is_write_tool(name: &str) -> bool {
    matches!(name, "edit" | "write" | "patch" | "apply_patch" | "multiedit")
}

fn clip(text: &str) -> String {
    match text.char_indices().nth(MAX_DETAIL_CHARS) {
        Some((end, _)) => format!("{}\n…", &text[..end]),
        None => text.to_string(),
    }
}

fn format_input(input: Option<&Value>) -> Option<String> {
    let input = input?;
    if let Some(text) = input.as_str() {
        return Some(clip(text));
    }
    serde_json::to_string_pretty(input).ok().map(|text| clip(&text))
}

fn text_from_content(content: Option<&Value>) -> String {
    let Some(content) = content else { return String::new() };
    match content.as_array() {
        Some(blocks) => blocks
            .iter()
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        None => content.as_str().unwrap_or_default().to_string(),
    }
}

fn str_of<'a>(value: Option<&'a Value>, key: &str) -> Option<&'a str> {
    value?.get(key)?.as_str()
}

fn input_path(input: Option<&Value>) -> Option<&str> {
    let input = input.filter(|v| v.is_object());
    str_of(input, "path").or_else(|| str_of(input, "filePath")).or_else(|| str_of(input, "file"))
}

/// A plain-words title for a tool call, such as "Read notes.md".
pub fn tool_title(name: &str, input: Option<&Value>) -> String {
    match name {
        "read" => input_path(input).map_or_else(
            || "Read a document".to_string(),
            |path| format!("Read {}", basename(path)),
        ),
        "grep" | "search" => str_of(input, "pattern")
            .or_else(|| str_of(input, "query"))
            .map_or_else(|| "Searched the folder".to_string(), |p| format!("Searched for “{p}”")),
        "glob" | "list" | "ls" | "find" => "Looked through the folder".to_string(),
        "webfetch" => str_of(input, "url")
            .map_or_else(|| "Fetched a page".to_string(), |url| format!("Fetched {url}")),
        "websearch" => str_of(input, "query").map_or_else(
            || "Searched the web".to_string(),
            |q| format!("Searched the web for “{q}”"),
        ),
        "todowrite" | "todoread" => "Updated its plan".to_string(),
        _ => {
            let mut chars = name.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
    }
}

/// The files in an edit permission's or tool result's `metadata.files`.
pub fn file_changes(metadata: Option<&Value>, directory: &str) -> Vec<FileChange> {
    let Some(entries) = metadata.and_then(|m| m.get("files")).and_then(Value::as_array) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let file = entry.get("file").or_else(|| entry.get("path"))?.as_str()?;
            let status = match entry.get("status").and_then(Value::as_str) {
                Some("added") => FileStatus::Added,
                Some("deleted") => FileStatus::Deleted,
                _ => FileStatus::Modified,
            };
            let count = |key: &str| entry.get(key).and_then(Value::as_f64).unwrap_or(0.0) as i64;
            Some(FileChange {
                path: resolve(file, directory),
                display_path: file.to_string(),
                patch: entry.get("patch").and_then(Value::as_str).unwrap_or_default().to_string(),
                additions: count("additions"),
                deletions: count("deletions"),
                status,
            })
        })
        .collect()
}

/// Translates the event stream for one session into companion updates. One instance lives as
/// long as the session, so tool names and pending permissions carry across turns.
pub struct TurnTranslator {
    session_id: String,
    directory: String,
    tool_names: HashMap<String, String>,
    tool_inputs: HashMap<String, Value>,
    permission_tools: HashMap<String, String>,
    rejected_permissions: HashSet<String>,
    cited: HashSet<String>,
    reasoning_open: bool,
}

impl TurnTranslator {
    pub fn new(session_id: &str, directory: &str) -> TurnTranslator {
        TurnTranslator {
            session_id: session_id.to_string(),
            directory: directory.to_string(),
            tool_names: HashMap::new(),
            tool_inputs: HashMap::new(),
            permission_tools: HashMap::new(),
            rejected_permissions: HashSet::new(),
            cited: HashSet::new(),
            reasoning_open: false,
        }
    }

    /// Clears per-turn state; call before each prompt.
    pub fn begin_turn(&mut self) {
        self.cited.clear();
        self.reasoning_open = false;
    }

    pub fn translate(&mut self, event: &Event) -> TranslateResult {
        let data = &event.data;
        if data.get("sessionID").and_then(Value::as_str) != Some(self.session_id.as_str()) {
            return TranslateResult::default();
        }
        let text = |key: &str| data.get(key).and_then(Value::as_str);
        match event.kind.as_str() {
            "session.text.delta" => match text("delta").filter(|d| !d.is_empty()) {
                Some(delta) => {
                    let mut list = self.close_reasoning();
                    list.push(Update::Delta(delta.to_string()));
                    updates(list)
                }
                None => TranslateResult::default(),
            },
            "session.reasoning.delta" => match text("delta").filter(|d| !d.is_empty()) {
                Some(delta) => {
                    self.reasoning_open = true;
                    updates(vec![Update::Thinking(delta.to_string())])
                }
                None => TranslateResult::default(),
            },
            "session.reasoning.ended" => updates(self.close_reasoning()),
            "session.tool.input.started" => self.tool_started(text("id"), text("name")),
            "session.tool.called" => self.tool_called(text("id"), data.get("input")),
            "session.tool.success" => {
                self.tool_succeeded(text("id"), data.get("content"), data.get("metadata"))
            }
            "session.tool.failed" => self.tool_failed(text("id"), data.get("error")),
            "permission.asked" => self.permission_asked(data),
            "permission.replied" => self.permission_replied(text("requestID"), text("reply")),
            "session.execution.started" => {
                TranslateResult { started: true, ..TranslateResult::default() }
            }
            "session.execution.succeeded" => TranslateResult {
                updates: self.close_reasoning(),
                finish: Some(TurnFinish::new(Outcome::Succeeded)),
                ..TranslateResult::default()
            },
            "session.execution.failed" => {
                let error = str_of(data.get("error"), "message")
                    .unwrap_or("OpenCode could not finish the response.")
                    .to_string();
                TranslateResult {
                    updates: self.close_reasoning(),
                    finish: Some(TurnFinish { outcome: Outcome::Failed, error: Some(error) }),
                    ..TranslateResult::default()
                }
            }
            kind if kind.starts_with("session.execution.") => TranslateResult {
                updates: self.close_reasoning(),
                finish: Some(TurnFinish::new(Outcome::Interrupted)),
                ..TranslateResult::default()
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

    fn tool_started(&mut self, id: Option<&str>, name: Option<&str>) -> TranslateResult {
        let (Some(id), Some(name)) = (id.filter(|s| !s.is_empty()), name.filter(|s| !s.is_empty()))
        else {
            return TranslateResult::default();
        };
        self.tool_names.insert(id.to_string(), name.to_string());
        let mut list = self.close_reasoning();
        if !is_write_tool(name) {
            list.push(Update::Tool(ToolPart::new(id, &tool_title(name, None), ToolState::Pending)));
        }
        updates(list)
    }

    fn tool_name(&self, id: &str) -> String {
        self.tool_names.get(id).cloned().unwrap_or_else(|| "tool".to_string())
    }

    fn tool_called(&mut self, id: Option<&str>, input: Option<&Value>) -> TranslateResult {
        let Some(id) = id.filter(|s| !s.is_empty()) else { return TranslateResult::default() };
        let name = self.tool_name(id);
        if is_write_tool(&name) {
            return TranslateResult::default();
        }
        if let Some(input) = input {
            self.tool_inputs.insert(id.to_string(), input.clone());
        }
        let mut tool = ToolPart::new(id, &tool_title(&name, input), ToolState::Running);
        tool.input = format_input(input);
        let mut list = vec![Update::Tool(tool)];
        if name == "read"
            && let Some(path) = input_path(input).filter(|p| files::is_document(Path::new(p)))
        {
            let absolute = resolve(path, &self.directory);
            if self.cited.insert(absolute.clone()) {
                list.push(Update::Citation(Citation {
                    source_id: format!("read:{absolute}"),
                    label: basename(&absolute),
                    path: absolute,
                }));
            }
        }
        updates(list)
    }

    fn tool_succeeded(
        &mut self,
        id: Option<&str>,
        content: Option<&Value>,
        metadata: Option<&Value>,
    ) -> TranslateResult {
        let Some(id) = id.filter(|s| !s.is_empty()) else { return TranslateResult::default() };
        let name = self.tool_name(id);
        let changed = file_changes(metadata, &self.directory);
        if is_write_tool(&name) || !changed.is_empty() {
            return updates(vec![Update::Change {
                tool_call_id: id.to_string(),
                permission_id: None,
                status: ChangeStatus::Applied,
                files: (!changed.is_empty()).then_some(changed),
                error: None,
            }]);
        }
        let mut tool =
            ToolPart::new(id, &tool_title(&name, self.tool_inputs.get(id)), ToolState::Completed);
        tool.output = Some(clip(&text_from_content(content)));
        updates(vec![Update::Tool(tool)])
    }

    fn tool_failed(&mut self, id: Option<&str>, error: Option<&Value>) -> TranslateResult {
        let Some(id) = id.filter(|s| !s.is_empty()) else { return TranslateResult::default() };
        let name = self.tool_name(id);
        let message = str_of(error, "message")
            .or_else(|| error.and_then(Value::as_str))
            .unwrap_or("Tool failed")
            .to_string();
        if is_write_tool(&name) {
            let rejected = self.permission_tools.iter().any(|(permission, tool)| {
                tool == id && self.rejected_permissions.contains(permission)
            });
            return updates(vec![if rejected {
                Update::change(id, ChangeStatus::Rejected)
            } else {
                Update::Change {
                    tool_call_id: id.to_string(),
                    permission_id: None,
                    status: ChangeStatus::Failed,
                    files: None,
                    error: Some(message),
                }
            }]);
        }
        let mut tool =
            ToolPart::new(id, &tool_title(&name, self.tool_inputs.get(id)), ToolState::Error);
        tool.error = Some(message);
        updates(vec![Update::Tool(tool)])
    }

    fn permission_asked(&mut self, data: &Map<String, Value>) -> TranslateResult {
        let Some(permission_id) =
            data.get("id").and_then(Value::as_str).filter(|id| !id.is_empty())
        else {
            return TranslateResult::default();
        };
        let action = data.get("action").and_then(Value::as_str).unwrap_or("unknown");
        let tool_call_id = str_of(data.get("source"), "id").unwrap_or(permission_id).to_string();

        if action == "edit" || action == "write" {
            self.permission_tools.insert(permission_id.to_string(), tool_call_id.clone());
            let changed = file_changes(data.get("metadata"), &self.directory);
            let mut list = self.close_reasoning();
            list.push(Update::Change {
                tool_call_id,
                permission_id: Some(permission_id.to_string()),
                status: ChangeStatus::Pending,
                files: Some(changed),
                error: None,
            });
            return updates(list);
        }

        // Mdow is a document companion: anything beyond reading and editing documents in the
        // open folder (shell commands, other folders, the network) is declined automatically.
        let resources: Vec<&str> = data
            .get("resources")
            .and_then(Value::as_array)
            .map(|list| list.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let target = if resources.is_empty() {
            String::new()
        } else {
            format!(" ({})", resources.join(", "))
        };
        TranslateResult {
            auto_reject: Some((
                permission_id.to_string(),
                format!(
                    "Mdow does not allow the {action} permission{target}. Work only with the documents in this folder."
                ),
            )),
            ..TranslateResult::default()
        }
    }

    fn permission_replied(&mut self, id: Option<&str>, reply: Option<&str>) -> TranslateResult {
        let Some(id) = id else { return TranslateResult::default() };
        let Some(tool_call_id) = self.permission_tools.get(id).cloned() else {
            return TranslateResult::default();
        };
        if reply != Some("reject") {
            return TranslateResult::default();
        }
        self.rejected_permissions.insert(id.to_string());
        if self.tool_names.get(&tool_call_id).is_some_and(|name| !is_write_tool(name)) {
            return TranslateResult::default();
        }
        updates(vec![Update::change(&tool_call_id, ChangeStatus::Rejected)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SESSION: &str = "ses_test";
    const DIR: &str = "/docs";
    const PATCH: &str = "--- notes.md\n+++ notes.md\n@@ -1 +1 @@\n-teh\n+the\n";

    fn event(kind: &str, data: Value) -> Event {
        let mut data = data.as_object().cloned().unwrap_or_default();
        data.insert("sessionID".to_string(), json!(SESSION));
        Event { id: None, kind: kind.to_string(), data }
    }

    fn translator() -> TurnTranslator {
        TurnTranslator::new(SESSION, DIR)
    }

    #[test]
    fn ignores_events_from_other_sessions() {
        let other = Event {
            id: None,
            kind: "session.text.delta".to_string(),
            data: json!({"sessionID": "ses_other", "delta": "hi"}).as_object().unwrap().clone(),
        };
        assert!(translator().translate(&other).updates.is_empty());
    }

    #[test]
    fn streams_reasoning_then_closes_it_when_text_starts() {
        let mut t = translator();
        assert_eq!(
            t.translate(&event("session.reasoning.delta", json!({"delta": "Hmm"}))).updates,
            [Update::Thinking("Hmm".to_string())]
        );
        assert_eq!(
            t.translate(&event("session.text.delta", json!({"delta": "Fixed."}))).updates,
            [Update::ThinkingDone, Update::Delta("Fixed.".to_string())]
        );
    }

    #[test]
    fn shows_reads_as_titled_tool_rows_and_cites_the_document() {
        let mut t = translator();
        t.translate(&event("session.tool.input.started", json!({"id": "call_1", "name": "read"})));
        let called = t.translate(&event(
            "session.tool.called",
            json!({"id": "call_1", "input": {"path": "/docs/notes.md"}}),
        ));
        let Update::Tool(tool) = &called.updates[0] else { panic!("expected a tool row") };
        assert_eq!(tool.tool_call_id, "call_1");
        assert_eq!(tool.name, "Read notes.md");
        assert_eq!(tool.state, ToolState::Running);
        assert_eq!(
            called.updates.last(),
            Some(&Update::Citation(Citation {
                source_id: "read:/docs/notes.md".to_string(),
                path: "/docs/notes.md".to_string(),
                label: "notes.md".to_string(),
            }))
        );
        let done = t.translate(&event(
            "session.tool.success",
            json!({"id": "call_1", "content": [{"type": "text", "text": "1: # Notes"}]}),
        ));
        let mut expected = ToolPart::new("call_1", "Read notes.md", ToolState::Completed);
        expected.output = Some("1: # Notes".to_string());
        assert_eq!(done.updates, [Update::Tool(expected)]);
    }

    #[test]
    fn relative_reads_are_resolved_and_cited_once() {
        let mut t = translator();
        t.translate(&event("session.tool.input.started", json!({"id": "a", "name": "read"})));
        let first = t.translate(&event(
            "session.tool.called",
            json!({"id": "a", "input": {"path": "guide/intro.md"}}),
        ));
        assert_eq!(first.updates.len(), 2);
        t.translate(&event("session.tool.input.started", json!({"id": "b", "name": "read"})));
        let second = t.translate(&event(
            "session.tool.called",
            json!({"id": "b", "input": {"path": "guide/intro.md"}}),
        ));
        assert_eq!(second.updates.len(), 1);
    }

    #[test]
    fn turns_an_edit_permission_into_a_pending_change_card_then_applies_it() {
        let mut t = translator();
        let started = t.translate(&event(
            "session.tool.input.started",
            json!({"id": "call_2", "name": "edit"}),
        ));
        assert!(started.updates.is_empty());
        let asked = t.translate(&event(
            "permission.asked",
            json!({
                "id": "per_1", "action": "edit", "resources": ["notes.md"],
                "metadata": {"files": [{"file": "notes.md", "patch": PATCH, "status": "modified",
                    "additions": 1, "deletions": 1}]},
                "source": {"type": "tool", "id": "call_2"},
            }),
        ));
        assert_eq!(
            asked.updates,
            [Update::Change {
                tool_call_id: "call_2".to_string(),
                permission_id: Some("per_1".to_string()),
                status: ChangeStatus::Pending,
                files: Some(vec![FileChange {
                    path: "/docs/notes.md".to_string(),
                    display_path: "notes.md".to_string(),
                    patch: PATCH.to_string(),
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
                "id": "call_2", "content": [{"type": "text", "text": "Edited notes.md"}],
                "metadata": {"files": [{"file": "notes.md", "patch": PATCH, "additions": 1,
                    "deletions": 1}]},
            }),
        ));
        let Update::Change { tool_call_id, status, .. } = &applied.updates[0] else {
            panic!("expected a change")
        };
        assert_eq!(tool_call_id, "call_2");
        assert_eq!(*status, ChangeStatus::Applied);
    }

    #[test]
    fn marks_a_rejected_edit_as_rejected_rather_than_failed() {
        let mut t = translator();
        t.translate(&event("session.tool.input.started", json!({"id": "call_3", "name": "edit"})));
        t.translate(&event(
            "permission.asked",
            json!({"id": "per_2", "action": "edit", "source": {"id": "call_3"}}),
        ));
        assert_eq!(
            t.translate(&event(
                "permission.replied",
                json!({"requestID": "per_2", "reply": "reject"})
            ))
            .updates,
            [Update::change("call_3", ChangeStatus::Rejected)]
        );
        assert_eq!(
            t.translate(&event(
                "session.tool.failed",
                json!({"id": "call_3", "error": {"message": "Rejected"}})
            ))
            .updates,
            [Update::change("call_3", ChangeStatus::Rejected)]
        );
    }

    #[test]
    fn reports_failed_edits_with_their_error() {
        let mut t = translator();
        t.translate(&event("session.tool.input.started", json!({"id": "c", "name": "patch"})));
        assert_eq!(
            t.translate(&event(
                "session.tool.failed",
                json!({"id": "c", "error": {"message": "No match"}})
            ))
            .updates,
            [Update::Change {
                tool_call_id: "c".to_string(),
                permission_id: None,
                status: ChangeStatus::Failed,
                files: None,
                error: Some("No match".to_string()),
            }]
        );
    }

    #[test]
    fn declines_shell_permissions_automatically() {
        let result = translator().translate(&event(
            "permission.asked",
            json!({"id": "per_3", "action": "shell", "resources": ["ls"],
                "source": {"id": "call_4"}}),
        ));
        assert!(result.updates.is_empty());
        let (id, reason) = result.auto_reject.expect("declined");
        assert_eq!(id, "per_3");
        assert!(reason.contains("shell"));
        assert!(reason.contains("(ls)"));
    }

    #[test]
    fn a_declined_shell_permission_never_shows_a_change_card() {
        let mut t = translator();
        t.translate(&event(
            "permission.asked",
            json!({"id": "per_4", "action": "shell", "source": {"id": "call_5"}}),
        ));
        let replied = t.translate(&event(
            "permission.replied",
            json!({"requestID": "per_4", "reply": "reject"}),
        ));
        assert!(replied.updates.is_empty());
    }

    #[test]
    fn reports_how_the_turn_ended() {
        let mut t = translator();
        let finish = |o| Some(TurnFinish::new(o));
        assert_eq!(
            t.translate(&event("session.execution.succeeded", json!({}))).finish,
            finish(Outcome::Succeeded)
        );
        assert_eq!(
            t.translate(&event(
                "session.execution.failed",
                json!({"error": {"type": "provider.auth", "message": "Sign in again"}})
            ))
            .finish,
            Some(TurnFinish { outcome: Outcome::Failed, error: Some("Sign in again".to_string()) })
        );
        assert_eq!(
            t.translate(&event("session.execution.interrupted", json!({}))).finish,
            finish(Outcome::Interrupted)
        );
        let started = t.translate(&event("session.execution.started", json!({})));
        assert_eq!(started.finish, None);
        assert!(started.started);
    }

    #[test]
    fn tool_titles_use_plain_words() {
        assert_eq!(
            tool_title("grep", Some(&json!({"pattern": "roadmap"}))),
            "Searched for “roadmap”"
        );
        assert_eq!(
            tool_title("glob", Some(&json!({"pattern": "**/*.md"}))),
            "Looked through the folder"
        );
        assert_eq!(
            tool_title("websearch", Some(&json!({"query": "swift"}))),
            "Searched the web for “swift”"
        );
        assert_eq!(tool_title("mystery", None), "Mystery");
    }

    #[test]
    fn file_changes_resolve_relative_paths_and_skip_malformed_entries() {
        let metadata = json!({"files": [{"file": "a.md", "status": "added"}, {"nope": true}, "x"]});
        assert_eq!(
            file_changes(Some(&metadata), DIR),
            [FileChange {
                path: "/docs/a.md".to_string(),
                display_path: "a.md".to_string(),
                patch: String::new(),
                additions: 0,
                deletions: 0,
                status: FileStatus::Added,
            }]
        );
        assert!(file_changes(None, DIR).is_empty());
    }

    #[test]
    fn sse_parser_joins_data_lines() {
        let mut parser = SseParser::default();
        let stream = "event: message\ndata: {\"type\":\"a\",\"data\":{}}\n\ndata: one\r\ndata: two\n\n: heartbeat\n\n";
        let received: Vec<String> =
            stream.split('\n').filter_map(|line| parser.feed_line(line)).collect();
        assert_eq!(received, ["{\"type\":\"a\",\"data\":{}}", "one\ntwo"]);
    }

    #[test]
    fn parses_typed_events_and_drops_malformed_payloads() {
        let parsed = Event::parse(
            "{\"id\":\"evt_1\",\"type\":\"session.idle\",\"data\":{\"sessionID\":\"s\"}}",
        )
        .unwrap();
        assert_eq!(parsed.id.as_deref(), Some("evt_1"));
        assert_eq!(parsed.kind, "session.idle");
        assert_eq!(parsed.data.get("sessionID"), Some(&json!("s")));
        assert!(Event::parse("{\"data\":{}}").is_none());
        assert!(Event::parse("not json").is_none());
    }
}
