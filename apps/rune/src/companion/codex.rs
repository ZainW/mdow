//! Codex, through its own app server (`codex app-server`): OpenAI's interface for apps that run
//! Codex, built into the `codex` program. Its protocol is JSON-RPC, one message per line on
//! stdin and stdout; `codex app-server generate-ts` prints its types. No adapter, no Node.
//!
//! One server serves every window; each chat is an ephemeral thread on it, kept out of Codex's
//! own history. A thread runs in the workspace-write sandbox with the "untrusted" approval
//! policy, so Codex asks before every patch and every command outside its known-safe set (the
//! read-only ones, which run without asking). Mdow answers by the same rules as for the other
//! agents (`acp::decide`): patches inside the folder go to the window as diffs to accept or
//! decline, and every command it asks about is declined, since a command Codex isn't sure of
//! can write however its parse labels it. (In the read-only sandbox Codex won't propose edits
//! at all.)
//!
//! Like `claude.rs`, the module answers the requests an ACP agent does and reports in ACP's
//! shapes; [`super::acp::Client`] hands Codex's requests here.

use super::acp::{self, Event};
use super::agents::Agent;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{LazyLock, Mutex, MutexGuard};

/// A chat: one thread on the server.
#[derive(Default)]
struct Thread {
    directory: String,
    model: String,
    effort: String,
    /// The turn running now, and who waits on it.
    turn: Option<String>,
    done: Option<Sender<Result<Value, String>>>,
    cancelled: bool,
    /// Stopped turns still winding down: what they say, and their end, isn't the new turn's.
    stale: HashSet<String>,
    /// Patches as Codex proposed them, by item.
    changes: HashMap<String, Value>,
    /// Text has been said this turn, and the message saying it now.
    said: bool,
    message: String,
    /// Items whose text or summary arrived in parts.
    streamed: HashSet<String>,
}

impl Thread {
    /// Makes `tx` the turn's waiter. A stopped turn that hasn't ended yet: its end isn't this
    /// one's.
    fn begin(&mut self, tx: Sender<Result<Value, String>>) {
        if let Some(old) = self.turn.take() {
            self.stale.insert(old);
        }
        self.done = Some(tx);
        self.cancelled = false;
    }
}

#[derive(Default)]
struct State {
    stdin: Option<ChildStdin>,
    child: Option<Child>,
    next: i64,
    waiting: HashMap<i64, Sender<Result<Value, String>>>,
    threads: HashMap<String, Thread>,
    /// From `model/list`, once per server.
    models: Vec<Value>,
    errors: String,
    generation: u64,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(Default::default);
/// Held while the server starts, so concurrent first requests wait for one launch.
static STARTING: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn thread_id(params: &Value) -> String {
    params.get("sessionId").or(params.get("threadId")).and_then(Value::as_str).unwrap_or("").into()
}

/// An ACP request, answered by Codex's app server.
pub fn request(method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "initialize" | "session/set_mode" => Ok(json!({})),
        "session/new" => new_thread(
            params.get("cwd").and_then(Value::as_str).unwrap_or("/"),
            params.pointer("/_meta/systemPrompt/append").and_then(Value::as_str).unwrap_or(""),
        ),
        "session/prompt" => {
            let text: Vec<&str> = params
                .get("prompt")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|b| b.get("text").and_then(Value::as_str)).collect())
                .unwrap_or_default();
            prompt(&thread_id(params), &text.join("\n\n"))
        }
        "session/set_model" | "session/set_config_option" => {
            let id = thread_id(params);
            let value = params.get("value").or(params.get("modelId")).and_then(Value::as_str);
            let value = value.unwrap_or("").to_string();
            let setting = params.get("configId").and_then(Value::as_str).unwrap_or("model");
            let mut st = lock();
            let models = st.models.clone();
            let t = st.threads.get_mut(&id).ok_or("This chat with Codex has ended.")?;
            // Both apply from the next turn: `turn/start` carries them.
            match setting {
                "model" => {
                    t.model = value;
                    let levels = efforts(&models, &t.model);
                    if !levels.iter().any(|(l, _)| *l == t.effort) {
                        t.effort = default_effort(&models, &t.model);
                    }
                }
                "effort" => t.effort = value,
                _ => return Err("Codex has no such setting.".into()),
            }
            Ok(json!({"configOptions": options(&models, t)}))
        }
        _ => Err(format!("Codex doesn’t answer {method}.")),
    }
}

