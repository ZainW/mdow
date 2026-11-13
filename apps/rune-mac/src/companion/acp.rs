//! Agents spoken to over the Agent Client Protocol (Cursor's `agent acp`), ported from Mrow's
//! `acp.rs`: JSON-RPC, one message per line on the agent's stdin and stdout. One process per
//! agent serves every window. Claude and Codex answer the same requests in-process, through
//! clients for their own interfaces (`claude.rs` drives Claude Code, `codex.rs` Codex's app
//! server), so the companion treats every agent the same way.
//!
//! Mdow offers no file or terminal access of its own. Permission requests are answered by tool
//! kind (`agents::allows`): reads and searches run, edits go to the window as a diff to accept
//! or decline, and the rest are declined.
//!
//! Requests block their (worker) thread until the reply. Session updates and edit permissions go
//! to the window that owns the session, through its actor's channel and waker.

use super::agents::{self, Agent};
use super::events::basename;
use super::logic::display_path;
use super::types::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, LazyLock, Mutex};

/// What an agent tells the window that owns a session.
#[derive(Debug)]
pub enum Event {
    Update(Value),
    /// The agent asks to make an edit: answer with [`Client::respond`].
    Permission {
        id: Value,
        call: Value,
        options: Vec<Value>,
    },
}

type Route = (Sender<super::service::Cmd>, rune::Waker);

/// Each session's window.
static ROUTES: LazyLock<Mutex<HashMap<String, Route>>> = LazyLock::new(Default::default);

pub fn route(session: &str, tx: Sender<super::service::Cmd>, waker: rune::Waker) {
    ROUTES.lock().unwrap_or_else(|e| e.into_inner()).insert(session.to_string(), (tx, waker));
}

pub fn unroute(session: &str) {
    ROUTES.lock().unwrap_or_else(|e| e.into_inner()).remove(session);
}

/// Sends `event` to the session's window; false when no window owns it.
pub(super) fn deliver(session: &str, event: Event) -> bool {
    let route = ROUTES.lock().unwrap_or_else(|e| e.into_inner()).get(session).cloned();
    let Some((tx, waker)) = route else { return false };
    let sent = tx.send(super::service::Cmd::Acp(event)).is_ok();
    if sent {
        waker();
    }
    sent
}

#[derive(Default)]
struct State {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    next_id: i64,
    waiting: HashMap<i64, Sender<Result<Value, String>>>,
    errors: String,
    /// Bumped per launch, so a stale reader's exit doesn't end a newer process's requests.
    generation: u64,
}

pub struct Client {
    agent: Agent,
    state: Mutex<State>,
    /// Held while starting, so concurrent first requests wait for one launch.
    starting: Mutex<bool>,
}

static CLIENTS: LazyLock<Vec<Arc<Client>>> = LazyLock::new(|| {
    Agent::ALL
        .into_iter()
        .filter(|a| a.is_acp())
        .map(|agent| {
            Arc::new(Client { agent, state: Mutex::default(), starting: Mutex::new(false) })
        })
        .collect()
});

/// The one client for this agent.
pub fn client(agent: Agent) -> Arc<Client> {
    CLIENTS.iter().find(|c| c.agent == agent).cloned().expect("every ACP agent has a client")
}

/// Stops every agent that's running (the app is quitting, or the last window went).
pub fn stop_all() {
    for c in CLIENTS.iter() {
        c.stop();
    }
    super::claude::stop_all();
    super::codex::stop_all();
}

/// Ends a session: Claude's process goes with it, Codex lets its thread go, and an ACP agent's
/// session is cancelled.
pub fn close(agent: Agent, session: &str) {
    match agent {
        Agent::Claude => super::claude::close(session),
        Agent::Codex => super::codex::close(session),
        _ => client(agent).notify("session/cancel", json!({"sessionId": session})),
    }
}

/// Whether the process behind a session is still running.
pub fn alive(agent: Agent, session: &str) -> bool {
    match agent {
        Agent::Claude => super::claude::alive(session),
        Agent::Codex => super::codex::alive(session),
        _ => client(agent).is_running(),
    }
}

impl Client {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Sends a request and waits for its reply. Starts the agent if it isn't running.
    pub fn request(self: &Arc<Self>, method: &str, params: Value) -> Result<Value, String> {
        match self.agent {
            Agent::Claude => return super::claude::request(method, &params),
            Agent::Codex => return super::codex::request(method, &params),
            _ => {}
        }
        self.start()?;
        self.send(method, params)
    }

