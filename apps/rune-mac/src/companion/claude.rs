//! Claude, through the user's own Claude Code: a Rust port of the part of Anthropic's Agent SDK
//! that Mdow needs. Its reference is the open-source Python SDK (`claude-agent-sdk-python`,
//! `_internal/transport/subprocess_cli.py` and `_internal/query.py`), which drives the CLI the
//! same way the TypeScript SDK does.
//!
//! Each chat is one `claude` process in stream-json mode: one JSON message per line on its stdin
//! and stdout. Mdow writes user messages and control requests (initialize, interrupt, set_model,
//! apply_flag_settings). Claude Code writes its messages, partial ones included, and asks
//! permission for each tool call outside its read-only defaults (`can_use_tool`, through
//! `--permission-prompt-tool stdio`). No Node, no adapter, and nothing touches the user's
//! credentials: the CLI signs in as it always does.
//!
//! The session is kept small and hermetic, which also keeps it fast. It gets only the file tools
//! (`--tools`), no MCP servers, and none of the user's settings files, so their permission rules
//! can't approve an edit before Mdow sees it.
//!
//! The module answers the requests an ACP agent does (`session/new`, `session/prompt`, …) and
//! reports in ACP's shapes, so the rest of the companion treats Claude like the other agents:
//! [`super::acp::Client`] hands Claude's requests here.

use super::acp::{self, Event};
use super::agents::Agent;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

/// The tools a session has: reading, searching, and edits (each one reviewed).
const TOOLS: &str = "Read,Glob,Grep,Edit,Write";

/// What Mdow tells Claude it declined, by whom.
const DECLINED_BY_USER: &str = "The user declined this change.";
const DECLINED_BY_MDOW: &str =
    "Mdow doesn’t allow this here: only reading this folder and reviewed edits.";

#[derive(Default)]
struct State {
    stdin: Option<ChildStdin>,
    child: Option<Child>,
    next: u64,
    /// Control requests waiting on their reply, by request id.
    waiting: HashMap<String, Sender<Result<Value, String>>>,
    /// The prompt waiting on its result.
    turn: Option<Sender<Result<Value, String>>>,
    cancelled: bool,
    /// Stopped turns whose result hasn't come yet. Claude Code answers messages in order, so
    /// the next results, and what streams before them, belong to those turns, not the new one.
    stale: u32,
    /// Permission requests waiting on the user, by request id: the tool's input and whether it
    /// was an edit.
    asked: HashMap<String, (Value, bool)>,
    /// The models from `initialize`.
    models: Vec<Value>,
    model: String,
    effort: String,
    /// The message streaming now, and those whose text arrived in parts.
    message: String,
    streamed: HashSet<String>,
    /// The end of stderr, for errors.
    errors: String,
}

struct Session {
    id: String,
    directory: String,
    state: Mutex<State>,
}

static SESSIONS: LazyLock<Mutex<HashMap<String, Arc<Session>>>> = LazyLock::new(Default::default);

fn sessions() -> MutexGuard<'static, HashMap<String, Arc<Session>>> {
    SESSIONS.lock().unwrap_or_else(|e| e.into_inner())
}

fn session(params: &Value) -> Result<Arc<Session>, String> {
    let id = params.get("sessionId").and_then(Value::as_str).unwrap_or("");
    sessions().get(id).cloned().ok_or_else(|| "This chat with Claude has ended.".to_string())
}

/// An ACP request, answered by Claude Code.
pub fn request(method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "initialize" | "session/set_mode" => Ok(json!({})),
        "session/new" => start(
            params.get("cwd").and_then(Value::as_str).unwrap_or("/"),
            params.pointer("/_meta/systemPrompt/append").and_then(Value::as_str).unwrap_or(""),
        ),
        "session/prompt" => {
            let text: Vec<&str> = params
                .get("prompt")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|b| b.get("text").and_then(Value::as_str)).collect())
                .unwrap_or_default();
            session(params)?.prompt(&text.join("\n\n"))
        }
        "session/set_model" => {
            let model = params.get("modelId").and_then(Value::as_str).unwrap_or("");
            session(params)?.set_model(model)
        }
        "session/set_config_option" => {
            let s = session(params)?;
            let value = params.get("value").and_then(Value::as_str).unwrap_or("");
            match params.get("configId").and_then(Value::as_str) {
                Some("model") => s.set_model(value),
                Some("effort") => s.set_effort(value),
                _ => Err("Claude has no such setting.".into()),
            }
        }
        _ => Err(format!("Claude doesn’t answer {method}.")),
    }
}