/// An ACP notification: only `session/cancel`, which interrupts the turn.
pub fn notify(method: &str, params: &Value) {
    if method == "session/cancel" {
        interrupt(&thread_id(params));
    }
}

/// Answers an approval sent as [`Event::Permission`].
pub fn respond(id: &Value, allow: bool) {
    let Some(rpc) = id.get("rpc") else { return };
    let decision = if allow { "accept" } else { "decline" };
    write(&json!({"id": rpc, "result": {"decision": decision}}));
}

/// Whether the server is running and still has the thread (a restarted one has none).
pub fn alive(thread: &str) -> bool {
    let st = lock();
    st.stdin.is_some() && st.threads.contains_key(thread)
}

/// Ends a chat: its turn stops and the server lets the thread go.
pub fn close(thread: &str) {
    interrupt(thread);
    if lock().threads.remove(thread).is_some() {
        send_unanswered("thread/unsubscribe", json!({"threadId": thread}));
    }
}

pub fn stop_all() {
    let mut st = lock();
    st.stdin = None;
    st.threads.clear();
    if let Some(mut child) = st.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn write(message: &Value) {
    let mut line = message.to_string();
    line.push('\n');
    acp::log(">> ", &line);
    let mut st = lock();
    if let Some(stdin) = st.stdin.as_mut()
        && stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()).is_err()
    {
        st.stdin = None;
    }
}

/// Sends a request and waits for its reply.
fn call(method: &str, params: Value) -> Result<Value, String> {
    let (tx, rx) = channel();
    let id = {
        let mut st = lock();
        if st.stdin.is_none() {
            return Err("Codex isn’t running.".into());
        }
        st.next += 1;
        let id = st.next;
        st.waiting.insert(id, tx);
        id
    };
    write(&json!({"id": id, "method": method, "params": params}));
    acp::wait(&rx, Agent::Codex, false)
}

/// Sends a request whose reply nothing waits on.
fn send_unanswered(method: &str, params: Value) {
    let id = {
        let mut st = lock();
        st.next += 1;
        st.next
    };
    write(&json!({"id": id, "method": method, "params": params}));
}

fn start() -> Result<(), String> {
    let _starting = STARTING.lock().unwrap_or_else(|e| e.into_inner());
    if lock().stdin.is_some() {
        return Ok(());
    }
    let program = Agent::Codex.command()?;
    let mut child = Command::new(&program[0])
        .args(&program[1..])
        .current_dir(super::logic::home())
        .env("PATH", super::process::environment_path())
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn’t start Codex at {}: {e}", program[0]))?;
    let stdout = child.stdout.take().ok_or("No output from Codex.")?;
    let stderr = child.stderr.take().ok_or("No output from Codex.")?;
    let generation = {
        let mut st = lock();
        st.generation += 1;
        st.stdin = child.stdin.take();
        st.child = Some(child);
        st.errors.clear();
        st.models.clear();
        st.generation
    };
    acp::read_errors(stderr, |part| acp::keep_errors(&mut lock().errors, part));
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).split(b'\n') {
            let Ok(line) = line else { break };
            if line.is_empty() {
                continue;
            }
            acp::log("<< ", &format!("{}\n", String::from_utf8_lossy(&line)));
            if let Ok(message) = serde_json::from_slice::<Value>(&line) {
                handle(message);
            }
        }
        exited(generation);
    });
    let init = call(
        "initialize",
        json!({
            "clientInfo": {"name": "mdow", "title": "Mdow", "version": env!("CARGO_PKG_VERSION")},
            "capabilities": {"experimentalApi": false, "requestAttestation": false},
        }),
    );
    if let Err(e) = init {
        stop_all();
        return Err(e);
    }
    write(&json!({"method": "initialized"}));
    Ok(())
}

fn exited(generation: u64) {
    let (waiting, done, errors) = {
        let mut st = lock();
        if st.generation != generation {
            return;
        }
        st.stdin = None;
        if let Some(mut child) = st.child.take() {
            let _ = child.wait();
        }
        // Its threads went with it.
        let done: Vec<_> = st.threads.drain().filter_map(|(_, mut t)| t.done.take()).collect();
        (std::mem::take(&mut st.waiting), done, std::mem::take(&mut st.errors))
    };
    let failure = acp::stopped(Agent::Codex, &errors);
    for tx in waiting.into_values().chain(done) {
        let _ = tx.send(Err(failure.clone()));
    }
}