    pub fn notify(&self, method: &str, params: Value) {
        match self.agent {
            Agent::Claude => return super::claude::notify(method, &params),
            Agent::Codex => return super::codex::notify(method, &params),
            _ => {}
        }
        self.write(&json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    pub fn is_running(&self) -> bool {
        self.lock().stdin.is_some()
    }

    pub fn stop(&self) {
        let mut s = self.lock();
        s.stdin = None;
        if let Some(mut child) = s.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Answers an edit permission: the agent's allow-once option, or its reject option.
    pub fn respond(&self, id: Value, options: &[Value], allow: bool) {
        match self.agent {
            Agent::Claude => return super::claude::respond(&id, allow),
            Agent::Codex => return super::codex::respond(&id, allow),
            _ => {}
        }
        let pick = |prefix: &str| {
            options
                .iter()
                .find(|o| {
                    o.get("kind").and_then(Value::as_str).is_some_and(|k| k.starts_with(prefix))
                })
                .and_then(|o| o.get("optionId").cloned())
        };
        let choice =
            if allow { pick("allow_once").or_else(|| pick("allow")) } else { pick("reject") };
        let outcome = match choice {
            Some(option) => json!({"outcome": "selected", "optionId": option}),
            None => json!({"outcome": "cancelled"}),
        };
        self.write(&json!({"jsonrpc": "2.0", "id": id, "result": {"outcome": outcome}}));
    }

    fn send(&self, method: &str, params: Value) -> Result<Value, String> {
        let (tx, rx) = channel();
        let id = {
            let mut s = self.lock();
            if s.stdin.is_none() {
                return Err(format!("{} isn’t running.", self.agent.process()));
            }
            s.next_id += 1;
            let id = s.next_id;
            s.waiting.insert(id, tx);
            id
        };
        self.write(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        rx.recv().unwrap_or_else(|_| Err(format!("{} stopped.", self.agent.process())))
    }

    fn write(&self, message: &Value) {
        let mut line = message.to_string();
        line.push('\n');
        log(">> ", &line);
        let mut s = self.lock();
        if let Some(stdin) = s.stdin.as_mut()
            && stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()).is_err()
        {
            s.stdin = None;
        }
    }

    fn start(self: &Arc<Self>) -> Result<(), String> {
        let mut started = self.starting.lock().unwrap_or_else(|e| e.into_inner());
        if *started && self.is_running() {
            return Ok(());
        }
        *started = false;
        self.launch()?;
        let reply = self.send(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {
                    "fs": {"readTextFile": false, "writeTextFile": false},
                    "terminal": false,
                    // Cursor: plain model ids, with effort as a separate setting.
                    "_meta": {"parameterizedModelPicker": true},
                },
                "clientInfo": {"name": "Mdow", "version": env!("CARGO_PKG_VERSION")},
            }),
        );
        if let Err(e) = reply {
            self.stop();
            return Err(e);
        }
        *started = true;
        Ok(())
    }

    fn launch(self: &Arc<Self>) -> Result<(), String> {
        let command = self.agent.command()?;
        let name = self.agent.process();
        let mut child = Command::new(&command[0])
            .args(&command[1..])
            .current_dir(super::logic::home())
            .env("PATH", super::process::environment_path())
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Couldn’t start {name} at {}: {e}", command[0]))?;
        let stdout = child.stdout.take().ok_or_else(|| format!("No output from {name}."))?;
        let mut stderr = child.stderr.take().ok_or_else(|| format!("No output from {name}."))?;
        let generation = {
            let mut s = self.lock();
            s.generation += 1;
            s.stdin = child.stdin.take();
            s.child = Some(child);
            s.errors.clear();
            s.generation
        };
        let me = self.clone();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                let mut s = me.lock();
                s.errors.push_str(&String::from_utf8_lossy(&buffer[..n]));
                // Kept for error messages: the last couple of kilobytes is enough.
                if s.errors.len() > 2000 {
                    let mut cut = s.errors.len() - 2000;
                    while !s.errors.is_char_boundary(cut) {
                        cut += 1;
                    }
                    s.errors.drain(..cut);
                }
            }
        });
        let me = self.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).split(b'\n') {
                let Ok(line) = line else { break };
                if line.is_empty() {
                    continue;
                }
                let text = String::from_utf8_lossy(&line);
                log("<< ", &format!("{text}\n"));
                if let Ok(message) = serde_json::from_str::<Value>(&text) {
                    me.handle(message);
                }
            }
            me.exited(generation);
        });
        Ok(())
    }

    fn exited(&self, generation: u64) {
        let (waiting, errors) = {
            let mut s = self.lock();
            if s.generation != generation {
                return;
            }
            s.stdin = None;
            if let Some(mut child) = s.child.take() {
                let _ = child.wait();
            }
            (std::mem::take(&mut s.waiting), std::mem::take(&mut s.errors))
        };
        let detail = errors.trim();
        let tail: String =
            detail.chars().rev().take(300).collect::<Vec<_>>().into_iter().rev().collect();
        let name = self.agent.process();
        let failure = if detail.is_empty() {
            format!("{name} stopped.")
        } else {
            format!("{name} stopped. {tail}")
        };
        for (_, tx) in waiting {
            let _ = tx.send(Err(failure.clone()));
        }
    }

    fn handle(&self, message: Value) {
        let method = message.get("method").and_then(Value::as_str).map(str::to_string);
        match (message.get("id"), method) {
            (Some(id), None) => {
                let Some(id) = id.as_i64() else { return };
                let tx = self.lock().waiting.remove(&id);
                let result = match message.get("error") {
                    Some(error) => Err(explain(
                        self.agent,
                        error
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("The agent returned an error."),
                    )),
                    None => Ok(message.get("result").cloned().unwrap_or(json!({}))),
                };
                if let Some(tx) = tx {
                    let _ = tx.send(result);
                }
            }
            (Some(id), Some(method)) => self.answer(id.clone(), &method, message.get("params")),
            (None, Some(method)) if method == "session/update" => {
                let params = message.get("params");
                let session = params.and_then(|p| p.get("sessionId")).and_then(Value::as_str);
                if let (Some(session), Some(update)) =
                    (session, params.and_then(|p| p.get("update")))
                {
                    deliver(session, Event::Update(update.clone()));
                }
            }
            _ => {}
        }
    }

    /// The agent's own requests: only permission, which is allowed, sent to the window for
    /// review (edits), or declined, by the tool's kind.
    fn answer(&self, id: Value, method: &str, params: Option<&Value>) {
        if method != "session/request_permission" {
            self.write(&json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("Mdow doesn’t offer {method}.")}}));
            return;
        }
        let call = params.and_then(|p| p.get("toolCall")).cloned().unwrap_or(json!({}));
        let options: Vec<Value> = params
            .and_then(|p| p.get("options"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let session = params.and_then(|p| p.get("sessionId")).and_then(Value::as_str).unwrap_or("");
        decide(self.agent, session, id, call, options);
    }
}