/// An ACP notification: only `session/cancel`, which interrupts the turn.
pub fn notify(method: &str, params: &Value) {
    if method == "session/cancel"
        && let Ok(s) = session(params)
    {
        s.interrupt();
    }
}

/// Answers a permission request sent as [`Event::Permission`].
pub fn respond(id: &Value, allow: bool) {
    let sid = id.get("session").and_then(Value::as_str).unwrap_or("");
    let Some(rid) = id.get("request").and_then(Value::as_str) else { return };
    if let Some(s) = sessions().get(sid).cloned() {
        s.answer(rid, allow);
    }
}

/// Whether a chat's process is still there.
pub fn alive(session: &str) -> bool {
    sessions().get(session).is_some_and(|s| s.lock().stdin.is_some())
}

/// Ends a chat: its process goes.
pub fn close(session: &str) {
    let s = sessions().remove(session);
    if let Some(s) = s {
        s.stop();
    }
}

pub fn stop_all() {
    let all: Vec<_> = sessions().drain().map(|(_, s)| s).collect();
    for s in all {
        s.stop();
    }
}

/// A random version 4 UUID, for `--session-id`.
fn uuid() -> String {
    let mut b = [0u8; 16];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        b = (t ^ ((std::process::id() as u128) << 64)).to_le_bytes();
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

/// Starts a chat in `directory`: the process, then the SDK's `initialize`.
fn start(directory: &str, instructions: &str) -> Result<Value, String> {
    let program = Agent::Claude.command()?;
    let id = uuid();
    let mut command = Command::new(&program[0]);
    command
        .args(&program[1..])
        .args(["--output-format", "stream-json", "--input-format", "stream-json", "--verbose"])
        .args(["--permission-prompt-tool", "stdio", "--permission-mode", "default"])
        .args(["--tools", TOOLS, "--setting-sources=", "--strict-mcp-config"])
        .args(["--mcp-config", r#"{"mcpServers":{}}"#])
        .args(["--include-partial-messages", "--thinking", "adaptive"])
        .args(["--thinking-display", "summarized"])
        .arg(format!("--session-id={id}"));
    if !instructions.is_empty() {
        command.args(["--append-system-prompt", instructions]);
    }
    let mut child = command
        .current_dir(directory)
        .env("PATH", super::process::environment_path())
        // Set when Mdow itself runs inside Claude Code; the child would think it's nested.
        .env_remove("CLAUDECODE")
        .env("CLAUDE_AGENT_SDK_CLIENT_APP", "Mdow")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn’t start Claude Code at {}: {e}", program[0]))?;
    let stdout = child.stdout.take().ok_or("No output from Claude Code.")?;
    let stderr = child.stderr.take().ok_or("No output from Claude Code.")?;
    let s = Arc::new(Session {
        id: id.clone(),
        directory: directory.to_string(),
        state: Mutex::new(State {
            stdin: child.stdin.take(),
            child: Some(child),
            model: "default".into(),
            ..State::default()
        }),
    });
    let me = s.clone();
    acp::read_errors(stderr, move |part| acp::keep_errors(&mut me.lock().errors, part));
    let me = s.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).split(b'\n') {
            let Ok(line) = line else { break };
            if line.is_empty() {
                continue;
            }
            acp::log("<< ", &format!("{}\n", String::from_utf8_lossy(&line)));
            if let Ok(message) = serde_json::from_slice::<Value>(&line) {
                me.handle(message);
            }
        }
        me.exited();
    });
    sessions().insert(id.clone(), s.clone());
    let reply = s.control(json!({"subtype": "initialize", "hooks": null}));
    let reply = match reply {
        Ok(r) => r,
        Err(e) => {
            close(&id);
            return Err(e);
        }
    };
    let mut st = s.lock();
    st.models = reply.get("models").and_then(Value::as_array).cloned().unwrap_or_default();
    Ok(json!({"sessionId": id, "configOptions": options(&st)}))
}