fn new_thread(directory: &str, instructions: &str) -> Result<Value, String> {
    start()?;
    if lock().models.is_empty() {
        let mut models = Vec::new();
        let mut cursor = Value::Null;
        loop {
            let page = call("model/list", json!({"cursor": cursor}))?;
            models.extend(page.get("data").and_then(Value::as_array).cloned().unwrap_or_default());
            cursor = page.get("nextCursor").cloned().unwrap_or(Value::Null);
            if cursor.is_null() {
                break;
            }
        }
        lock().models = models;
    }
    let reply = call(
        "thread/start",
        json!({
            "cwd": directory,
            "sandbox": "workspace-write",
            "approvalPolicy": "untrusted",
            "ephemeral": true,
            "developerInstructions": instructions,
            // The user's MCP servers stay out: faster to start, and nothing to approve.
            "config": {"mcp_servers": {}},
        }),
    )
    .map_err(|e| acp::explain(Agent::Codex, &e))?;
    let id = reply
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .ok_or("Codex didn’t start a chat.")?
        .to_string();
    let mut st = lock();
    let model = reply.get("model").and_then(Value::as_str).unwrap_or("").to_string();
    let effort = reply
        .get("reasoningEffort")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| default_effort(&st.models, &model));
    let thread = Thread { directory: directory.to_string(), model, effort, ..Thread::default() };
    let options = options(&st.models, &thread);
    st.threads.insert(id.clone(), thread);
    Ok(json!({"sessionId": id, "configOptions": options}))
}

fn prompt(id: &str, text: &str) -> Result<Value, String> {
    let (tx, rx) = channel();
    let (model, effort) = {
        let mut st = lock();
        let t = st.threads.get_mut(id).ok_or("This chat with Codex has ended.")?;
        t.begin(tx);
        t.said = false;
        t.message.clear();
        (t.model.clone(), t.effort.clone())
    };
    let mut params = json!({
        "threadId": id,
        "input": [{"type": "text", "text": text, "text_elements": []}],
        "summary": "auto",
    });
    if !model.is_empty() {
        params["model"] = json!(model);
    }
    if !effort.is_empty() {
        params["effort"] = json!(effort);
    }
    match call("turn/start", params) {
        Ok(reply) => {
            if let Some(turn) = reply.pointer("/turn/id").and_then(Value::as_str)
                && let Some(t) = lock().threads.get_mut(id)
            {
                t.turn = Some(turn.to_string());
            }
        }
        Err(e) => {
            if let Some(t) = lock().threads.get_mut(id) {
                t.done = None;
            }
            return Err(acp::explain(Agent::Codex, &e));
        }
    }
    acp::wait(&rx, Agent::Codex, true)
}

fn interrupt(id: &str) {
    let turn = {
        let mut st = lock();
        let Some(t) = st.threads.get_mut(id) else { return };
        if t.done.is_none() {
            return;
        }
        t.cancelled = true;
        t.turn.clone()
    };
    if let Some(turn) = turn {
        send_unanswered("turn/interrupt", json!({"threadId": id, "turnId": turn}));
    }
}

/// A thread's settings in ACP's shape: its model, and the model's reasoning effort.
fn options(models: &[Value], t: &Thread) -> Vec<Value> {
    let choices: Vec<Value> = models
        .iter()
        .filter(|m| !m.get("hidden").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|m| {
            let value = slug(m)?;
            let name = m.get("displayName").and_then(Value::as_str).unwrap_or(value);
            Some(json!({"value": value, "name": name}))
        })
        .collect();
    let mut out = vec![
        json!({"id": "model", "category": "model", "currentValue": t.model, "options": choices}),
    ];
    let levels = efforts(models, &t.model);
    if !levels.is_empty() {
        let choices: Vec<Value> =
            levels.into_iter().map(|(value, name)| json!({"value": value, "name": name})).collect();
        out.push(json!({"id": "effort", "category": "thought_level", "currentValue": t.effort, "options": choices}));
    }
    out
}

