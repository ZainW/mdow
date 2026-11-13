//! The companion's service, ported from the Swift app's `CompanionService.swift`.
//!
//! - The runtime is shared by every window: the private OpenCode server (started when a
//!   companion first opens, stopped when the app quits, the last window goes, or no pane has
//!   been used for ten minutes), its status and the live model list.
//! - Each window runs a session actor on its own thread. It owns the OpenCode session, the
//!   event stream and the turn in flight, and reports to the window's [`Companion`] through a
//!   channel and the window's waker. All network and process work happens there, never on the
//!   UI thread.
//!
//! [`Companion`]: super::Companion

use super::acp;
use super::agents::Agent;
use super::client::{Client, ModelRef, PermissionReply, StreamItem, UreqTransport, stream_events};
use super::events::{OpencodeEvent, Outcome, Translator, TurnFinish};
use super::logic;
use super::process::{self, Server};
use super::types::*;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

/// How long the server keeps running with every pane closed and nothing in flight.
const IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MODEL_WARMUP_ATTEMPTS: usize = 8;
const EVENT_STREAM_TIMEOUT: Duration = Duration::from_secs(10);

// MARK: Runtime

struct Runtime {
    /// None until the first check finishes (or after the server was stopped).
    status: Option<RuntimeStatus>,
    models: ModelState,
    server: Option<Arc<Server>>,
    client: Option<Client>,
    stopping: bool,
    visible: usize,
    turns: usize,
    idle_generation: u64,
    listeners: Vec<(u64, Sender<Cmd>, rune::Waker)>,
    /// The ACP agents' status and models, shared by every window.
    agents: HashMap<Agent, AcpRuntime>,
}

struct AcpRuntime {
    /// None while the agent starts.
    status: Option<RuntimeStatus>,
    models: ModelState,
    /// Each model's effort setting, as the agent last described it.
    efforts: HashMap<String, acp::Efforts>,
}

impl AcpRuntime {
    fn new() -> AcpRuntime {
        AcpRuntime { status: None, models: ModelState::loading(), efforts: HashMap::new() }
    }
}

static RUNTIME: LazyLock<Mutex<Runtime>> = LazyLock::new(|| {
    Mutex::new(Runtime {
        status: None,
        models: ModelState::loading(),
        server: None,
        client: None,
        stopping: false,
        visible: 0,
        turns: 0,
        idle_generation: 0,
        listeners: Vec::new(),
        agents: HashMap::new(),
    })
});
/// Bumped whenever the status or the models change, so every window refreshes.
static REVISION: AtomicU64 = AtomicU64::new(1);
/// Starting the server is done by one actor at a time.
static START: Mutex<()> = Mutex::new(());
static NEXT_LISTENER: AtomicU64 = AtomicU64::new(1);

fn runtime() -> std::sync::MutexGuard<'static, Runtime> {
    RUNTIME.lock().unwrap_or_else(|e| e.into_inner())
}

/// Wakes every window so it reads the new status and models.
fn changed() {
    REVISION.fetch_add(1, Ordering::SeqCst);
    let wakers: Vec<rune::Waker> = runtime().listeners.iter().map(|l| l.2.clone()).collect();
    for w in wakers {
        w();
    }
}

pub fn revision() -> u64 {
    REVISION.load(Ordering::SeqCst)
}

/// The agent's status and models.
pub fn snapshot(agent: Agent) -> (Option<RuntimeStatus>, ModelState) {
    let r = runtime();
    if !agent.is_acp() {
        return (r.status.clone(), r.models.clone());
    }
    r.agents
        .get(&agent)
        .map(|a| (a.status.clone(), a.models.clone()))
        .unwrap_or((None, ModelState::loading()))
}

fn with_agent(agent: Agent, f: impl FnOnce(&mut AcpRuntime)) {
    f(runtime().agents.entry(agent).or_insert_with(AcpRuntime::new));
    changed();
}

fn is_available() -> bool {
    runtime().status.as_ref().is_some_and(|s| s.availability == Availability::Available)
}

/// The client of the running server, without starting one.
fn current_client() -> Option<Client> {
    let r = runtime();
    match (&r.client, &r.server, &r.status) {
        (Some(c), Some(s), Some(st))
            if s.is_running() && st.availability == Availability::Available =>
        {
            Some(c.clone())
        }
        _ => None,
    }
}

fn ensure_client() -> Result<Client, String> {
    {
        let r = runtime();
        if let (Some(c), Some(s)) = (&r.client, &r.server)
            && s.is_running()
        {
            return Ok(c.clone());
        }
    }
    let _start = START.lock().unwrap_or_else(|e| e.into_inner());
    {
        let r = runtime();
        if let (Some(c), Some(s)) = (&r.client, &r.server)
            && s.is_running()
        {
            return Ok(c.clone());
        }
    }
    let (status, binary) = process::detect();
    let Some(binary) = binary else {
        return Err(status.detail.unwrap_or_else(|| "OpenCode is not available.".into()));
    };
    process::install_cleanup();
    let server = Server::start(&binary.path, server_exited)?;
    let client = Client::new(&server.url, &server.password, Arc::new(UreqTransport::new()));
    {
        let mut r = runtime();
        r.server = Some(Arc::new(server));
        r.client = Some(client.clone());
        r.stopping = false;
        r.status = Some(RuntimeStatus::new(Availability::Available, Some(binary.version), None));
    }
    schedule_idle_stop();
    changed();
    Ok(client)
}