/// The session's settings in ACP's shape: its model, and the model's effort where it has one.
fn options(st: &State) -> Vec<Value> {
    let name = |m: &Value| {
        m.get("displayName").or(m.get("value")).and_then(Value::as_str).unwrap_or("").to_string()
    };
    let models: Vec<Value> = st
        .models
        .iter()
        .filter_map(|m| {
            let value = m.get("value")?.as_str()?;
            Some(json!({"value": value, "name": name(m)}))
        })
        .collect();
    let mut out = vec![
        json!({"id": "model", "category": "model", "currentValue": st.model, "options": models}),
    ];
    let levels = efforts(st);
    if !levels.is_empty() {
        let choices: Vec<Value> =
            levels.iter().map(|l| json!({"value": l, "name": acp::effort_name(l)})).collect();
        out.push(json!({"id": "effort", "category": "thought_level", "currentValue": st.effort, "options": choices}));
    }
    out
}

/// The effort levels the current model takes.
fn efforts(st: &State) -> Vec<String> {
    st.models
        .iter()
        .find(|m| m.get("value").and_then(Value::as_str) == Some(st.model.as_str()))
        .and_then(|m| m.get("supportedEffortLevels"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

impl Session {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn write(&self, message: &Value) {
        let mut line = message.to_string();
        line.push('\n');
        acp::log(">> ", &line);
        let mut st = self.lock();
        if let Some(stdin) = st.stdin.as_mut()
            && stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()).is_err()
        {
            st.stdin = None;
        }
    }

    fn next_request(&self) -> String {
        let mut st = self.lock();
        st.next += 1;
        format!("mdow_{}", st.next)
    }

    /// Sends a control request and waits for its reply.
    fn control(&self, request: Value) -> Result<Value, String> {
        let (tx, rx) = channel();
        let id = self.next_request();
        {
            let mut st = self.lock();
            if st.stdin.is_none() {
                return Err("Claude Code isn’t running.".into());
            }
            st.waiting.insert(id.clone(), tx);
        }
        self.write(&json!({"type": "control_request", "request_id": id, "request": request}));
        acp::wait(&rx, Agent::Claude, false)
    }

    /// Sends the user's message and waits for the turn's result.
    fn prompt(&self, text: &str) -> Result<Value, String> {
        let (tx, rx) = channel();
        if self.lock().stdin.is_none() {
            return Err("Claude Code stopped.".into());
        }
        self.begin(tx);
        self.write(&json!({
            "type": "user",
            "message": {"role": "user", "content": text},
            "parent_tool_use_id": null,
            "session_id": self.id,
        }));
        acp::wait(&rx, Agent::Claude, true)
    }

    /// Makes `tx` the turn's waiter. A stopped turn still running: its result isn't this one's.
    fn begin(&self, tx: Sender<Result<Value, String>>) {
        let mut st = self.lock();
        if st.turn.replace(tx).is_some() {
            st.stale += 1;
        }
        st.cancelled = false;
    }

    fn interrupt(&self) {
        {
            let mut st = self.lock();
            if st.turn.is_none() {
                return;
            }
            st.cancelled = true;
        }
        // Its reply isn't waited on: the turn's result ends the prompt.
        let id = self.next_request();
        self.write(&json!({"type": "control_request", "request_id": id, "request": {"subtype": "interrupt"}}));
    }

    fn set_model(&self, model: &str) -> Result<Value, String> {
        self.control(json!({"subtype": "set_model", "model": model}))?;
        let mut st = self.lock();
        st.model = model.to_string();
        // The effort carries over where the new model takes it.
        if !efforts(&st).contains(&st.effort) {
            st.effort.clear();
        }
        Ok(json!({"configOptions": options(&st)}))
    }

    fn set_effort(&self, effort: &str) -> Result<Value, String> {
        self.control(
            json!({"subtype": "apply_flag_settings", "settings": {"effortLevel": effort}}),
        )?;
        let mut st = self.lock();
        st.effort = effort.to_string();
        Ok(json!({"configOptions": options(&st)}))
    }

    fn answer(&self, request: &str, allow: bool) {
        let Some((input, edit)) = self.lock().asked.remove(request) else { return };
        let response = if allow {
            json!({"behavior": "allow", "updatedInput": input})
        } else {
            let message = if edit { DECLINED_BY_USER } else { DECLINED_BY_MDOW };
            json!({"behavior": "deny", "message": message})
        };
        self.write(&json!({"type": "control_response", "response": {"subtype": "success", "request_id": request, "response": response}}));
    }

    fn stop(&self) {
        let mut st = self.lock();
        st.stdin = None;
        if let Some(mut child) = st.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn exited(&self) {
        let (waiting, turn, errors) = {
            let mut st = self.lock();
            st.stdin = None;
            if let Some(mut child) = st.child.take() {
                let _ = child.wait();
            }
            st.stale = 0;
            (std::mem::take(&mut st.waiting), st.turn.take(), std::mem::take(&mut st.errors))
        };
        let failure = acp::stopped(Agent::Claude, &errors);
        for (_, tx) in waiting {
            let _ = tx.send(Err(failure.clone()));
        }
        if let Some(tx) = turn {
            let _ = tx.send(Err(failure));
        }
    }

    fn update(&self, update: Value) {
        acp::deliver(&self.id, Event::Update(update));
    }

    fn handle(&self, message: Value) {
        let kind = message.get("type").and_then(Value::as_str).unwrap_or("");
        // Subagents' messages carry their parent call; Mdow gives Claude no subagents. A
        // stopped turn's last words are dropped.
        let top =
            message.get("parent_tool_use_id").is_none_or(Value::is_null) && self.lock().stale == 0;
        match kind {
            "control_response" => {
                let response = message.get("response").cloned().unwrap_or_default();
                let id = response.get("request_id").and_then(Value::as_str).unwrap_or("");
                let Some(tx) = self.lock().waiting.remove(id) else { return };
                let result = match response.get("subtype").and_then(Value::as_str) {
                    Some("error") => Err(acp::explain(
                        Agent::Claude,
                        response
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("Claude Code refused."),
                    )),
                    _ => Ok(response.get("response").cloned().unwrap_or(json!({}))),
                };
                let _ = tx.send(result);
            }
            "control_request" => self.permission(&message),
            "stream_event" if top => self.stream(message.get("event").unwrap_or(&Value::Null)),
            "assistant" if top => self.assistant(message.get("message").unwrap_or(&Value::Null)),
            "user" if top => self.tool_results(message.get("message").unwrap_or(&Value::Null)),
            "result" => self.result(&message),
            _ => {}
        }
    }

    /// Text and thinking as they arrive.
    fn stream(&self, event: &Value) {
        match event.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                let id = event.pointer("/message/id").and_then(Value::as_str).unwrap_or("");
                self.lock().message = id.to_string();
            }
            Some("content_block_delta") => {
                let delta = event.get("delta").unwrap_or(&Value::Null);
                let (kind, text) = match delta.get("type").and_then(Value::as_str) {
                    Some("text_delta") => ("agent_message_chunk", delta.get("text")),
                    Some("thinking_delta") => ("agent_thought_chunk", delta.get("thinking")),
                    _ => return,
                };
                let Some(text) = text.and_then(Value::as_str).filter(|t| !t.is_empty()) else {
                    return;
                };
                {
                    let mut st = self.lock();
                    let id = st.message.clone();
                    st.streamed.insert(id);
                }
                self.update(
                    json!({"sessionUpdate": kind, "content": {"type": "text", "text": text}}),
                );
            }
            _ => {}
        }
    }

    /// A finished message: its tool calls, and its text where none of it streamed.
    fn assistant(&self, message: &Value) {
        let id = message.get("id").and_then(Value::as_str).unwrap_or("");
        let streamed = self.lock().streamed.contains(id);
        for block in message.get("content").and_then(Value::as_array).into_iter().flatten() {
            let s = |k: &str| block.get(k).and_then(Value::as_str).unwrap_or("");
            match s("type") {
                "tool_use" => {
                    let input = block.get("input").cloned().unwrap_or(json!({}));
                    let (kind, title) = describe(s("name"), &input);
                    self.update(json!({
                        "sessionUpdate": "tool_call",
                        "toolCallId": s("id"),
                        "title": title,
                        "kind": kind,
                        "status": "pending",
                        "rawInput": input,
                    }));
                }
                "text" if !streamed && !s("text").is_empty() => self.update(json!({
                    "sessionUpdate": "agent_message_chunk",
                    "content": {"type": "text", "text": s("text")},
                })),
                "thinking" if !streamed && !s("thinking").is_empty() => self.update(json!({
                    "sessionUpdate": "agent_thought_chunk",
                    "content": {"type": "text", "text": s("thinking")},
                })),
                _ => {}
            }
        }
    }

    /// Tool results come back as a user message.
    fn tool_results(&self, message: &Value) {
        let Some(blocks) = message.get("content").and_then(Value::as_array) else { return };
        for block in
            blocks.iter().filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_result"))
        {
            let text = match block.get("content") {
                Some(Value::String(t)) => t.clone(),
                Some(Value::Array(parts)) => parts
                    .iter()
                    .filter_map(|p| p.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            };
            let failed = block.get("is_error").and_then(Value::as_bool).unwrap_or(false);
            self.update(json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": block.get("tool_use_id").and_then(Value::as_str).unwrap_or(""),
                "status": if failed { "failed" } else { "completed" },
                "content": [{"type": "content", "content": {"type": "text", "text": text}}],
            }));
        }
    }

    fn result(&self, message: &Value) {
        let (turn, cancelled) = {
            let mut st = self.lock();
            st.streamed.clear();
            if st.stale > 0 {
                st.stale -= 1;
                return;
            }
            (st.turn.take(), std::mem::take(&mut st.cancelled))
        };
        let Some(tx) = turn else { return };
        let s = |k: &str| message.get(k).and_then(Value::as_str).unwrap_or("");
        let reply = if cancelled {
            Ok("cancelled")
        } else if s("subtype") == "error_max_turns" {
            Ok("max_turn_requests")
        } else if message.get("is_error").and_then(Value::as_bool).unwrap_or(false)
            || s("subtype").starts_with("error")
        {
            let errors: Vec<&str> = message
                .get("errors")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let text = if !s("result").is_empty() {
                s("result").to_string()
            } else if !errors.is_empty() {
                errors.join(" ")
            } else {
                "Claude Code stopped with an error.".to_string()
            };
            Err(acp::explain(Agent::Claude, &text))
        } else {
            Ok(match s("stop_reason") {
                "max_tokens" => "max_tokens",
                "refusal" => "refusal",
                _ => "end_turn",
            })
        };
        let _ = tx.send(reply.map(|r| json!({"stopReason": r})));
    }

    /// `can_use_tool`: reads in the folder run, edits go to the window, the rest is declined,
    /// by the same rules as the ACP agents (`acp::decide`).
    fn permission(&self, message: &Value) {
        let request_id =
            message.get("request_id").and_then(Value::as_str).unwrap_or("").to_string();
        let request = message.get("request").unwrap_or(&Value::Null);
        let s = |k: &str| request.get(k).and_then(Value::as_str).unwrap_or("");
        if s("subtype") != "can_use_tool" {
            let error = format!("Mdow doesn’t handle {}.", s("subtype"));
            self.write(&json!({"type": "control_response", "response": {"subtype": "error", "request_id": request_id, "error": error}}));
            return;
        }
        let tool = s("tool_name");
        let input = request.get("input").cloned().unwrap_or(json!({}));
        let (mut kind, title) = describe(tool, &input);
        // Claude Code asks about a read only when it's outside the folder; an edit there is
        // declined like any other change outside it.
        if !self.inside(&input) {
            kind = "other";
        }
        let mut call = json!({
            "toolCallId": s("tool_use_id"),
            "title": title,
            "kind": kind,
            "rawInput": input,
        });
        if kind == "edit"
            && let Some(diff) = diff(tool, &input)
        {
            call["content"] = json!([diff]);
        }
        self.lock().asked.insert(request_id.clone(), (input, kind == "edit"));
        let options = vec![
            json!({"optionId": "allow", "kind": "allow_once", "name": "Allow"}),
            json!({"optionId": "reject", "kind": "reject_once", "name": "Reject"}),
        ];
        acp::decide(
            Agent::Claude,
            &self.id,
            json!({"session": self.id, "request": request_id}),
            call,
            options,
        );
    }

    /// Whether a tool's file or folder is in the session's folder.
    fn inside(&self, input: &Value) -> bool {
        let path = ["file_path", "path"].iter().find_map(|k| input.get(*k).and_then(Value::as_str));
        path.is_none_or(|p| acp::within(p, &self.directory))
    }
}