fn slug(model: &Value) -> Option<&str> {
    model.get("model").or(model.get("id")).and_then(Value::as_str)
}

fn find_model<'a>(models: &'a [Value], model: &str) -> Option<&'a Value> {
    models.iter().find(|m| slug(m) == Some(model))
}

/// A model's reasoning efforts, as (value, name).
fn efforts(models: &[Value], model: &str) -> Vec<(String, String)> {
    find_model(models, model)
        .and_then(|m| m.get("supportedReasoningEfforts"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|e| e.get("reasoningEffort").and_then(Value::as_str))
                .map(|l| (l.to_string(), acp::effort_name(l)))
                .collect()
        })
        .unwrap_or_default()
}

fn default_effort(models: &[Value], model: &str) -> String {
    find_model(models, model)
        .and_then(|m| m.get("defaultReasoningEffort"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn update(thread: &str, update: Value) {
    acp::deliver(thread, Event::Update(update));
}

fn handle(message: Value) {
    let method = message.get("method").and_then(Value::as_str).map(str::to_string);
    match (message.get("id"), method) {
        (Some(id), None) => {
            let Some(id) = id.as_i64() else { return };
            let Some(tx) = lock().waiting.remove(&id) else { return };
            let result = match message.get("error") {
                Some(e) => Err(acp::explain(
                    Agent::Codex,
                    e.get("message").and_then(Value::as_str).unwrap_or("Codex returned an error."),
                )),
                None => Ok(message.get("result").cloned().unwrap_or(json!({}))),
            };
            let _ = tx.send(result);
        }
        (Some(id), Some(method)) => approval(id.clone(), &method, message.get("params")),
        (None, Some(method)) => {
            notification(&method, message.get("params").unwrap_or(&Value::Null))
        }
        _ => {}
    }
}

fn notification(method: &str, p: &Value) {
    let s = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let thread = s("threadId");
    let turn = p.get("turnId").or(p.pointer("/turn/id")).and_then(Value::as_str).unwrap_or("");
    // A stopped turn's last words, and its end, once a new turn has begun.
    let stale = {
        let mut st = lock();
        match st.threads.get_mut(&thread) {
            Some(t) if method == "turn/completed" => t.stale.remove(turn),
            Some(t) => t.stale.contains(turn),
            None => false,
        }
    };
    if stale {
        return;
    }
    match method {
        "item/agentMessage/delta" => {
            let (item, delta) = (s("itemId"), s("delta"));
            if delta.is_empty() {
                return;
            }
            let text = {
                let mut st = lock();
                let Some(t) = st.threads.get_mut(&thread) else { return };
                t.streamed.insert(item.clone());
                // Each message of the turn its own paragraph.
                let gap = t.said && t.message != item;
                t.said = true;
                t.message = item;
                if gap { format!("\n\n{delta}") } else { delta }
            };
            update(
                &thread,
                json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}),
            );
        }
        "item/reasoning/summaryTextDelta" => {
            let delta = s("delta");
            if delta.is_empty() {
                return;
            }
            if let Some(t) = lock().threads.get_mut(&thread) {
                t.streamed.insert(s("itemId"));
            }
            update(
                &thread,
                json!({"sessionUpdate": "agent_thought_chunk", "content": {"type": "text", "text": delta}}),
            );
        }
        "item/started" => started(&thread, p.get("item").unwrap_or(&Value::Null)),
        "item/completed" => completed(&thread, p.get("item").unwrap_or(&Value::Null)),
        "turn/started" => {
            if let Some(t) = lock().threads.get_mut(&thread) {
                t.turn = p.pointer("/turn/id").and_then(Value::as_str).map(str::to_string);
            }
        }
        "turn/completed" => {
            let turn = p.get("turn").unwrap_or(&Value::Null);
            let (done, cancelled) = {
                let mut st = lock();
                let Some(t) = st.threads.get_mut(&thread) else { return };
                t.turn = None;
                t.streamed.clear();
                t.changes.clear();
                t.stale.clear();
                (t.done.take(), std::mem::take(&mut t.cancelled))
            };
            let Some(tx) = done else { return };
            let status = turn.get("status").and_then(Value::as_str).unwrap_or("");
            let reply = match status {
                _ if cancelled => Ok("cancelled"),
                "interrupted" => Ok("cancelled"),
                "failed" => Err(acp::explain(
                    Agent::Codex,
                    turn.pointer("/error/message")
                        .and_then(Value::as_str)
                        .unwrap_or("Codex stopped with an error."),
                )),
                _ => Ok("end_turn"),
            };
            let _ = tx.send(reply.map(|r| json!({"stopReason": r})));
        }
        _ => {}
    }
}

fn started(thread: &str, item: &Value) {
    let s = |k: &str| item.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let directory = match lock().threads.get(thread) {
        Some(t) => t.directory.clone(),
        None => return,
    };
    let call = match s("type").as_str() {
        "commandExecution" => command_call(item, &directory),
        "fileChange" => {
            let changes = item.get("changes").cloned().unwrap_or(json!([]));
            if let Some(t) = lock().threads.get_mut(thread) {
                t.changes.insert(s("id"), changes.clone());
            }
            change_call(&s("id"), &changes, &directory)
        }
        "webSearch" => json!({"toolCallId": s("id"), "title": "Search the web", "kind": "fetch"}),
        "mcpToolCall" => json!({"toolCallId": s("id"), "title": s("tool"), "kind": "other"}),
        _ => return,
    };
    let mut call = call;
    call["sessionUpdate"] = json!("tool_call");
    call["status"] = json!("in_progress");
    update(thread, call);
}

fn completed(thread: &str, item: &Value) {
    let s = |k: &str| item.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let id = s("id");
    let streamed = lock().threads.get(thread).is_some_and(|t| t.streamed.contains(&id));
    let status = match s("status").as_str() {
        "completed" => "completed",
        _ => "failed",
    };
    match s("type").as_str() {
        "agentMessage" if !streamed && !s("text").is_empty() => {
            let text = {
                let mut st = lock();
                let Some(t) = st.threads.get_mut(thread) else { return };
                let gap = t.said;
                t.said = true;
                t.message = id;
                if gap { format!("\n\n{}", s("text")) } else { s("text") }
            };
            update(
                thread,
                json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}),
            );
        }
        "reasoning" if !streamed => {
            let summary: Vec<&str> = item
                .get("summary")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            if !summary.is_empty() {
                update(
                    thread,
                    json!({"sessionUpdate": "agent_thought_chunk", "content": {"type": "text", "text": summary.join("\n\n")}}),
                );
            }
        }
        "commandExecution" => {
            let output = s("aggregatedOutput");
            update(
                thread,
                json!({
                    "sessionUpdate": "tool_call_update",
                    "toolCallId": id,
                    "status": status,
                    "content": [{"type": "content", "content": {"type": "text", "text": output}}],
                }),
            );
        }
        "fileChange" | "webSearch" | "mcpToolCall" => {
            update(
                thread,
                json!({"sessionUpdate": "tool_call_update", "toolCallId": id, "status": status}),
            );
        }
        _ => {}
    }
}