/// Checks for OpenCode, starts its server and loads the live model list.
fn connect(directory: &str) {
    {
        let mut r = runtime();
        if r.status.as_ref().is_some_and(|s| s.availability != Availability::Available) {
            r.status = None;
        }
    }
    changed();
    if let Err(error) = ensure_client() {
        let detected = process::detect().0;
        let status = if detected.availability == Availability::Available {
            RuntimeStatus::new(Availability::Failed, detected.version, Some(error))
        } else {
            detected
        };
        {
            let mut r = runtime();
            r.models = ModelState::unavailable(status.detail.clone());
            r.status = Some(status);
        }
        changed();
        return;
    }
    load_models(directory);
}

fn server_exited(id: u64) {
    let listeners = {
        let mut r = runtime();
        if r.server.as_ref().map(|s| s.id) != Some(id) {
            return;
        }
        let unexpected = !r.stopping;
        r.server = None;
        r.client = None;
        r.stopping = false;
        r.status = (unexpected && r.visible > 0).then(|| {
            RuntimeStatus::new(
                Availability::Failed,
                None,
                Some("OpenCode stopped unexpectedly.".into()),
            )
        });
        r.models = ModelState::loading();
        r.listeners.iter().map(|l| l.1.clone()).collect::<Vec<_>>()
    };
    for l in listeners {
        let _ = l.send(Cmd::ServerStopped);
    }
    changed();
}

/// Stops the server right away, for idle time and the last window going away.
fn stop_now() {
    let (server, listeners) = {
        let mut r = runtime();
        r.idle_generation += 1;
        let Some(server) = r.server.clone() else { return };
        r.stopping = true;
        (server, r.listeners.iter().map(|l| l.1.clone()).collect::<Vec<_>>())
    };
    for l in listeners {
        let _ = l.send(Cmd::ServerStopped);
    }
    server.stop();
}

pub fn pane_visibility_changed(visible: bool) {
    {
        let mut r = runtime();
        r.visible = if visible { r.visible + 1 } else { r.visible.saturating_sub(1) };
    }
    schedule_idle_stop();
}

fn turn_count_changed(delta: isize) {
    {
        let mut r = runtime();
        r.turns = r.turns.saturating_add_signed(delta);
    }
    schedule_idle_stop();
}

fn schedule_idle_stop() {
    let generation = {
        let mut r = runtime();
        r.idle_generation += 1;
        if r.visible > 0 || r.turns > 0 || r.server.is_none() {
            return;
        }
        r.idle_generation
    };
    std::thread::spawn(move || {
        std::thread::sleep(IDLE_TIMEOUT);
        if runtime().idle_generation == generation {
            stop_now();
        }
    });
}

// MARK: Models

fn load_models(directory: &str) -> ModelState {
    let state = match ensure_client() {
        Err(e) => ModelState::unavailable(Some(e)),
        Ok(client) => fetch_models(&client, directory)
            .unwrap_or_else(|e| ModelState::unavailable(Some(e.message))),
    };
    runtime().models = state.clone();
    changed();
    state
}

fn fetch_models(client: &Client, directory: &str) -> Result<ModelState, super::client::HttpError> {
    // A fresh server loads the model catalog in the background, so the first answers can be
    // empty.
    let mut models = client.list_models(directory)?;
    let mut attempt = 1;
    while models.is_empty() && attempt < MODEL_WARMUP_ATTEMPTS {
        std::thread::sleep(Duration::from_millis(500));
        models = client.list_models(directory)?;
        attempt += 1;
    }
    let providers = client.list_providers(directory).unwrap_or_default();
    let options = logic::model_options(&models, &providers);
    let mut current = crate::prefs::get()
        .companion_model
        .filter(|saved| options.iter().any(|o| &o.value == saved));
    if current.is_none()
        && let Ok(Some(fallback)) = client.default_model(directory)
        && options.iter().any(|o| o.value == fallback.value())
    {
        current = Some(fallback.value());
    }
    let reason = options
        .is_empty()
        .then(|| "No models yet. Run `opencode auth login` to connect a provider.".to_string());
    Ok(ModelState {
        options,
        current,
        stale: false,
        unavailable_reason: reason,
        efforts: Vec::new(),
        effort: None,
    })
}

fn has_model(value: &str) -> bool {
    runtime().models.options.iter().any(|o| o.value == value)
}

/// Remembers the choice for new conversations; sessions switch their own session.
pub fn select_model(agent: Agent, value: &str) -> Result<(), String> {
    if agent.is_acp() {
        let known = runtime()
            .agents
            .get(&agent)
            .is_some_and(|a| a.models.options.iter().any(|o| o.value == value));
        if !known {
            return Err(format!("Model is not available: {value}"));
        }
        crate::prefs::update(|p| {
            p.companion_agent_models.insert(agent.raw().into(), value.to_string());
        });
        with_agent(agent, |a| {
            a.models.current = Some(value.to_string());
            let e = a.efforts.get(value);
            a.models.efforts = e.map(|e| e.values.clone()).unwrap_or_default();
            a.models.effort = e.map(|e| chosen_effort(agent, value).unwrap_or(e.current.clone()));
        });
        return Ok(());
    }
    if ModelRef::parse(value).is_none() || !has_model(value) {
        return Err(format!("Model is not available: {value}"));
    }
    crate::prefs::update(|p| p.companion_model = Some(value.to_string()));
    runtime().models.current = Some(value.to_string());
    changed();
    Ok(())
}