/// A tool's ACP kind and title.
fn describe(tool: &str, input: &Value) -> (&'static str, String) {
    let s = |k: &str| input.get(k).and_then(Value::as_str).unwrap_or("");
    match tool {
        "Read" => ("read", format!("Read {}", s("file_path"))),
        "Glob" => ("search", format!("Find {}", s("pattern"))),
        "Grep" => ("search", format!("Search for {}", s("pattern"))),
        "Edit" | "MultiEdit" => ("edit", format!("Edit {}", s("file_path"))),
        "Write" => ("edit", format!("Write {}", s("file_path"))),
        "TodoWrite" => ("think", "Plan".into()),
        other => ("other", other.to_string()),
    }
}

/// An edit as ACP's diff content: the file now and after the edit.
fn diff(tool: &str, input: &Value) -> Option<Value> {
    let path = input.get("file_path")?.as_str()?;
    let old = std::fs::read_to_string(path).ok();
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let replace = |text: &str, e: &Value| {
        let (from, to) = (s(e, "old_string"), s(e, "new_string"));
        if e.get("replace_all").and_then(Value::as_bool).unwrap_or(false) {
            text.replace(&from, &to)
        } else {
            text.replacen(&from, &to, 1)
        }
    };
    let new = match tool {
        "Write" => s(input, "content"),
        "Edit" => replace(old.as_deref()?, input),
        "MultiEdit" => {
            input.get("edits")?.as_array()?.iter().fold(old.clone()?, |text, e| replace(&text, e))
        }
        _ => return None,
    };
    Some(json!({"type": "diff", "path": path, "oldText": old, "newText": new}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_become_diffs_and_tools_get_kinds() {
        let dir = std::env::temp_dir().join(format!("mdow-claude-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("note.md");
        std::fs::write(&file, "Teh cat. Teh mat.\n").unwrap();
        let path = file.to_string_lossy().to_string();
        let edit = json!({"file_path": path, "old_string": "Teh", "new_string": "The"});
        let d = diff("Edit", &edit).unwrap();
        assert_eq!(d["newText"], "The cat. Teh mat.\n");
        assert_eq!(d["oldText"], "Teh cat. Teh mat.\n");
        let all = json!({"file_path": path, "old_string": "Teh", "new_string": "The", "replace_all": true});
        assert_eq!(diff("Edit", &all).unwrap()["newText"], "The cat. The mat.\n");
        let new = json!({"file_path": dir.join("new.md").to_string_lossy(), "content": "Hi\n"});
        assert!(diff("Write", &new).unwrap()["oldText"].is_null());
        assert_eq!(
            describe("Grep", &json!({"pattern": "cat"})),
            ("search", "Search for cat".into())
        );
        assert_eq!(describe("Bash", &json!({})).0, "other");
        let session = Session {
            id: "s".into(),
            directory: dir.to_string_lossy().into(),
            state: Mutex::default(),
        };
        assert!(session.inside(&json!({"file_path": path})));
        assert!(session.inside(&json!({"pattern": "*.md"})));
        assert!(!session.inside(&json!({"file_path": "/etc/hosts"})));
        assert!(!session.inside(&json!({"file_path": "../../.zshrc"})));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_stopped_turns_result_doesnt_end_the_next() {
        let s = Session { id: "t".into(), directory: "/".into(), state: Mutex::default() };
        let (old, old_rx) = channel();
        s.begin(old);
        s.interrupt();
        // Sent again before the stopped turn's result came.
        let (new, new_rx) = channel();
        s.begin(new);
        assert!(old_rx.recv().is_err(), "the stopped turn's waiter is let go");
        let stream = json!({"type": "stream_event", "event": {"type": "content_block_delta", "delta": {"type": "text_delta", "text": "late"}}});
        s.handle(stream);
        assert!(s.lock().streamed.is_empty(), "the stopped turn's words are dropped");
        s.handle(json!({"type": "result", "subtype": "error_during_execution", "is_error": true}));
        assert!(new_rx.try_recv().is_err());
        s.handle(json!({"type": "result", "subtype": "success", "stop_reason": "end_turn"}));
        assert_eq!(new_rx.try_recv().unwrap().unwrap()["stopReason"], "end_turn");
    }

    #[test]
    fn options_follow_the_model() {
        let mut st = State {
            models: vec![
                json!({"value": "default", "displayName": "Default (recommended)", "supportedEffortLevels": ["low", "xhigh"]}),
                json!({"value": "claude-haiku-4-5", "displayName": "Haiku 4.5"}),
            ],
            model: "default".into(),
            ..State::default()
        };
        let o = options(&st);
        assert_eq!(acp::efforts(&o).unwrap().1.values[1].name, "Extra high");
        st.model = "claude-haiku-4-5".into();
        assert_eq!(options(&st).len(), 1);
        assert_eq!(acp::models_in(&json!({"configOptions": options(&st)})).len(), 2);
        assert_eq!(uuid().len(), 36);
    }
}