/// A command as a tool call: a read or a search when every part of it only reads the folder.
fn command_call(item: &Value, directory: &str) -> Value {
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let actions: Vec<Value> =
        item.get("commandActions").and_then(Value::as_array).cloned().unwrap_or_default();
    let inside =
        |a: &Value| a.get("path").and_then(Value::as_str).is_none_or(|p| acp::within(p, directory));
    let kinds: HashSet<String> = actions.iter().map(|a| s(a, "type")).collect();
    let reads = !actions.is_empty()
        && actions.iter().all(inside)
        && kinds.iter().all(|k| matches!(k.as_str(), "read" | "listFiles" | "search"));
    let first = actions.first().cloned().unwrap_or(json!({}));
    let (kind, title) = if !reads {
        let command =
            actions.first().map(|a| s(a, "command")).unwrap_or_else(|| s(item, "command"));
        ("execute", format!("Run {command}"))
    } else if kinds.len() == 1 && kinds.contains("read") {
        let names: Vec<String> = actions.iter().map(|a| s(a, "path")).collect();
        ("read", format!("Read {}", names.join(", ")))
    } else if kinds.contains("search") {
        let query = actions.iter().find_map(|a| a.get("query").and_then(Value::as_str));
        ("search", query.map(|q| format!("Search for {q}")).unwrap_or("Search the folder".into()))
    } else {
        (
            "search",
            format!("List {}", first.get("path").and_then(Value::as_str).unwrap_or("the folder")),
        )
    };
    // A single read names its file, for "Used N sources".
    let mut input = json!({"command": s(item, "command")});
    if kind == "read" && actions.len() == 1 {
        input["path"] = json!(s(&first, "path"));
    }
    json!({"toolCallId": s(item, "id"), "title": title, "kind": kind, "rawInput": input})
}