/// The default model in Settings: None is OpenCode's own default.
pub fn set_default_model(value: Option<String>) {
    crate::prefs::update(|p| p.companion_model = value.clone());
    let mut r = runtime();
    if let Some(v) = value
        && r.models.options.iter().any(|o| o.value == v)
    {
        r.models.current = Some(v);
    }
    drop(r);
    changed();
}

/// The saved model when OpenCode still offers it, otherwise OpenCode's own default.
fn session_model(directory: &str) -> Option<ModelRef> {
    let saved = crate::prefs::get().companion_model?;
    if runtime().models.options.is_empty() {
        load_models(directory);
    }
    if has_model(&saved) { ModelRef::parse(&saved) } else { None }
}

// MARK: Session actor

/// What a window asks its session actor to do, and what the actor's threads tell it.
pub enum Cmd {
    Connect {
        agent: Agent,
        directory: String,
    },
    Send {
        agent: Agent,
        text: String,
        active_path: Option<String>,
        tags: Vec<ContextTag>,
        directory: String,
        epoch: u64,
    },
    Stop,
    NewChat,
    Review {
        permission_id: String,
        tool_call_id: String,
        decision: Decision,
    },
    SwitchModel(String),
    /// The effort for the current ACP model.
    SwitchEffort(String),
    /// From the ACP agent of this window's session.
    Acp(acp::Event),
    /// An ACP prompt finished: its stop reason, or why it failed.
    AcpDone {
        turn: u64,
        result: Result<Option<String>, String>,
    },
    Stream {
        generation: u64,
        item: StreamItem,
    },
    StreamFailed {
        generation: u64,
        error: String,
    },
    ServerStopped,
    Shutdown,
}

/// What the actor tells the window.
#[derive(Debug)]
pub enum Msg {
    Update(Update),
    /// Shown under the conversation.
    Error(String),
    /// Sending a review failed: show its buttons again.
    Restore {
        tool_call_id: String,
        permission_id: String,
    },
}

/// The window's end of its actor.
pub struct Handle {
    pub id: u64,
    pub tx: Sender<Cmd>,
    pub rx: Receiver<Msg>,
    /// Bumped by the window when the user stops or starts over, so a turn still starting
    /// knows nothing waits for it.
    pub epoch: Arc<AtomicU64>,
}

impl Handle {
    pub fn send(&self, cmd: Cmd) {
        let _ = self.tx.send(cmd);
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Shutdown);
        let last = {
            let mut r = runtime();
            r.listeners.retain(|l| l.0 != self.id);
            r.listeners.is_empty()
        };
        if last {
            stop_now();
            acp::stop_all();
        }
    }
}

/// Starts a window's actor.
pub fn spawn(waker: rune::Waker) -> Handle {
    let (tx, rx) = mpsc::channel::<Cmd>();
    let (out, msgs) = mpsc::channel::<Msg>();
    let id = NEXT_LISTENER.fetch_add(1, Ordering::Relaxed);
    let epoch = Arc::new(AtomicU64::new(0));
    runtime().listeners.push((id, tx.clone(), waker.clone()));
    let actor = Actor {
        me: tx.clone(),
        out,
        waker,
        epoch: epoch.clone(),
        active: None,
        turn: None,
        pending: HashSet::new(),
        next_generation: 1,
        acp: None,
        acp_turns: 0,
    };
    std::thread::Builder::new()
        .name("mdow-companion".into())
        .spawn(move || actor.run(rx))
        .expect("companion thread");
    Handle { id, tx, rx: msgs, epoch }
}

struct ActiveSession {
    id: String,
    directory: String,
    translator: Translator,
    generation: u64,
    cancel: Arc<AtomicBool>,
}

struct Turn {
    message_id: String,
    /// When the prompt was sent (ms since 1970), to tell this turn's finish from an earlier
    /// run's.
    prompted_at: f64,
    /// Set once OpenCode starts the run for this prompt. Until then, events still arriving from
    /// a run the user stopped are ignored, so a late "interrupted" can't end this turn.
    started: bool,
}

struct Actor {
    me: Sender<Cmd>,
    out: Sender<Msg>,
    waker: rune::Waker,
    epoch: Arc<AtomicU64>,
    active: Option<ActiveSession>,
    turn: Option<Turn>,
    pending: HashSet<String>,
    next_generation: u64,
    /// The window's session with an ACP agent, when it runs one.
    acp: Option<AcpSession>,
    /// Counts ACP prompts, so a stopped one's late reply is ignored.
    acp_turns: u64,
}