/// Answers a permission request by the tool's kind: reads and searches run, edits go to the
/// session's window for review, and the rest is declined and shown in the chat.
pub(super) fn decide(agent: Agent, session: &str, id: Value, call: Value, options: Vec<Value>) {
    let client = client(agent);
    let kind = call.get("kind").and_then(Value::as_str).unwrap_or("other");
    if agents::allows(kind) {
        client.respond(id, &options, true);
        return;
    }
    let reviewable = agent.edits() && (kind == "edit" || !diffs(&call).is_empty());
    if reviewable
        && deliver(
            session,
            Event::Permission { id: id.clone(), call: call.clone(), options: options.clone() },
        )
    {
        return;
    }
    client.respond(id, &options, false);
    // Shown in the chat, so it's clear what the agent wanted and didn't get.
    deliver(session, Event::Update(json!({"sessionUpdate": "mdow_declined", "toolCall": call})));
}

pub fn explain(agent: Agent, message: &str) -> String {
    let lower = message.to_lowercase();
    if ["auth", "log in", "login", "unauthori", "sign in"].iter().any(|w| lower.contains(w)) {
        return agent.sign_in().into();
    }
    message.to_string()
}

/// `MDOW_ACP_LOG=<file>` writes every message, both ways, for debugging.
pub(super) fn log(direction: &str, line: &str) {
    static FILE: LazyLock<Option<Mutex<std::fs::File>>> = LazyLock::new(|| {
        std::env::var_os("MDOW_ACP_LOG").and_then(|p| std::fs::File::create(p).ok()).map(Mutex::new)
    });
    if let Some(file) = FILE.as_ref() {
        let _ = file
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .write_all(format!("{direction}{line}").as_bytes());
    }
}