/// A patch as a tool call, with each file as ACP's diff content. Deletes, moves and files
/// outside the folder make it something other than an edit, which is declined.
fn change_call(id: &str, changes: &Value, directory: &str) -> Value {
    let list = changes.as_array().cloned().unwrap_or_default();
    let mut kind = "edit";
    let mut content = Vec::new();
    let mut paths = Vec::new();
    for c in &list {
        let path = c.get("path").and_then(Value::as_str).unwrap_or("");
        let change = c.pointer("/kind/type").and_then(Value::as_str).unwrap_or("");
        let moved = c.pointer("/kind/move_path").is_some_and(|m| !m.is_null());
        let diff = c.get("diff").and_then(Value::as_str).unwrap_or("");
        paths.push(path.to_string());
        if change == "delete" {
            kind = "delete";
        } else if moved {
            kind = "move";
        } else if !acp::within(path, directory) {
            kind = "other";
        }
        let old = if change == "add" { None } else { std::fs::read_to_string(path).ok() };
        let new = match (&old, diff.contains("@@")) {
            (Some(old), true) => apply(old, diff),
            (None, true) => apply("", diff),
            (_, false) => Some(diff.to_string()),
        };
        if let Some(new) = new {
            content.push(json!({"type": "diff", "path": path, "oldText": old, "newText": new}));
        }
    }
    let verb = if kind == "delete" { "Delete" } else { "Edit" };
    json!({"toolCallId": id, "title": format!("{verb} {}", paths.join(", ")), "kind": kind, "content": content})
}

/// Whether a command only reads the folder: read-only programs, perhaps piped, with no
/// redirection, substitution or chaining, no flag that writes or runs something, and no path
/// outside the folder. Anything it can't be sure of is false.
fn reads_only(command: &str, directory: &str) -> bool {
    let Some(segments) = words(command) else { return false };
    // Codex runs commands through a login shell: `/usr/bin/bash -lc 'cat notes.md'`.
    if let [only] = segments.as_slice()
        && let [shell, flag, inner] = only.as_slice()
        && matches!(program(shell), "bash" | "zsh" | "sh")
        && matches!(flag.as_str(), "-c" | "-lc")
    {
        return reads_only(inner, directory);
    }
    segments.iter().all(|w| safe(w, directory))
}

fn program(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// A command's words, split at pipes, as a POSIX shell would quote them; None for anything but
/// plain words and pipes.
fn words(command: &str) -> Option<Vec<Vec<String>>> {
    let mut segments = vec![Vec::new()];
    let mut word = String::new();
    let mut quoted = false;
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();
    let end_word = |segments: &mut Vec<Vec<String>>, word: &mut String, quoted: &mut bool| {
        if *quoted || !word.is_empty() {
            segments.last_mut().unwrap().push(std::mem::take(word));
        }
        *quoted = false;
    };
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '$' | '`' | '\\') => return None,
            (Some(_), c) => word.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                quoted = true;
            }
            (None, ' ' | '\t') => end_word(&mut segments, &mut word, &mut quoted),
            (None, '|') => {
                if chars.peek() == Some(&'|') {
                    return None;
                }
                end_word(&mut segments, &mut word, &mut quoted);
                if segments.last()?.is_empty() {
                    return None;
                }
                segments.push(Vec::new());
            }
            (None, ';' | '&' | '>' | '<' | '$' | '`' | '(' | ')' | '{' | '}' | '\\' | '\n') => {
                return None;
            }
            (None, c) => word.push(c),
        }
    }
    if quote.is_some() {
        return None;
    }
    end_word(&mut segments, &mut word, &mut quoted);
    segments.iter().all(|s| !s.is_empty()).then_some(segments)
}