struct AcpSession {
    agent: Agent,
    id: String,
    directory: String,
    translator: acp::Translator,
    /// Edits waiting for review: Mdow's id → the request's id, its options and its tool call.
    reviews: HashMap<String, (Value, Vec<Value>, String)>,
    /// The session's settings as last reported, for switching models.
    options: Vec<Value>,
    /// The instructions go with the first prompt.
    instructed: bool,
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

impl Actor {
    fn run(mut self, rx: Receiver<Cmd>) {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                Cmd::Connect { agent, directory } if agent.is_acp() => {
                    self.acp_connect(agent, &directory)
                }
                Cmd::Connect { directory, .. } => connect(&directory),
                Cmd::Send { agent, text, active_path, tags, directory, epoch }
                    if agent.is_acp() =>
                {
                    self.acp_send(agent, &text, active_path.as_deref(), &tags, &directory, epoch)
                }
                Cmd::Send { text, active_path, tags, directory, epoch, .. } => {
                    self.send(&text, active_path.as_deref(), &tags, &directory, epoch)
                }
                Cmd::Stop if self.acp.is_some() => self.acp_cancel(),
                Cmd::Stop => self.cancel(),
                Cmd::NewChat => {
                    if self.turn.is_some() {
                        if self.acp.is_some() { self.acp_cancel() } else { self.cancel() }
                    }
                    self.close_session();
                    self.close_acp();
                }
                Cmd::Review { permission_id, decision, .. }
                    if self
                        .acp
                        .as_ref()
                        .is_some_and(|a| a.reviews.contains_key(&permission_id)) =>
                {
                    self.acp_review(&permission_id, decision)
                }
                Cmd::Review { permission_id, tool_call_id, decision } => {
                    if let Err(e) = self.reply_permission(&permission_id, decision) {
                        self.emit(Msg::Restore { tool_call_id, permission_id });
                        self.emit(Msg::Error(e));
                    }
                }
                Cmd::SwitchModel(value) if self.acp.is_some() => self.acp_switch_model(&value),
                Cmd::SwitchEffort(value) => self.acp_switch_effort(&value),
                Cmd::Acp(event) => self.acp_event(event),
                Cmd::AcpDone { turn, result } => self.acp_done(turn, result),
                Cmd::SwitchModel(value) => {
                    if let (Some(active), Some(model)) = (&self.active, ModelRef::parse(&value))
                        && let Ok(client) = ensure_client()
                        && let Err(e) = client.switch_model(&active.id, &model)
                    {
                        self.emit(Msg::Error(e.message));
                    }
                }
                Cmd::Stream { generation, item } => {
                    if self.active.as_ref().map(|a| a.generation) != Some(generation) {
                        continue;
                    }
                    match item {
                        StreamItem::Opened { reconnected: true } => self.reconcile_turn(),
                        StreamItem::Opened { .. } => {}
                        StreamItem::Event(event) => self.handle_event(&event),
                    }
                }
                Cmd::StreamFailed { generation, error } => {
                    if self.active.as_ref().map(|a| a.generation) == Some(generation)
                        && self.turn.is_some()
                    {
                        self.emit(Msg::Update(Update::Error(error)));
                        self.set_turn(None);
                    }
                }
                Cmd::ServerStopped => {
                    self.close_session();
                    if self.turn.is_some() {
                        self.emit(Msg::Update(Update::Error(
                            "OpenCode stopped unexpectedly. Send again to restart it.".into(),
                        )));
                        self.set_turn(None);
                    }
                }
                Cmd::Shutdown => {
                    if self.turn.is_some() {
                        if self.acp.is_some() { self.acp_cancel() } else { self.cancel() }
                    }
                    self.close_session();
                    self.close_acp();
                    return;
                }
            }
        }
        self.close_session();
        self.close_acp();
    }

    fn emit(&self, msg: Msg) {
        if self.out.send(msg).is_ok() {
            (self.waker)();
        }
    }

    fn set_turn(&mut self, turn: Option<Turn>) {
        match (self.turn.is_some(), turn.is_some()) {
            (false, true) => turn_count_changed(1),
            (true, false) => turn_count_changed(-1),
            _ => {}
        }
        self.turn = turn;
    }

    fn send(
        &mut self,
        text: &str,
        active_path: Option<&str>,
        tags: &[ContextTag],
        directory: &str,
        epoch: u64,
    ) {
        if self.turn.is_some() {
            self.emit(Msg::Update(Update::Warning(
                "Wait for the current response or stop it first.".into(),
            )));
            return;
        }
        let message_id = new_id("turn");
        self.set_turn(Some(Turn {
            message_id: message_id.clone(),
            prompted_at: now_ms(),
            started: false,
        }));
        if let Err(e) = self.prompt(text, active_path, tags, directory, epoch)
            && self.live(epoch)
            && self.turn.as_ref().is_some_and(|t| t.message_id == message_id)
        {
            self.emit(Msg::Update(Update::Error(e)));
            self.set_turn(None);
        }
    }

    /// Nothing stopped this turn since the window sent it.
    fn live(&self, epoch: u64) -> bool {
        self.epoch.load(Ordering::SeqCst) == epoch
    }

    fn prompt(
        &mut self,
        text: &str,
        active_path: Option<&str>,
        tags: &[ContextTag],
        directory: &str,
        epoch: u64,
    ) -> Result<(), String> {
        let client = ensure_client()?;
        // Stopped while OpenCode was starting: nothing is waiting for a response any more.
        if !self.live(epoch) {
            return Ok(());
        }
        self.ensure_session(&client, directory)?;
        if !self.live(epoch) {
            return Ok(());
        }
        let session = self.active.as_mut().expect("session");
        session.translator.begin_turn();
        if let Some(turn) = &mut self.turn {
            turn.prompted_at = now_ms();
        }
        let id = session.id.clone();
        client
            .prompt(&id, &logic::compose_prompt(text, active_path, tags, directory))
            .map_err(|e| e.message)
    }

    fn ensure_session(&mut self, client: &Client, directory: &str) -> Result<(), String> {
        if self.active.as_ref().is_some_and(|a| a.directory == directory) {
            return Ok(());
        }
        self.close_session();
        let model = session_model(directory);
        let id = client
            .create_session(directory, model.as_ref(), &logic::session_permissions())
            .map_err(|e| e.message)?;
        // Older 2.x servers may lack session instructions; the permission rules still apply.
        let _ = client.set_instructions(&id, "mdow", logic::INSTRUCTIONS);

        let generation = self.next_generation;
        self.next_generation += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let (open_tx, open_rx) = mpsc::channel::<Result<(), String>>();
        {
            let client = client.clone();
            let directory = directory.to_string();
            let cancel = cancel.clone();
            let me = self.me.clone();
            std::thread::Builder::new()
                .name("mdow-companion-events".into())
                .spawn(move || {
                    let mut first = true;
                    let result = stream_events(&client, &directory, &cancel, |item| {
                        if first && matches!(item, StreamItem::Opened { .. }) {
                            first = false;
                            let _ = open_tx.send(Ok(()));
                        }
                        let _ = me.send(Cmd::Stream { generation, item });
                    });
                    if let Err(e) = result {
                        let _ = open_tx.send(Err(e.message.clone()));
                        let _ = me.send(Cmd::StreamFailed { generation, error: e.message });
                    }
                })
                .map_err(|e| e.to_string())?;
        }
        self.active = Some(ActiveSession {
            id: id.clone(),
            directory: directory.into(),
            translator: Translator::new(&id, directory),
            generation,
            cancel,
        });

        // Prompting before the stream is open would lose the start of the response, so wait.
        let opened = match open_rx.recv_timeout(EVENT_STREAM_TIMEOUT) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("OpenCode did not open its event stream.".into()),
        };
        if opened.is_err() {
            self.close_session();
        }
        opened
    }

    fn close_session(&mut self) {
        if let Some(a) = self.active.take() {
            a.cancel.store(true, Ordering::SeqCst);
        }
        self.pending.clear();
    }

    /// Events sent while the stream was reconnecting are not replayed. If the turn finished in
    /// that gap, read how it ended from the session instead of waiting forever.
    fn reconcile_turn(&mut self) {
        let (Some(turn), Some(active), Some(client)) = (&self.turn, &self.active, current_client())
        else {
            return;
        };
        let Ok(info) = client.get_session(&active.id) else { return };
        let (Some(outcome), Some(idle)) = (info.outcome, info.idle) else { return };
        if idle < turn.prompted_at {
            return;
        }
        if let Some(t) = &mut self.turn {
            t.started = true;
        }
        self.finish_turn(TurnFinish::new(Outcome::parse(&outcome)));
    }

    fn handle_event(&mut self, event: &OpencodeEvent) {
        let Some(active) = &mut self.active else { return };
        let session_id = active.id.clone();
        let result = active.translator.translate(event);
        if result.started
            && let Some(t) = &mut self.turn
        {
            t.started = true;
        }
        let live = self.turn.as_ref().is_some_and(|t| t.started);
        if live {
            for update in result.updates {
                if let Update::Change {
                    permission_id: Some(p),
                    status: ChangeStatus::Pending,
                    ..
                } = &update
                {
                    self.pending.insert(p.clone());
                }
                self.emit(Msg::Update(update));
            }
        }
        if let Some((permission_id, reason)) = result.auto_reject
            && let Some(client) = current_client()
        {
            let _ = client.reply_permission(
                &session_id,
                &permission_id,
                PermissionReply::Reject,
                Some(&reason),
            );
        }
        if let Some(finish) = result.finish
            && live
        {
            self.finish_turn(finish);
        }
    }

    fn finish_turn(&mut self, finish: TurnFinish) {
        let Some(turn) = &self.turn else { return };
        let id = turn.message_id.clone();
        self.emit(Msg::Update(match finish.outcome {
            Outcome::Failed => {
                Update::Error(finish.error.unwrap_or_else(|| "OpenCode failed.".into()))
            }
            Outcome::Interrupted => Update::Cancelled(id),
            Outcome::Succeeded => Update::Done(id),
        }));
        self.set_turn(None);
        self.pending.clear();
    }

    fn reply_permission(&mut self, permission_id: &str, decision: Decision) -> Result<(), String> {
        let stale = || "This change is no longer waiting for review.".to_string();
        let (Some(active), Some(client)) = (&self.active, current_client()) else {
            return Err(stale());
        };
        if !is_available() || !self.pending.remove(permission_id) {
            return Err(stale());
        }
        // `always` would save the rule in OpenCode's global config, so Mdow only answers once.
        let (reply, message) = match decision {
            Decision::Approve => (PermissionReply::Once, None),
            Decision::Reject => (PermissionReply::Reject, Some("The user rejected this change.")),
        };
        client.reply_permission(&active.id, permission_id, reply, message).map_err(|e| e.message)
    }

    fn cancel(&mut self) {
        let turn = self.turn.take();
        if turn.is_some() {
            turn_count_changed(-1);
        }
        if let (Some(active), Some(client)) = (&self.active, current_client()) {
            let _ = client.interrupt(&active.id);
            for id in self.pending.drain() {
                let _ = client.reply_permission(
                    &active.id,
                    &id,
                    PermissionReply::Reject,
                    Some("The user stopped the response."),
                );
            }
        }
        self.pending.clear();
        if let Some(turn) = turn {
            self.emit(Msg::Update(Update::Cancelled(turn.message_id)));
        }
    }
}