// MARK: Sessions

/// What a new session starts with: the folder, and for Claude and Codex, Mdow's instructions in
/// their system prompt (an ACP agent gets them with the first message).
pub fn session_params(agent: Agent, directory: &str) -> Value {
    if agent.native() {
        let instructions = super::logic::instructions(agent);
        return json!({"cwd": directory, "_meta": {"systemPrompt": {"append": instructions}}});
    }
    json!({"cwd": directory, "mcpServers": []})
}

/// A reply's settings. An agent that lists its models outside its settings (Claude's adapter)
/// gets a stand-in model setting, so the rest reads one shape.
pub fn options(reply: &Value) -> Vec<Value> {
    let mut options =
        reply.get("configOptions").and_then(Value::as_array).cloned().unwrap_or_default();
    if !options.iter().any(is_model_option)
        && let Some(current) =
            reply.get("models").and_then(|m| m.get("currentModelId")).and_then(Value::as_str)
    {
        options.push(json!({"id": "model", "category": "model", "currentValue": current, "_mdowStandIn": true}));
    }
    options
}

pub fn is_model_option(o: &Value) -> bool {
    o.get("category").and_then(Value::as_str) == Some("model")
        || o.get("id").and_then(Value::as_str) == Some("model")
}

/// A select setting's choices as (value, name), with grouped choices flattened.
fn choices(options: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for o in options.as_array().map(Vec::as_slice).unwrap_or_default() {
        if o.get("options").is_some() {
            out.extend(choices(&o["options"]));
        } else if let Some(value) = o.get("value").and_then(Value::as_str) {
            out.push((
                value.to_string(),
                o.get("name").and_then(Value::as_str).unwrap_or(value).to_string(),
            ));
        }
    }
    out
}

/// The models a session reply lists, as (id, name).
pub fn models_in(reply: &Value) -> Vec<(String, String)> {
    if let Some(listed) =
        reply.get("models").and_then(|m| m.get("availableModels")).and_then(Value::as_array)
    {
        return listed
            .iter()
            .filter_map(|m| {
                let id = m.get("modelId")?.as_str()?;
                Some((
                    id.to_string(),
                    m.get("name").and_then(Value::as_str).unwrap_or(id).to_string(),
                ))
            })
            .collect();
    }
    reply
        .get("configOptions")
        .and_then(Value::as_array)
        .and_then(|a| a.iter().find(|o| is_model_option(o)))
        .map(|o| choices(&o["options"]))
        .unwrap_or_default()
}

/// A model's effort setting as the agent offers it: its id differs by agent ("reasoning_effort",
/// "effort"); `values` is empty for a model without one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Efforts {
    pub config_id: String,
    pub values: Vec<EffortOption>,
    pub current: String,
}

/// The model a session is on and its effort setting, from a settings snapshot.
pub fn efforts(options: &[Value]) -> Option<(String, Efforts)> {
    let s = |o: &Value, k: &str| o.get(k).and_then(Value::as_str).map(str::to_string);
    let model = options.iter().find(|o| is_model_option(o)).and_then(|o| s(o, "currentValue"))?;
    let none = Efforts { config_id: String::new(), values: vec![], current: String::new() };
    let Some(thought) =
        options.iter().find(|o| s(o, "category").as_deref() == Some("thought_level"))
    else {
        return Some((model, none));
    };
    let Some(id) = s(thought, "id") else { return Some((model, none)) };
    let values = choices(&thought["options"])
        .into_iter()
        .map(|(value, name)| EffortOption { value, name })
        .collect();
    Some((
        model,
        Efforts { config_id: id, values, current: s(thought, "currentValue").unwrap_or_default() },
    ))
}

pub fn set_option(
    agent: Agent,
    session: &str,
    id: &str,
    value: &str,
) -> Result<Vec<Value>, String> {
    client(agent)
        .request(
            "session/set_config_option",
            json!({"sessionId": session, "configId": id, "value": value}),
        )
        .map(|r| options(&r))
}