/// One piped part of a command: a read-only program on paths in the folder.
fn safe(words: &[String], directory: &str) -> bool {
    let Some((name, args)) = words.split_first() else { return false };
    let inside = |a: &String| {
        let value = if a.starts_with('-') { a.split_once('=').map_or("", |v| v.1) } else { a };
        !value.starts_with('~')
            && (!(value.starts_with('/') || value.contains("..")) || acp::within(value, directory))
    };
    if !args.iter().all(inside) {
        return false;
    }
    let has = |flags: &[&str]| args.iter().any(|a| flags.iter().any(|f| a.starts_with(f)));
    match program(name) {
        "cat" | "head" | "tail" | "ls" | "wc" | "nl" | "stat" | "file" | "pwd" | "echo"
        | "grep" => true,
        // `--pre` runs a program on each file.
        "rg" => !has(&["--pre"]),
        "find" => {
            !has(&["-delete", "-exec", "-execdir", "-ok", "-okdir", "-fprint", "-fprintf", "-fls"])
        }
        // Only printing lines: `sed -n '1,80p' notes.md`.
        "sed" => {
            let script = args.iter().find(|a| !a.starts_with('-'));
            args.iter().any(|a| a == "-n")
                && args.iter().filter(|a| a.starts_with('-')).all(|a| a == "-n")
                && script.is_some_and(|s| {
                    s.chars().all(|c| c.is_ascii_digit() || matches!(c, ',' | '$' | 'p' | ';'))
                })
        }
        _ => false,
    }
}

/// Applies a unified diff's hunks (no file headers) to `old`.
fn apply(old: &str, diff: &str) -> Option<String> {
    let lines: Vec<&str> = old.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut at = 0usize;
    for line in diff.lines() {
        if let Some(header) = line.strip_prefix("@@ -") {
            let start: usize = header.split([',', ' ']).next()?.parse().ok()?;
            let target = start.saturating_sub(1).max(at);
            out.extend(lines.get(at..target)?.iter().map(|l| l.to_string()));
            at = target;
        } else if let Some(added) = line.strip_prefix('+') {
            out.push(added.to_string());
        } else if line.starts_with('-') {
            at += 1;
        } else if let Some(kept) = line.strip_prefix(' ') {
            out.push(kept.to_string());
            at += 1;
        } else if line.is_empty() {
            // A blank context line whose leading space was trimmed.
            out.push(String::new());
            at += 1;
        }
    }
    out.extend(lines.get(at..)?.iter().map(|l| l.to_string()));
    let mut text = out.join("\n");
    if old.is_empty() || old.ends_with('\n') {
        text.push('\n');
    }
    Some(text)
}