// MARK: ACP agents

/// "Anthropic", "OpenAI"…: how a model list groups an agent's models.
fn maker(id: &str) -> &'static str {
    let id = id.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| id.contains(w));
    if id.starts_with("auto") || id.starts_with("composer") {
        "Cursor"
    } else if has(&["claude", "anthropic", "opus", "sonnet", "haiku", "fable"]) {
        "Anthropic"
    } else if has(&["gpt", "openai", "codex", "o3", "o4"]) {
        "OpenAI"
    } else if has(&["gemini", "google"]) {
        "Google"
    } else if has(&["grok", "xai"]) {
        "xAI"
    } else {
        "Other"
    }
}

/// The effort picked for an agent's model, if any.
fn chosen_effort(agent: Agent, model: &str) -> Option<String> {
    crate::prefs::get().companion_efforts.get(&format!("{}/{model}", agent.raw())).cloned()
}

/// Notes the models a session reply lists.
fn absorb_models(agent: Agent, reply: &Value) {
    let listed = acp::models_in(reply);
    with_agent(agent, |a| {
        if !listed.is_empty() {
            a.models.options = listed
                .into_iter()
                .map(|(value, name)| {
                    let maker = maker(&value).to_string();
                    ModelOption { value, name, provider_id: maker.clone(), provider_name: maker }
                })
                .collect();
        }
        a.models.stale = false;
        a.models.unavailable_reason =
            a.models.options.is_empty().then(|| "This agent lists no models.".to_string());
    });
}