/// Puts a session on a model: through its model setting when it has one (Cursor, Codex), else
/// `session/set_model` (Claude's adapter).
pub fn set_model(
    agent: Agent,
    session: &str,
    known: &[Value],
    model: &str,
) -> Result<Vec<Value>, String> {
    let setting = known
        .iter()
        .find(|o| is_model_option(o) && o.get("_mdowStandIn").is_none())
        .and_then(|o| o.get("id"))
        .and_then(Value::as_str);
    if let Some(id) = setting {
        return set_option(agent, session, id, model);
    }
    client(agent).request("session/set_model", json!({"sessionId": session, "modelId": model}))?;
    let mut options: Vec<Value> = known.iter().filter(|o| !is_model_option(o)).cloned().collect();
    options.push(
        json!({"id": "model", "category": "model", "currentValue": model, "_mdowStandIn": true}),
    );
    Ok(options)
}

// MARK: Translating updates

/// A tool call as it has come in so far.
#[derive(Clone, Debug, Default)]
struct Call {
    title: String,
    kind: String,
    status: String,
    input: Option<String>,
    output: Option<String>,
    files: Vec<FileChange>,
    /// Shown as a change card rather than a step.
    change: bool,
    /// Declined by Mdow or by the user.
    declined: bool,
}

/// Turns one session's ACP updates into the conversation's.
pub struct Translator {
    directory: String,
    calls: HashMap<String, Call>,
    thinking: bool,
    cited: std::collections::HashSet<String>,
}

impl Translator {
    pub fn new(directory: &str) -> Translator {
        Translator {
            directory: directory.to_string(),
            calls: HashMap::new(),
            thinking: false,
            cited: Default::default(),
        }
    }

    fn close_thinking(&mut self, out: &mut Vec<Update>) {
        if self.thinking {
            self.thinking = false;
            out.push(Update::ThinkingDone);
        }
    }