/// Codex's own requests: approvals by the shared rules, everything else refused.
fn approval(id: Value, method: &str, params: Option<&Value>) {
    let p = params.cloned().unwrap_or(json!({}));
    let s = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let thread = s("threadId");
    let (directory, changes) = {
        let st = lock();
        match st.threads.get(&thread) {
            Some(t) => (t.directory.clone(), t.changes.get(&s("itemId")).cloned()),
            None => (String::new(), None),
        }
    };
    let call = match method {
        // Codex's parse (`commandActions`) only describes a command (`find … -delete` parses as
        // a search), so whether it runs is Mdow's own reading of the command.
        "item/commandExecution/requestApproval" if !directory.is_empty() => {
            let item = json!({"id": s("itemId"), "command": s("command"), "commandActions": p.get("commandActions")});
            let mut call = command_call(&item, &directory);
            if !reads_only(&s("command"), &directory) {
                call["kind"] = json!("execute");
                call["title"] = json!(format!("Run {}", s("command")));
            } else if call["kind"] == "execute" {
                call["kind"] = json!("search");
            }
            call
        }
        "item/fileChange/requestApproval" if !directory.is_empty() => {
            change_call(&s("itemId"), &changes.unwrap_or(json!([])), &directory)
        }
        _ => {
            write(
                &json!({"id": id, "error": {"code": -32601, "message": format!("Mdow doesn’t offer {method}.")}}),
            );
            return;
        }
    };
    let options = vec![
        json!({"optionId": "accept", "kind": "allow_once", "name": "Accept"}),
        json!({"optionId": "decline", "kind": "reject_once", "name": "Decline"}),
    ];
    acp::decide(Agent::Codex, &thread, json!({"rpc": id}), call, options);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunks_apply() {
        let old = "# Note\n\nTeh cat sat on the mat.\n";
        let diff = "@@ -2,2 +2,2 @@\n \n-Teh cat sat on the mat.\n+The cat sat on the mat.\n";
        assert_eq!(apply(old, diff).unwrap(), "# Note\n\nThe cat sat on the mat.\n");
        let two = "a\nb\nc\nd\ne\n";
        let diff = "@@ -1,2 +1,2 @@\n-a\n+A\n b\n@@ -4,2 +4,3 @@\n d\n-e\n+E\n+F\n";
        assert_eq!(apply(two, diff).unwrap(), "A\nb\nc\nd\nE\nF\n");
        assert_eq!(apply("", "@@ -0,0 +1,2 @@\n+hi\n+there\n").unwrap(), "hi\nthere\n");
    }

    #[test]
    fn commands_read_only_inside_the_folder() {
        let dir = std::env::temp_dir();
        let d = dir.to_string_lossy();
        let note = format!("{d}/note.md");
        let read = json!({"id": "x", "command": "cat note.md", "commandActions": [{"type": "read", "command": "cat note.md", "name": "note.md", "path": note}]});
        let c = command_call(&read, &d);
        assert_eq!(c["kind"], "read");
        assert_eq!(c["rawInput"]["path"], note);
        let outside = json!({"id": "x", "commandActions": [{"type": "read", "command": "cat /etc/hosts", "name": "hosts", "path": "/etc/hosts"}]});
        assert_eq!(command_call(&outside, &d)["kind"], "execute");
        let write =
            json!({"id": "x", "commandActions": [{"type": "unknown", "command": "rm note.md"}]});
        assert_eq!(command_call(&write, &d)["kind"], "execute");
        let search = json!({"id": "x", "commandActions": [{"type": "search", "command": "rg cat", "query": "cat", "path": null}]});
        assert_eq!(command_call(&search, &d)["title"], "Search for cat");
        let delete = json!([{"path": note, "kind": {"type": "delete"}, "diff": ""}]);
        assert_eq!(change_call("x", &delete, &d)["kind"], "delete");
        let outside = json!([{"path": "/etc/hosts", "kind": {"type": "update"}, "diff": "@@ -1 +1 @@\n-a\n+b\n"}]);
        assert_eq!(change_call("x", &outside, &d)["kind"], "other");
    }

    #[test]
    fn only_commands_that_read_the_folder_run() {
        let dir = std::env::temp_dir();
        let d = dir.to_string_lossy();
        for ok in [
            "/usr/bin/bash -lc 'cat notes.md'",
            "rg -n 'cat|dog' . | head -20",
            "sed -n '1,80p' notes.md",
            "find . -name '*.md'",
            "ls -la",
            "grep -rn \"Teh\" .",
        ] {
            assert!(reads_only(ok, &d), "{ok}");
        }
        for no in [
            "find . -name '*.md' -delete",
            "find . -exec rm {} ;",
            "bash -lc 'cat notes.md > copy.md'",
            "cat /etc/hosts",
            "cat ../../.zshrc",
            "cat ~/.zshrc",
            "rg --pre=sh x .",
            "sed -i 's/a/b/' notes.md",
            "sed -n 'w out' notes.md",
            "cat notes.md && rm notes.md",
            "echo \"$(rm -rf .)\"",
            "rm notes.md",
            "python3 -c 'print(1)'",
            "cat 'unterminated",
        ] {
            assert!(!reads_only(no, &d), "{no}");
        }
    }

    #[test]
    fn a_stopped_turns_end_doesnt_end_the_next() {
        let id = "mdow-test-thread";
        lock().threads.insert(id.into(), Thread { turn: Some("old".into()), ..Thread::default() });
        let (tx, rx) = channel();
        lock().threads.get_mut(id).unwrap().begin(tx);
        let ended =
            |turn: &str| json!({"threadId": id, "turn": {"id": turn, "status": "interrupted"}});
        notification("turn/started", &json!({"threadId": id, "turn": {"id": "new"}}));
        notification("turn/completed", &ended("old"));
        assert!(rx.try_recv().is_err());
        notification(
            "turn/completed",
            &json!({"threadId": id, "turn": {"id": "new", "status": "completed"}}),
        );
        assert_eq!(rx.try_recv().unwrap().unwrap()["stopReason"], "end_turn");
        lock().threads.remove(id);
    }
}