/// Notes the model a session is on and its effort setting.
fn absorb_options(agent: Agent, options: &[Value]) {
    let Some((model, e)) = acp::efforts(options) else { return };
    with_agent(agent, |a| {
        a.models.current = Some(model.clone());
        a.models.efforts = e.values.clone();
        a.models.effort = (!e.current.is_empty()).then(|| e.current.clone());
        a.efforts.insert(model, e);
    });
}

/// Sets the effort for the current model of an agent, for its next sessions.
pub fn select_effort(agent: Agent, value: &str) {
    let Some(model) = runtime().agents.get(&agent).and_then(|a| a.models.current.clone()) else {
        return;
    };
    crate::prefs::update(|p| {
        p.companion_efforts.insert(format!("{}/{model}", agent.raw()), value.to_string());
    });
    with_agent(agent, |a| a.models.effort = Some(value.to_string()));
}

impl Actor {
    /// Starts the agent and a session in `directory`, which lists the models.
    fn acp_connect(&mut self, agent: Agent, directory: &str) {
        if self.acp.as_ref().is_some_and(|a| a.agent == agent && a.directory == directory) {
            return;
        }
        if let Err(e) = self.acp_session(agent, directory) {
            let missing = agent.command().is_err();
            with_agent(agent, |a| {
                let availability =
                    if missing { Availability::Missing } else { Availability::Failed };
                a.status = Some(RuntimeStatus::new(availability, None, Some(e.clone())));
                a.models = ModelState::unavailable(Some(e));
            });
        }
    }