    /// The prose and reasoning chunks, tool calls and their progress.
    pub fn translate(&mut self, update: &Value) -> Vec<Update> {
        let s = |k: &str| update.get(k).and_then(Value::as_str).map(str::to_string);
        let chunk = || {
            update
                .get("content")
                .and_then(|c| c.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let mut out = Vec::new();
        match s("sessionUpdate").as_deref() {
            Some("agent_message_chunk") => {
                let text = chunk();
                if !text.is_empty() {
                    self.close_thinking(&mut out);
                    out.push(Update::Delta(text));
                }
            }
            Some("agent_thought_chunk") => {
                let text = chunk();
                if !text.is_empty() {
                    self.thinking = true;
                    out.push(Update::Thinking(text));
                }
            }
            Some("tool_call" | "tool_call_update") => {
                self.close_thinking(&mut out);
                if let Some(id) = s("toolCallId") {
                    self.merge(&id, update);
                    out.extend(self.emit(&id));
                }
            }
            Some("mdow_declined") => {
                let call = update.get("toolCall").cloned().unwrap_or(json!({}));
                let id = call.get("toolCallId").and_then(Value::as_str).unwrap_or("").to_string();
                let id = if id.is_empty() { new_id("declined") } else { id };
                self.merge(&id, &call);
                if let Some(c) = self.calls.get_mut(&id) {
                    c.declined = true;
                    c.change = false;
                }
                out.extend(self.emit(&id));
            }
            _ => {}
        }
        out
    }

    fn merge(&mut self, id: &str, update: &Value) {
        let directory = self.directory.clone();
        let call = self.calls.entry(id.to_string()).or_default();
        let s = |k: &str| update.get(k).and_then(Value::as_str).map(str::to_string);
        if let Some(title) = s("title").filter(|t| !t.is_empty()) {
            call.title = short_title(&title, &directory);
        }
        if let Some(kind) = s("kind") {
            call.kind = kind;
        }
        if let Some(status) = s("status")
            && call.status != "rejected"
        {
            call.status = status;
        }
        if let Some(raw) = update.get("rawInput").filter(|v| !v.is_null()) {
            call.input = serde_json::to_string_pretty(raw).ok();
        }
        let files = diffs(update)
            .into_iter()
            .map(|(path, old, new)| file_change(&path, old.as_deref(), &new, &directory))
            .collect::<Vec<_>>();
        if !files.is_empty() {
            call.files = files;
            call.change = true;
        } else if call.kind == "edit" {
            call.change = true;
        }
        let text: Vec<String> = update
            .get("content")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|c| {
                        c.get("content").and_then(|c| c.get("text")).and_then(Value::as_str)
                    })
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if !text.is_empty() {
            call.output = Some(clip(&text.join("\n")));
        }
    }

    /// The conversation's view of a call: a step, or a change card for an edit.
    fn emit(&mut self, id: &str) -> Vec<Update> {
        let Some(call) = self.calls.get(id).cloned() else { return Vec::new() };
        let mut out = Vec::new();
        if call.change && !call.declined {
            // An edit shows once it settles; a pending one shows when its review is asked for.
            let status = match call.status.as_str() {
                "completed" => ChangeStatus::Applied,
                "failed" => ChangeStatus::Failed,
                "rejected" => ChangeStatus::Rejected,
                _ => return out,
            };
            out.push(Update::Change {
                tool_call_id: id.to_string(),
                permission_id: None,
                status,
                files: (!call.files.is_empty()).then(|| call.files.clone()),
                error: (status == ChangeStatus::Failed)
                    .then(|| call.output.clone().unwrap_or_else(|| "The edit failed.".into())),
            });
            return out;
        }
        let state = if call.declined {
            ToolState::Cancelled
        } else {
            match call.status.as_str() {
                "completed" => ToolState::Completed,
                "failed" => ToolState::Error,
                "in_progress" => ToolState::Running,
                _ => ToolState::Pending,
            }
        };
        let kind = if call.declined { "declined".to_string() } else { call.kind.clone() };
        let title = if call.declined {
            if call.kind == "execute" {
                "Declined running a command".into()
            } else {
                format!("Declined: {}", call.title)
            }
        } else if call.title.is_empty() {
            "Working".into()
        } else {
            call.title.clone()
        };
        let mut part = ToolPart::new(id, title, state)
            .kind(Some(kind))
            .input(call.input.clone())
            .output(call.output.clone());
        if state == ToolState::Error {
            part = part.error(call.output.clone().or(Some("The tool failed.".into())));
        }
        out.push(Update::Tool(part));
        if call.kind == "read" && state == ToolState::Completed {
            out.extend(self.cite(&call));
        }
        out
    }

    /// A document the agent read, for "Used N sources".
    fn cite(&mut self, call: &Call) -> Vec<Update> {
        let Some(path) =
            call.input.as_deref().and_then(|i| serde_json::from_str::<Value>(i).ok()).and_then(
                |v| {
                    ["path", "file_path", "filePath", "target_file"]
                        .iter()
                        .find_map(|k| v.get(*k).and_then(Value::as_str).map(str::to_string))
                },
            )
        else {
            return Vec::new();
        };
        let absolute =
            if path.starts_with('/') { path } else { format!("{}/{path}", self.directory) };
        if !crate::files::is_document(std::path::Path::new(&absolute))
            || !self.cited.insert(absolute.clone())
        {
            return Vec::new();
        }
        vec![Update::Citation(Citation {
            source_id: format!("read:{absolute}"),
            label: basename(&absolute),
            path: absolute,
        })]
    }

    /// A review the agent asks for: the change card, waiting on `permission_id`.
    pub fn permission(&mut self, permission_id: &str, call: &Value) -> Update {
        let id = call
            .get("toolCallId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| new_id("edit"));
        self.merge(&id, call);
        let files = self.calls.get(&id).map(|c| c.files.clone()).unwrap_or_default();
        Update::Change {
            tool_call_id: id,
            permission_id: Some(permission_id.to_string()),
            status: ChangeStatus::Pending,
            files: (!files.is_empty()).then_some(files),
            error: None,
        }
    }

    /// The user declined an edit: its later "failed" reads as declined.
    pub fn declined(&mut self, tool_call_id: &str) {
        if let Some(c) = self.calls.get_mut(tool_call_id) {
            c.status = "rejected".into();
        }
    }
}

/// "Read /Users/me/notes/a.md" → "Read a.md": paths in the folder are shortened.
fn short_title(title: &str, directory: &str) -> String {
    let base =
        if directory.ends_with('/') { directory.to_string() } else { format!("{directory}/") };
    let title = title.replace(&base, "");
    title.replace('`', "")
}

fn clip(text: &str) -> String {
    const MAX: usize = 4000;
    if text.len() <= MAX {
        return text.to_string();
    }
    let mut cut = MAX;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}…", &text[..cut])
}

/// The diffs in a tool call's content, as (path, old text, new text).
fn diffs(call: &Value) -> Vec<(String, Option<String>, String)> {
    call.get("content")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|c| c.get("type").and_then(Value::as_str) == Some("diff"))
                .filter_map(|c| {
                    let path = c.get("path")?.as_str()?.to_string();
                    let old = c.get("oldText").and_then(Value::as_str).map(str::to_string);
                    let new = c.get("newText").and_then(Value::as_str).unwrap_or("").to_string();
                    Some((path, old, new))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn file_change(path: &str, old: Option<&str>, new: &str, directory: &str) -> FileChange {
    let display = display_path(path, directory);
    let (patch, additions, deletions) = unified_diff(&display, old.unwrap_or(""), new);
    FileChange {
        path: path.to_string(),
        display_path: display,
        patch,
        additions,
        deletions,
        status: if old.is_none() { FileStatus::Added } else { FileStatus::Modified },
    }
}

/// A unified diff of two texts with three lines of context, and its added and removed counts.
pub fn unified_diff(name: &str, old: &str, new: &str) -> (String, i64, i64) {
    let a: Vec<&str> = if old.is_empty() { Vec::new() } else { old.lines().collect() };
    let b: Vec<&str> = new.lines().collect();
    // The longest common subsequence, by table; very large documents fall back to replacing
    // the lines between the shared head and tail.
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let tail =
        a[head..].iter().rev().zip(b[head..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (am, bm) = (&a[head..a.len() - tail], &b[head..b.len() - tail]);
    let mut ops: Vec<(char, usize, usize)> = (0..head).map(|i| (' ', i, i)).collect();
    if am.len() * bm.len() <= 4_000_000 {
        let (n, m) = (am.len(), bm.len());
        let mut t = vec![0u32; (n + 1) * (m + 1)];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                t[i * (m + 1) + j] = if am[i] == bm[j] {
                    t[(i + 1) * (m + 1) + j + 1] + 1
                } else {
                    t[(i + 1) * (m + 1) + j].max(t[i * (m + 1) + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && am[i] == bm[j] {
                ops.push((' ', head + i, head + j));
                i += 1;
                j += 1;
            } else if i < n && (j == m || t[(i + 1) * (m + 1) + j] >= t[i * (m + 1) + j + 1]) {
                // Removals come before the additions that replace them.
                ops.push(('-', head + i, head + j));
                i += 1;
            } else {
                ops.push(('+', head + i, head + j));
                j += 1;
            }
        }
    } else {
        ops.extend((0..am.len()).map(|i| ('-', head + i, head)));
        ops.extend((0..bm.len()).map(|j| ('+', head + am.len(), head + j)));
    }
    ops.extend((0..tail).map(|k| (' ', a.len() - tail + k, b.len() - tail + k)));

    let (additions, deletions) = ops.iter().fold((0, 0), |(x, y), o| match o.0 {
        '+' => (x + 1, y),
        '-' => (x, y + 1),
        _ => (x, y),
    });
    let mut out = format!("--- {name}\n+++ {name}\n");
    let changed: Vec<usize> =
        ops.iter().enumerate().filter(|(_, o)| o.0 != ' ').map(|(i, _)| i).collect();
    let mut k = 0;
    while k < changed.len() {
        let start = changed[k].saturating_sub(3);
        let mut end = changed[k] + 3;
        while k + 1 < changed.len() && changed[k + 1] <= end + 3 {
            k += 1;
            end = changed[k] + 3;
        }
        let end = end.min(ops.len() - 1);
        let hunk = &ops[start..=end];
        let old_count = hunk.iter().filter(|o| o.0 != '+').count();
        let new_count = hunk.iter().filter(|o| o.0 != '-').count();
        let (old_start, new_start) = (hunk[0].1 + 1, hunk[0].2 + 1);
        out.push_str(&format!("@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"));
        for (op, i, j) in hunk {
            let line = if *op == '+' { b[*j] } else { a[*i] };
            out.push(*op);
            out.push_str(line);
            out.push('\n');
        }
        k += 1;
    }
    (out, additions, deletions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffs_a_one_line_fix_with_context() {
        let (patch, add, del) =
            unified_diff("a.md", "# A\n\nTeh fox.\nEnd\n", "# A\n\nThe fox.\nEnd\n");
        assert_eq!((add, del), (1, 1));
        assert_eq!(
            patch,
            "--- a.md\n+++ a.md\n@@ -1,4 +1,4 @@\n # A\n \n-Teh fox.\n+The fox.\n End\n"
        );
        let hunks = super::super::logic::parse_diff(&patch);
        assert_eq!(hunks.len(), 1);
        let (_, add, del) = unified_diff("b.md", "", "new\nfile\n");
        assert_eq!((add, del), (2, 0));
    }

    #[test]
    fn translates_chunks_tools_and_declines() {
        let mut t = Translator::new("/docs");
        let ups = t.translate(&json!({"sessionUpdate": "agent_thought_chunk", "content": {"type": "text", "text": "Hm"}}));
        assert_eq!(ups, vec![Update::Thinking("Hm".into())]);
        let ups = t.translate(&json!({"sessionUpdate": "tool_call", "toolCallId": "t1", "title": "Read /docs/a.md", "kind": "read", "status": "pending", "rawInput": {"path": "/docs/a.md"}}));
        assert_eq!(ups[0], Update::ThinkingDone);
        let Update::Tool(tool) = &ups[1] else { panic!() };
        assert_eq!((tool.name.as_str(), tool.kind.as_deref()), ("Read a.md", Some("read")));
        let ups = t.translate(&json!({"sessionUpdate": "tool_call_update", "toolCallId": "t1", "status": "completed"}));
        assert!(
            matches!(&ups[0], Update::Tool(t) if t.state == ToolState::Completed && t.name == "Read a.md")
        );
        assert!(matches!(&ups[1], Update::Citation(c) if c.path == "/docs/a.md"));
        let ups = t.translate(&json!({"sessionUpdate": "mdow_declined", "toolCall": {"toolCallId": "t2", "title": "ls", "kind": "execute"}}));
        assert!(
            matches!(&ups[0], Update::Tool(t) if t.kind.as_deref() == Some("declined") && t.state == ToolState::Cancelled)
        );
    }

    #[test]
    fn edits_become_change_cards() {
        let mut t = Translator::new("/docs");
        let call = json!({"toolCallId": "e1", "title": "Edit a.md", "kind": "edit", "status": "pending",
            "content": [{"type": "diff", "path": "/docs/a.md", "oldText": "teh\n", "newText": "the\n"}]});
        let mut update = call.clone();
        update["sessionUpdate"] = json!("tool_call");
        assert!(t.translate(&update).is_empty());
        let Update::Change { permission_id, status, files, .. } = t.permission("acp-1", &call)
        else {
            panic!()
        };
        assert_eq!((permission_id.as_deref(), status), (Some("acp-1"), ChangeStatus::Pending));
        assert_eq!(files.unwrap()[0].display_path, "a.md");
        let ups = t.translate(&json!({"sessionUpdate": "tool_call_update", "toolCallId": "e1", "status": "completed"}));
        assert!(matches!(&ups[0], Update::Change { status: ChangeStatus::Applied, .. }));
    }

    #[test]
    fn reads_models_and_efforts_from_settings() {
        let reply = json!({"configOptions": [
            {"id": "model", "category": "model", "currentValue": "gpt-6", "options": [{"value": "gpt-6", "name": "GPT-6"}, {"value": "gpt-6-mini", "name": "GPT-6 Mini"}]},
            {"id": "reasoning_effort", "category": "thought_level", "currentValue": "medium", "options": [{"value": "low", "name": "Low"}, {"value": "medium", "name": "Medium"}]}
        ]});
        assert_eq!(models_in(&reply).len(), 2);
        let (model, e) = efforts(&options(&reply)).unwrap();
        assert_eq!(
            (model.as_str(), e.config_id.as_str(), e.values.len()),
            ("gpt-6", "reasoning_effort", 2)
        );
        let claude = json!({"models": {"currentModelId": "sonnet", "availableModels": [{"modelId": "sonnet", "name": "Sonnet"}]}});
        assert_eq!(models_in(&claude), vec![("sonnet".to_string(), "Sonnet".to_string())]);
        assert!(options(&claude).iter().any(is_model_option));
    }
}