    fn acp_session(&mut self, agent: Agent, directory: &str) -> Result<(), String> {
        if self.acp.as_ref().is_some_and(|a| a.agent == agent && a.directory == directory) {
            return Ok(());
        }
        self.close_acp();
        with_agent(agent, |a| {
            if a.status.as_ref().is_some_and(|s| s.availability != Availability::Available) {
                a.status = None;
            }
        });
        agent.command()?;
        let client = acp::client(agent);
        let reply = client.request("session/new", acp::session_params(agent, directory))?;
        let id = reply
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{} didn’t start a chat.", agent.process()))?
            .to_string();
        with_agent(agent, |a| {
            a.status = Some(RuntimeStatus::new(Availability::Available, None, None));
        });
        absorb_models(agent, &reply);
        let abandon = |e: String| {
            client.notify("session/cancel", json!({"sessionId": id}));
            e
        };
        // The mode that keeps the agent to reading and reviewed edits, where it has modes.
        if let Some(mode) = agent.mode() {
            let modes: Vec<&str> = reply
                .get("modes")
                .and_then(|m| m.get("availableModes"))
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|m| m.get("id").and_then(Value::as_str)).collect())
                .unwrap_or_default();
            if modes.is_empty() || modes.contains(&mode) {
                client
                    .request("session/set_mode", json!({"sessionId": id, "modeId": mode}))
                    .map_err(|e| {
                        abandon(format!("Couldn’t put {} in its {mode} mode: {e}", agent.name()))
                    })?;
            } else {
                return Err(abandon(format!(
                    "{} has no {mode} mode, so Mdow won’t run it.",
                    agent.name()
                )));
            }
        }
        let mut options = acp::options(&reply);
        let saved = crate::prefs::get().companion_agent_models.get(agent.raw()).cloned();
        let current = acp::efforts(&options).map(|(m, _)| m);
        if let Some(model) = saved
            && current.as_deref() != Some(model.as_str())
            && acp::models_in(&reply).iter().any(|(m, _)| *m == model)
            && let Ok(o) = acp::set_model(agent, &id, &options, &model)
        {
            options = o;
        }
        if let Some((model, e)) = acp::efforts(&options)
            && let Some(chosen) = chosen_effort(agent, &model)
            && chosen != e.current
            && e.values.iter().any(|v| v.value == chosen)
            && let Ok(o) = acp::set_option(agent, &id, &e.config_id, &chosen)
        {
            options = o;
        }
        absorb_options(agent, &options);
        acp::route(&id, self.me.clone(), self.waker.clone());
        self.acp = Some(AcpSession {
            agent,
            id,
            directory: directory.to_string(),
            translator: acp::Translator::new(directory),
            reviews: HashMap::new(),
            options,
            // Claude and Codex have them in their system prompt.
            instructed: agent.native(),
        });
        Ok(())
    }

    fn close_acp(&mut self) {
        if let Some(a) = self.acp.take() {
            let client = acp::client(a.agent);
            for (id, options, _) in a.reviews.into_values() {
                client.respond(id, &options, false);
            }
            acp::close(a.agent, &a.id);
            acp::unroute(&a.id);
        }
    }

    fn acp_send(
        &mut self,
        agent: Agent,
        text: &str,
        active_path: Option<&str>,
        tags: &[ContextTag],
        directory: &str,
        epoch: u64,
    ) {
        if self.turn.is_some() {
            self.emit(Msg::Update(Update::Warning(
                "Wait for the current response or stop it first.".into(),
            )));
            return;
        }
        let message_id = new_id("turn");
        self.set_turn(Some(Turn { message_id, prompted_at: now_ms(), started: true }));
        if let Err(e) = self.acp_session(agent, directory) {
            if self.live(epoch) {
                self.emit(Msg::Update(Update::Error(e)));
            }
            self.set_turn(None);
            return;
        }
        if !self.live(epoch) {
            self.set_turn(None);
            return;
        }
        let session = self.acp.as_mut().expect("session");
        let mut prompt = super::logic::compose_prompt(text, active_path, tags, directory);
        if !session.instructed {
            session.instructed = true;
            prompt = format!(
                "<mdow-instructions>\n{}\n</mdow-instructions>\n\n{prompt}",
                super::logic::instructions(agent)
            );
        }
        self.acp_turns += 1;
        let (turn, id, me, waker) =
            (self.acp_turns, session.id.clone(), self.me.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let result = acp::client(agent)
                .request(
                    "session/prompt",
                    json!({"sessionId": id, "prompt": [{"type": "text", "text": prompt}]}),
                )
                .map(|r| r.get("stopReason").and_then(Value::as_str).map(str::to_string));
            if me.send(Cmd::AcpDone { turn, result }).is_ok() {
                waker();
            }
        });
    }

    fn acp_event(&mut self, event: acp::Event) {
        let Some(session) = &mut self.acp else { return };
        match event {
            acp::Event::Update(update) => {
                if update.get("sessionUpdate").and_then(Value::as_str)
                    == Some("config_option_update")
                {
                    let options = acp::options(&update);
                    session.options = options.clone();
                    absorb_options(session.agent, &options);
                    return;
                }
                let updates = session.translator.translate(&update);
                if self.turn.is_some() {
                    for u in updates {
                        self.emit(Msg::Update(u));
                    }
                }
            }
            acp::Event::Permission { id, call, options } => {
                if self.turn.is_none() {
                    acp::client(session.agent).respond(id, &options, false);
                    return;
                }
                let permission_id = new_id("acp-review");
                let update = session.translator.permission(&permission_id, &call);
                if let Update::Change { tool_call_id, .. } = &update {
                    session.reviews.insert(permission_id, (id, options, tool_call_id.clone()));
                }
                self.emit(Msg::Update(update));
            }
        }
    }

    fn acp_review(&mut self, permission_id: &str, decision: Decision) {
        let Some(session) = &mut self.acp else { return };
        let Some((id, options, tool_call_id)) = session.reviews.remove(permission_id) else {
            return;
        };
        let allow = decision == Decision::Approve;
        acp::client(session.agent).respond(id, &options, allow);
        if !allow {
            session.translator.declined(&tool_call_id);
            self.emit(Msg::Update(Update::change(tool_call_id, ChangeStatus::Rejected)));
        }
    }

    fn acp_done(&mut self, turn: u64, result: Result<Option<String>, String>) {
        if turn != self.acp_turns {
            return;
        }
        let Some(t) = &self.turn else { return };
        let id = t.message_id.clone();
        if let Some(session) = &mut self.acp {
            let client = acp::client(session.agent);
            for (rpc, options, _) in std::mem::take(&mut session.reviews).into_values() {
                client.respond(rpc, &options, false);
            }
        }
        let alive = self.acp.as_ref().is_none_or(|a| acp::alive(a.agent, &a.id));
        let update = match result {
            Ok(stop) => match stop.as_deref() {
                Some("cancelled") => Update::Cancelled(id),
                Some("refusal") => Update::Error("The model declined to answer.".into()),
                Some("max_tokens" | "max_turn_requests") => {
                    self.emit(Msg::Update(Update::Warning(
                        "The answer was cut off at the model’s limit.".into(),
                    )));
                    Update::Done(id)
                }
                _ => Update::Done(id),
            },
            Err(e) => {
                // The agent stopped: its sessions went with it.
                if !alive {
                    self.close_acp();
                }
                Update::Error(e)
            }
        };
        self.emit(Msg::Update(update));
        self.set_turn(None);
    }

    fn acp_cancel(&mut self) {
        let turn = self.turn.take();
        if turn.is_some() {
            turn_count_changed(-1);
        }
        // The prompt's reply ("cancelled") ends nothing now.
        self.acp_turns += 1;
        if let Some(session) = &mut self.acp {
            let client = acp::client(session.agent);
            for (id, options, _) in std::mem::take(&mut session.reviews).into_values() {
                client.respond(id, &options, false);
            }
            client.notify("session/cancel", json!({"sessionId": session.id}));
        }
        if let Some(turn) = turn {
            self.emit(Msg::Update(Update::Cancelled(turn.message_id)));
        }
    }

    fn acp_switch_model(&mut self, value: &str) {
        let Some(session) = &mut self.acp else { return };
        match acp::set_model(session.agent, &session.id, &session.options, value) {
            Ok(options) => {
                session.options = options.clone();
                absorb_options(session.agent, &options);
                // The new model's own effort, when one was picked for it.
                if let Some((model, e)) = acp::efforts(&options)
                    && let Some(chosen) = chosen_effort(session.agent, &model)
                    && chosen != e.current
                    && e.values.iter().any(|v| v.value == chosen)
                {
                    self.acp_switch_effort(&chosen);
                }
            }
            Err(e) => self.emit(Msg::Error(e)),
        }
    }

    fn acp_switch_effort(&mut self, value: &str) {
        let Some(session) = &mut self.acp else { return };
        let Some((_, e)) = acp::efforts(&session.options) else { return };
        if e.config_id.is_empty() {
            return;
        }
        match acp::set_option(session.agent, &session.id, &e.config_id, value) {
            Ok(options) => {
                session.options = options.clone();
                absorb_options(session.agent, &options);
            }
            Err(e) => self.emit(Msg::Error(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs a real turn against the installed `opencode`: start the server, ask for an edit,
    /// accept it, and stop the server when the window goes.
    /// `cargo test live_turn -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_turn() {
        let dir = std::env::temp_dir().join(format!("mdow-companion-live-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let notes = dir.join("notes.md");
        std::fs::write(&notes, "# Notes\n\nTeh quick brown fox.\n").unwrap();
        let directory = dir.to_string_lossy().into_owned();

        let handle = spawn(Arc::new(|| {}));
        handle.send(Cmd::Connect { agent: Agent::OpenCode, directory: directory.clone() });
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        loop {
            let (status, models) = snapshot(Agent::OpenCode);
            if status.as_ref().is_some_and(|s| s.availability != Availability::Available) {
                panic!("OpenCode unavailable: {status:?}");
            }
            if !models.stale {
                eprintln!("models: {} options, current {:?}", models.options.len(), models.current);
                break;
            }
            assert!(std::time::Instant::now() < deadline, "models never loaded");
            std::thread::sleep(Duration::from_millis(200));
        }

        handle.send(Cmd::Send {
            agent: Agent::OpenCode,
            text: "Fix the typo in notes.md. Change nothing else.".into(),
            active_path: Some(notes.to_string_lossy().into_owned()),
            tags: Vec::new(),
            directory,
            epoch: 0,
        });
        let mut conversation = super::super::conversation::Conversation::default();
        conversation.append_user_message("Fix the typo");
        conversation.begin_request();
        let deadline = std::time::Instant::now() + Duration::from_secs(180);
        let mut done = false;
        while !done {
            let msg = handle
                .rx
                .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .expect("turn timed out");
            eprintln!("{msg:?}");
            match msg {
                Msg::Update(u) => {
                    if let Update::Change {
                        tool_call_id,
                        permission_id: Some(p),
                        status: ChangeStatus::Pending,
                        ..
                    } = &u
                    {
                        handle.send(Cmd::Review {
                            permission_id: p.clone(),
                            tool_call_id: tool_call_id.clone(),
                            decision: Decision::Approve,
                        });
                    }
                    done = matches!(u, Update::Done(_) | Update::Error(_) | Update::Cancelled(_));
                    if let Update::Error(e) = &u {
                        panic!("turn failed: {e}");
                    }
                    conversation.apply(u);
                }
                Msg::Error(e) => panic!("{e}"),
                Msg::Restore { .. } => panic!("review was refused"),
            }
        }
        let text = std::fs::read_to_string(&notes).unwrap();
        assert!(text.contains("The quick brown fox."), "{text}");
        let m = conversation.messages.last().unwrap();
        assert!(
            m.parts
                .iter()
                .any(|p| matches!(p, Part::Change(c) if c.status == ChangeStatus::Applied))
        );
        eprintln!("reply: {}", m.content);

        let server = runtime().server.clone().expect("server");
        drop(handle);
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while server.is_running() {
            assert!(std::time::Instant::now() < deadline, "server still running");
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Runs a real turn against an installed ACP agent (`MDOW_LIVE_AGENT=codex`, `claude` or
    /// `cursor`; Codex by default): ask for an edit, accept it, and check the file.
    /// `cargo test live_acp_turn -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_acp_turn() {
        let agent = std::env::var("MDOW_LIVE_AGENT")
            .ok()
            .and_then(|a| Agent::from_raw(&a))
            .unwrap_or(Agent::Codex);
        let dir = std::env::temp_dir().join(format!("mdow-acp-live-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let notes = dir.join("notes.md");
        std::fs::write(&notes, "# Notes\n\nTeh quick brown fox.\n").unwrap();
        let directory = dir.to_string_lossy().into_owned();

        let handle = spawn(Arc::new(|| {}));
        handle.send(Cmd::Connect { agent, directory: directory.clone() });
        handle.send(Cmd::Send {
            agent,
            text: "Fix the typo in notes.md. Change nothing else.".into(),
            active_path: Some(notes.to_string_lossy().into_owned()),
            tags: Vec::new(),
            directory,
            epoch: 0,
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(300);
        loop {
            let msg = handle
                .rx
                .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .expect("turn timed out");
            eprintln!("{msg:?}");
            match msg {
                Msg::Update(Update::Change {
                    tool_call_id,
                    permission_id: Some(p),
                    status: ChangeStatus::Pending,
                    ..
                }) if agent.edits() => handle.send(Cmd::Review {
                    permission_id: p,
                    tool_call_id,
                    decision: Decision::Approve,
                }),
                Msg::Update(Update::Error(e)) | Msg::Error(e) => panic!("turn failed: {e}"),
                Msg::Update(Update::Done(_) | Update::Cancelled(_)) => break,
                _ => {}
            }
        }
        eprintln!("models: {:?}", snapshot(agent).1.options.len());
        if agent.edits() {
            let text = std::fs::read_to_string(&notes).unwrap();
            assert!(text.contains("The quick brown fox."), "{text}");
        }
        drop(handle);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
