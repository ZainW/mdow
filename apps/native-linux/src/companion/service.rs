//! The companion's service: `Runtime` is the private OpenCode server, shared by every window,
//! started when a panel first opens and stopped when the app quits or no panel has been used for
//! a while. `Session` is one window's conversation with it.

use super::client::{Client, HttpError, ModelRef, PermissionReply, StreamCancel, StreamItem};
use super::conversation::Conversation;
use super::events::{Event, Outcome, TurnFinish, TurnTranslator};
use super::logic;
use super::process::{self, Server};
use super::types::*;
use crate::prefs::Prefs;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long the server keeps running with every panel closed and nothing in flight.
const IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MODEL_WARMUP_ATTEMPTS: u32 = 8;
const EVENT_STREAM_TIMEOUT: Duration = Duration::from_secs(10);

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    gio::spawn_blocking(work).await.map_err(|_| "OpenCode's request failed.".to_string())
}

/// Runs a client call on a worker thread.
async fn call<T: Send + 'static>(
    client: &Client,
    work: impl FnOnce(&Client) -> Result<T, HttpError> + Send + 'static,
) -> Result<T, String> {
    let client = client.clone();
    blocking(move || work(&client).map_err(|err| err.message)).await?
}

type Listener = Box<dyn Fn() -> bool>;

thread_local! {
    static RUNTIME: Rc<Runtime> = Rc::new(Runtime::new());
}

pub struct Runtime {
    /// `None` until the first check finishes (or after the server was stopped).
    status: RefCell<Option<RuntimeStatus>>,
    model_state: RefCell<ModelState>,
    server: RefCell<Option<Arc<Server>>>,
    client: RefCell<Option<Client>>,
    starting: RefCell<Option<async_channel::Receiver<()>>>,
    start_error: RefCell<Option<String>>,
    sessions: RefCell<Vec<Weak<Session>>>,
    listeners: RefCell<Vec<Listener>>,
    visible_panels: Cell<i32>,
    active_turns: Cell<i32>,
    idle_timer: RefCell<Option<glib::SourceId>>,
    stopping: Cell<bool>,
    prefs: RefCell<Option<(Rc<RefCell<Prefs>>, PathBuf)>>,
}

impl Runtime {
    fn new() -> Runtime {
        Runtime {
            status: RefCell::new(None),
            model_state: RefCell::new(ModelState::loading()),
            server: RefCell::new(None),
            client: RefCell::new(None),
            starting: RefCell::new(None),
            start_error: RefCell::new(None),
            sessions: RefCell::new(Vec::new()),
            listeners: RefCell::new(Vec::new()),
            visible_panels: Cell::new(0),
            active_turns: Cell::new(0),
            idle_timer: RefCell::new(None),
            stopping: Cell::new(false),
            prefs: RefCell::new(None),
        }
    }

    pub fn shared() -> Rc<Runtime> {
        RUNTIME.with(Rc::clone)
    }

    pub fn set_prefs(&self, prefs: Rc<RefCell<Prefs>>, path: PathBuf) {
        self.prefs.replace(Some((prefs, path)));
    }

    pub fn status(&self) -> Option<RuntimeStatus> {
        self.status.borrow().clone()
    }

    pub fn model_state(&self) -> ModelState {
        self.model_state.borrow().clone()
    }

    pub fn is_available(&self) -> bool {
        self.status
            .borrow()
            .as_ref()
            .is_some_and(|s| s.availability == RuntimeAvailability::Available)
    }

    /// `listener` is called when status or models change, until it returns false.
    pub fn subscribe(&self, listener: Listener) {
        self.listeners.borrow_mut().push(listener);
    }

    fn notify(&self) {
        let listeners = self.listeners.take();
        let kept: Vec<Listener> = listeners.into_iter().filter(|listener| listener()).collect();
        let mut current = self.listeners.borrow_mut();
        let added = std::mem::replace(&mut *current, kept);
        current.extend(added);
    }

    fn set_status(&self, status: Option<RuntimeStatus>) {
        self.status.replace(status);
        self.notify();
    }

    fn set_model_state(&self, state: ModelState) {
        self.model_state.replace(state);
        self.notify();
    }

    fn saved_model(&self) -> Option<String> {
        self.prefs.borrow().as_ref().and_then(|(prefs, _)| prefs.borrow().companion_model.clone())
    }

    fn save_model(&self, value: Option<String>) {
        if let Some((prefs, path)) = self.prefs.borrow().as_ref() {
            prefs.borrow_mut().companion_model = value;
            if let Err(err) = prefs.borrow().save_to(path) {
                eprintln!("mdow: could not save state: {err}");
            }
        }
    }

    /// Checks for OpenCode, starts its server and loads the live model list.
    pub async fn connect(self: Rc<Self>, directory: Option<String>) {
        if self.status.borrow().is_some() && !self.is_available() {
            self.set_status(None);
        }
        if let Err(error) = self.ensure_client().await {
            let detected = blocking(|| process::detect().0).await.ok();
            let status = match detected {
                Some(found) if found.availability != RuntimeAvailability::Available => found,
                found => RuntimeStatus {
                    availability: RuntimeAvailability::Failed,
                    version: found.and_then(|f| f.version),
                    detail: Some(error),
                },
            };
            let reason = status.detail.clone();
            self.status.replace(Some(status));
            self.set_model_state(ModelState {
                options: Vec::new(),
                current_value: None,
                stale: true,
                unavailable_reason: reason,
            });
            return;
        }
        let directory =
            directory.unwrap_or_else(|| glib::home_dir().to_string_lossy().into_owned());
        self.load_models(&directory).await;
    }

    fn running_client(&self) -> Option<Client> {
        if self.server.borrow().is_some() { self.client.borrow().clone() } else { None }
    }

    pub async fn ensure_client(self: &Rc<Self>) -> Result<Client, String> {
        if let Some(client) = self.running_client() {
            return Ok(client);
        }
        let waiting = self.starting.borrow().clone();
        if let Some(waiting) = waiting {
            let _ = waiting.recv().await;
            return self.running_client().ok_or_else(|| {
                self.start_error
                    .borrow()
                    .clone()
                    .unwrap_or_else(|| "OpenCode is not available.".to_string())
            });
        }
        let (done, waiting) = async_channel::bounded::<()>(1);
        self.starting.replace(Some(waiting));
        let result = self.start_server().await;
        self.starting.replace(None);
        drop(done);
        self.start_error.replace(result.as_ref().err().cloned());
        result
    }

    async fn start_server(self: &Rc<Self>) -> Result<Client, String> {
        let (exit_sender, exit_receiver) = async_channel::bounded::<()>(1);
        let started = blocking(move || {
            let (status, binary) = process::detect();
            let Some(binary) = binary else {
                return Err(status
                    .detail
                    .unwrap_or_else(|| "OpenCode is not available.".to_string()));
            };
            let server = process::start(&binary.path, move || {
                let _ = exit_sender.send_blocking(());
            })?;
            Ok((server, binary.version))
        })
        .await??;
        let (server, version) = started;
        let client = Client::new(&server.url, &server.password);
        let server = Arc::new(server);
        self.server.replace(Some(server.clone()));
        self.client.replace(Some(client.clone()));
        self.set_status(Some(RuntimeStatus {
            availability: RuntimeAvailability::Available,
            version: Some(version),
            detail: None,
        }));
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let _ = exit_receiver.recv().await;
            if let Some(runtime) = weak.upgrade() {
                runtime.server_exited(&server);
            }
        });
        Ok(client)
    }

    fn server_exited(&self, exited: &Arc<Server>) {
        let current = self.server.borrow().as_ref().is_some_and(|s| Arc::ptr_eq(s, exited));
        if !current {
            return;
        }
        let unexpected = !self.stopping.replace(false);
        self.server.replace(None);
        self.client.replace(None);
        let status = (unexpected && self.visible_panels.get() > 0).then(|| RuntimeStatus {
            availability: RuntimeAvailability::Failed,
            version: None,
            detail: Some("OpenCode stopped unexpectedly.".to_string()),
        });
        self.status.replace(status);
        self.set_model_state(ModelState::loading());
        for session in self.live_sessions() {
            session.server_stopped();
        }
    }

    fn live_sessions(&self) -> Vec<Rc<Session>> {
        let mut sessions = self.sessions.borrow_mut();
        sessions.retain(|s| s.strong_count() > 0);
        sessions.iter().filter_map(Weak::upgrade).collect()
    }

    /// Stops the server right away, for quitting and idle time.
    pub fn stop_now(&self) {
        if let Some(timer) = self.idle_timer.take() {
            timer.remove();
        }
        let Some(server) = self.server.borrow().clone() else { return };
        self.stopping.set(true);
        for session in self.live_sessions() {
            session.server_stopped();
        }
        server.stop();
    }

    pub fn panel_visibility_changed(self: &Rc<Self>, visible: bool) {
        self.visible_panels.set((self.visible_panels.get() + if visible { 1 } else { -1 }).max(0));
        self.schedule_idle_stop();
    }

    fn turn_count_changed(self: &Rc<Self>, delta: i32) {
        self.active_turns.set((self.active_turns.get() + delta).max(0));
        self.schedule_idle_stop();
    }

    fn schedule_idle_stop(self: &Rc<Self>) {
        if let Some(timer) = self.idle_timer.take() {
            timer.remove();
        }
        if self.visible_panels.get() != 0
            || self.active_turns.get() != 0
            || self.server.borrow().is_none()
        {
            return;
        }
        let weak = Rc::downgrade(self);
        let timer = glib::timeout_add_local_once(IDLE_TIMEOUT, move || {
            if let Some(runtime) = weak.upgrade() {
                runtime.idle_timer.take();
                runtime.stop_now();
            }
        });
        self.idle_timer.replace(Some(timer));
    }

    pub async fn load_models(self: &Rc<Self>, directory: &str) {
        let client = match self.ensure_client().await {
            Ok(client) => client,
            Err(error) => {
                self.set_model_state(ModelState {
                    options: Vec::new(),
                    current_value: None,
                    stale: true,
                    unavailable_reason: Some(error),
                });
                return;
            }
        };
        let saved = self.saved_model();
        let state = async {
            // A fresh server loads the model catalog in the background, so the first answers can
            // be empty.
            let dir = directory.to_string();
            let mut models = call(&client, move |c| c.list_models(&dir)).await?;
            let mut attempt = 1;
            while models.is_empty() && attempt < MODEL_WARMUP_ATTEMPTS {
                glib::timeout_future(Duration::from_millis(500)).await;
                let dir = directory.to_string();
                models = call(&client, move |c| c.list_models(&dir)).await?;
                attempt += 1;
            }
            let dir = directory.to_string();
            let providers =
                call(&client, move |c| c.list_providers(&dir)).await.unwrap_or_default();
            let options = logic::model_options(&models, &providers);
            let mut current = saved.filter(|value| options.iter().any(|o| &o.value == value));
            if current.is_none() {
                let dir = directory.to_string();
                if let Ok(Some(fallback)) = call(&client, move |c| c.default_model(&dir)).await
                    && options.iter().any(|o| o.value == fallback.value())
                {
                    current = Some(fallback.value());
                }
            }
            let unavailable_reason = options.is_empty().then(|| {
                "No models yet. Run `opencode auth login` to connect a provider.".to_string()
            });
            Ok::<ModelState, String>(ModelState {
                options,
                current_value: current,
                stale: false,
                unavailable_reason,
            })
        }
        .await;
        self.set_model_state(state.unwrap_or_else(|error| ModelState {
            options: Vec::new(),
            current_value: None,
            stale: true,
            unavailable_reason: Some(error),
        }));
    }

    fn has_model(&self, value: &str) -> bool {
        self.model_state.borrow().options.iter().any(|o| o.value == value)
    }

    /// Remembers the choice; sessions switch their own OpenCode session.
    fn select_model(&self, value: &str) -> Result<(), String> {
        if ModelRef::parse(value).is_none() || !self.has_model(value) {
            return Err(format!("Model is not available: {value}"));
        }
        self.save_model(Some(value.to_string()));
        self.model_state.borrow_mut().current_value = Some(value.to_string());
        self.notify();
        Ok(())
    }

    /// The default model chosen in Settings; `None` uses OpenCode's own.
    pub fn set_default_model(&self, value: Option<String>) {
        self.save_model(value);
    }

    pub fn default_model_value(&self) -> Option<String> {
        self.saved_model()
    }

    /// The saved model when OpenCode still offers it, otherwise OpenCode's own default.
    async fn session_model(self: &Rc<Self>, directory: &str) -> Option<ModelRef> {
        let saved = self.saved_model()?;
        if self.model_state.borrow().options.is_empty() {
            self.load_models(directory).await;
        }
        if self.has_model(&saved) { ModelRef::parse(&saved) } else { None }
    }
}

/// What the session knows about the window it sits in.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct DocumentContext {
    /// The document on screen, if any.
    pub document: Option<PathBuf>,
    /// A folder the user opened, if any. Without one the companion works in the document's folder.
    pub folder: Option<PathBuf>,
}

struct ActiveSession {
    id: String,
    directory: String,
    translator: RefCell<TurnTranslator>,
    cancel: Arc<StreamCancel>,
    is_open: Cell<bool>,
    open_waiter: RefCell<Option<async_channel::Sender<Result<(), String>>>>,
}

impl ActiveSession {
    fn resolve_open(&self, result: Result<(), String>) {
        if let Some(waiter) = self.open_waiter.take() {
            let _ = waiter.try_send(result);
        }
    }
}

struct ActiveTurn {
    message_id: String,
    /// When the prompt was sent (ms), to tell this turn's finish from an earlier run's.
    prompted_at: Cell<f64>,
    /// Set once OpenCode starts the run for this prompt. Until then, events still arriving from a
    /// run the user stopped are ignored, so a late "interrupted" can't end this turn.
    started: Cell<bool>,
}

fn now_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

pub struct Session {
    pub runtime: Rc<Runtime>,
    conversation: RefCell<Conversation>,
    context: RefCell<DocumentContext>,
    active: RefCell<Option<Rc<ActiveSession>>>,
    pending_permissions: RefCell<HashSet<String>>,
    turn: RefCell<Option<Rc<ActiveTurn>>>,
    on_change: RefCell<Option<Rc<dyn Fn()>>>,
}

impl Session {
    pub fn new() -> Rc<Session> {
        let runtime = Runtime::shared();
        let session = Rc::new(Session {
            runtime: runtime.clone(),
            conversation: RefCell::new(Conversation::default()),
            context: RefCell::new(DocumentContext::default()),
            active: RefCell::new(None),
            pending_permissions: RefCell::new(HashSet::new()),
            turn: RefCell::new(None),
            on_change: RefCell::new(None),
        });
        runtime.sessions.borrow_mut().push(Rc::downgrade(&session));
        session
    }

    pub fn set_on_change(&self, on_change: Rc<dyn Fn()>) {
        self.on_change.replace(Some(on_change));
    }

    fn changed(&self) {
        let on_change = self.on_change.borrow().clone();
        if let Some(on_change) = on_change {
            on_change();
        }
    }

    pub fn set_context(&self, context: DocumentContext) {
        self.context.replace(context);
    }

    pub fn conversation(&self) -> Conversation {
        self.conversation.borrow().clone()
    }

    pub fn directory(&self) -> String {
        let context = self.context.borrow();
        logic::session_directory(
            context.document.as_deref(),
            context.folder.as_deref(),
            &glib::home_dir(),
        )
    }

    fn set_turn(&self, turn: Option<Rc<ActiveTurn>>) {
        let was = self.turn.borrow().is_some();
        let is = turn.is_some();
        self.turn.replace(turn);
        if was != is {
            self.runtime.turn_count_changed(if is { 1 } else { -1 });
        }
    }

    fn is_current_turn(&self, turn: &Rc<ActiveTurn>) -> bool {
        self.turn.borrow().as_ref().is_some_and(|t| Rc::ptr_eq(t, turn))
    }

    fn emit(&self, update: Update) {
        self.conversation.borrow_mut().apply(update);
        self.changed();
    }

    /// Sends `text` as the user's message, with what they are looking at as context.
    pub fn send(self: &Rc<Self>, text: &str) {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() || self.conversation.borrow().streaming {
            return;
        }
        {
            let mut conversation = self.conversation.borrow_mut();
            conversation.append_user_message(&trimmed);
            conversation.begin_request();
        }
        self.changed();
        let active_path =
            self.context.borrow().document.as_ref().map(|path| path.to_string_lossy().into_owned());
        let tags = self.conversation.borrow().tags.clone();
        let session = self.clone();
        glib::spawn_future_local(async move {
            session.send_to_opencode(trimmed, active_path, tags).await;
        });
    }

    pub fn stop(self: &Rc<Self>) {
        let session = self.clone();
        glib::spawn_future_local(async move {
            session.cancel().await;
            session.conversation.borrow_mut().cancel_request();
            session.changed();
        });
    }

    pub fn new_chat(self: &Rc<Self>) {
        self.conversation.borrow_mut().reset();
        self.changed();
        self.shutdown();
    }

    pub fn add_tag(&self, path: &Path) {
        self.conversation.borrow_mut().add_tag(ContextTag::new(&path.to_string_lossy()));
        self.changed();
    }

    pub fn remove_tag(&self, source_id: &str) {
        self.conversation.borrow_mut().remove_tag(source_id);
        self.changed();
    }

    pub fn select_model(self: &Rc<Self>, value: &str) {
        match self.runtime.select_model(value) {
            Ok(()) => self.conversation.borrow_mut().error = None,
            Err(error) => {
                self.conversation.borrow_mut().error = Some(error);
                self.changed();
                return;
            }
        }
        self.changed();
        let active = self.active.borrow().clone();
        let (Some(active), Some(model)) = (active, ModelRef::parse(value)) else { return };
        let session = self.clone();
        glib::spawn_future_local(async move {
            let result = async {
                let client = session.runtime.ensure_client().await?;
                let id = active.id.clone();
                call(&client, move |c| c.switch_model(&id, &model)).await
            }
            .await;
            if let Err(error) = result {
                session.conversation.borrow_mut().error = Some(error);
                session.changed();
            }
        });
    }

    /// Accepts (`once`) or declines a suggested edit.
    pub fn review(self: &Rc<Self>, permission_id: &str, decision: PermissionDecision) {
        let Some(tool_call_id) =
            self.conversation.borrow().change(permission_id).map(|c| c.tool_call_id.clone())
        else {
            return;
        };
        // Hide the buttons right away; OpenCode confirms with applied or rejected events.
        let status = match decision {
            PermissionDecision::Reject => ChangeStatus::Rejected,
            PermissionDecision::Approve => ChangeStatus::Pending,
        };
        self.conversation.borrow_mut().mark_reviewed(&tool_call_id, status, None);
        self.changed();
        let session = self.clone();
        let permission = permission_id.to_string();
        glib::spawn_future_local(async move {
            if let Err(error) = session.reply_permission(&permission, decision).await {
                let mut conversation = session.conversation.borrow_mut();
                conversation.mark_reviewed(
                    &tool_call_id,
                    ChangeStatus::Pending,
                    Some(permission.clone()),
                );
                conversation.error = Some(error);
                drop(conversation);
                session.changed();
            }
        });
    }

    /// The window closed or the chat was cleared: stop whatever is running.
    pub fn shutdown(self: &Rc<Self>) {
        let session = self.clone();
        glib::spawn_future_local(async move {
            if session.turn.borrow().is_some() {
                session.cancel().await;
            }
            session.close_session();
        });
    }

    async fn send_to_opencode(
        self: &Rc<Self>,
        text: String,
        active_path: Option<String>,
        tags: Vec<ContextTag>,
    ) {
        if self.turn.borrow().is_some() {
            self.emit(Update::Warning(
                "Wait for the current response or stop it first.".to_string(),
            ));
            return;
        }
        let turn = Rc::new(ActiveTurn {
            message_id: new_id(),
            prompted_at: Cell::new(now_ms()),
            started: Cell::new(false),
        });
        self.set_turn(Some(turn.clone()));
        let result = async {
            let client = self.runtime.ensure_client().await?;
            if !self.is_current_turn(&turn) {
                return Ok(());
            }
            let directory = self.directory();
            let session = self.ensure_session(&client, &directory).await?;
            // Stopped while OpenCode was starting: nothing is waiting for a response any more.
            if !self.is_current_turn(&turn) {
                return Ok(());
            }
            session.translator.borrow_mut().begin_turn();
            turn.prompted_at.set(now_ms());
            let prompt = logic::compose_prompt(&text, active_path.as_deref(), &tags, &directory);
            let id = session.id.clone();
            call(&client, move |c| c.prompt(&id, &prompt)).await
        }
        .await;
        if let Err(error) = result
            && self.is_current_turn(&turn)
        {
            self.emit(Update::Error(error));
            self.set_turn(None);
        }
    }

    async fn ensure_session(
        self: &Rc<Self>,
        client: &Client,
        directory: &str,
    ) -> Result<Rc<ActiveSession>, String> {
        let existing = self.active.borrow().clone();
        if let Some(active) = existing.filter(|a| a.directory == directory) {
            return Ok(active);
        }
        self.close_session();

        let model = self.runtime.session_model(directory).await;
        let dir = directory.to_string();
        let id = call(client, move |c| {
            c.create_session(&dir, model.as_ref(), &logic::session_permissions())
        })
        .await?;
        // Older 2.x servers may lack session instructions; the permission rules still apply.
        let session_id = id.clone();
        let _ = call(client, move |c| c.set_instructions(&session_id, "mdow", logic::INSTRUCTIONS))
            .await;

        let (open_sender, open_receiver) = async_channel::bounded(1);
        let session = Rc::new(ActiveSession {
            translator: RefCell::new(TurnTranslator::new(&id, directory)),
            id,
            directory: directory.to_string(),
            cancel: Arc::new(StreamCancel::default()),
            is_open: Cell::new(false),
            open_waiter: RefCell::new(Some(open_sender)),
        });
        self.active.replace(Some(session.clone()));

        let (items, incoming) = async_channel::unbounded::<StreamItem>();
        let stream_client = client.clone();
        let stream_dir = directory.to_string();
        let cancel = session.cancel.clone();
        std::thread::spawn(move || {
            stream_client
                .stream_events(&stream_dir, &cancel, |item| items.send_blocking(item).is_ok());
        });
        let weak = Rc::downgrade(self);
        let stream_session = session.clone();
        glib::spawn_future_local(async move {
            while let Ok(item) = incoming.recv().await {
                let Some(this) = weak.upgrade() else { return };
                match item {
                    StreamItem::Opened { reconnected } => {
                        stream_session.is_open.set(true);
                        stream_session.resolve_open(Ok(()));
                        if reconnected {
                            this.reconcile_turn(&stream_session).await;
                        }
                    }
                    StreamItem::Event(event) => this.handle_event(&stream_session, &event),
                    StreamItem::Failed(error) => {
                        stream_session.resolve_open(Err(error.clone()));
                        let current = this
                            .active
                            .borrow()
                            .as_ref()
                            .is_some_and(|a| Rc::ptr_eq(a, &stream_session));
                        if current && this.turn.borrow().is_some() {
                            this.emit(Update::Error(error));
                            this.set_turn(None);
                        }
                    }
                }
            }
        });

        // Prompting before the stream is open would lose the start of the response, so wait.
        if !session.is_open.get() {
            let timeout_session = session.clone();
            glib::spawn_future_local(async move {
                glib::timeout_future(EVENT_STREAM_TIMEOUT).await;
                timeout_session
                    .resolve_open(Err("OpenCode did not open its event stream.".to_string()));
            });
            let opened = open_receiver
                .recv()
                .await
                .unwrap_or_else(|_| Err("OpenCode did not open its event stream.".to_string()));
            if let Err(error) = opened {
                if self.active.borrow().as_ref().is_some_and(|a| Rc::ptr_eq(a, &session)) {
                    self.close_session();
                }
                return Err(error);
            }
        }
        Ok(session)
    }

    /// Events sent while the stream was reconnecting are not replayed. If the turn finished in
    /// that gap, read how it ended from the session instead of waiting forever.
    async fn reconcile_turn(self: &Rc<Self>, session: &Rc<ActiveSession>) {
        let Some(turn) = self.turn.borrow().clone() else { return };
        let Ok(client) = self.runtime.ensure_client().await else { return };
        let id = session.id.clone();
        let Ok(info) = call(&client, move |c| c.get_session(&id)).await else { return };
        if !self.is_current_turn(&turn) {
            return;
        }
        let (Some(outcome), Some(idle)) = (info.outcome, info.time.idle) else { return };
        if idle < turn.prompted_at.get() {
            return;
        }
        turn.started.set(true);
        let outcome = match outcome.as_str() {
            "succeeded" => Outcome::Succeeded,
            "failed" => Outcome::Failed,
            _ => Outcome::Interrupted,
        };
        self.finish_turn(TurnFinish { outcome, error: None });
    }

    fn close_session(&self) {
        if let Some(active) = self.active.take() {
            active.cancel.cancel();
            active.resolve_open(Err("The conversation was closed.".to_string()));
        }
        self.pending_permissions.borrow_mut().clear();
    }

    fn handle_event(self: &Rc<Self>, session: &Rc<ActiveSession>, event: &Event) {
        if !self.active.borrow().as_ref().is_some_and(|a| Rc::ptr_eq(a, session)) {
            return;
        }
        let result = session.translator.borrow_mut().translate(event);
        let turn = self.turn.borrow().clone();
        if result.started
            && let Some(turn) = &turn
        {
            turn.started.set(true);
        }
        let live = turn.is_some_and(|t| t.started.get());

        if live {
            for update in result.updates {
                if let Update::Change {
                    permission_id: Some(permission),
                    status: ChangeStatus::Pending,
                    ..
                } = &update
                {
                    self.pending_permissions.borrow_mut().insert(permission.clone());
                }
                self.emit(update);
            }
        }

        if let Some((permission, reason)) = result.auto_reject {
            let this = self.clone();
            let id = session.id.clone();
            glib::spawn_future_local(async move {
                if let Ok(client) = this.runtime.ensure_client().await {
                    let _ = call(&client, move |c| {
                        c.reply_permission(&id, &permission, PermissionReply::Reject, Some(&reason))
                    })
                    .await;
                }
            });
        }

        if let Some(finish) = result.finish
            && live
        {
            self.finish_turn(finish);
        }
    }

    fn finish_turn(&self, finish: TurnFinish) {
        let Some(turn) = self.turn.borrow().clone() else { return };
        match finish.outcome {
            Outcome::Failed => {
                self.emit(Update::Error(
                    finish.error.unwrap_or_else(|| "OpenCode failed.".to_string()),
                ));
            }
            Outcome::Interrupted => self.emit(Update::Cancelled(turn.message_id.clone())),
            Outcome::Succeeded => self.emit(Update::Done(turn.message_id.clone())),
        }
        self.set_turn(None);
        self.pending_permissions.borrow_mut().clear();
    }

    async fn reply_permission(
        self: &Rc<Self>,
        permission: &str,
        decision: PermissionDecision,
    ) -> Result<(), String> {
        let active = self.active.borrow().clone();
        let waiting = self.pending_permissions.borrow().contains(permission);
        let Some(active) = active.filter(|_| self.runtime.is_available() && waiting) else {
            return Err("This change is no longer waiting for review.".to_string());
        };
        self.pending_permissions.borrow_mut().remove(permission);
        let client = self.runtime.ensure_client().await?;
        let id = active.id.clone();
        let permission = permission.to_string();
        // Mdow only ever answers once: `always` would save the rule in OpenCode's global config.
        call(&client, move |c| match decision {
            PermissionDecision::Approve => {
                c.reply_permission(&id, &permission, PermissionReply::Once, None)
            }
            PermissionDecision::Reject => c.reply_permission(
                &id,
                &permission,
                PermissionReply::Reject,
                Some("The user rejected this change."),
            ),
        })
        .await
    }

    async fn cancel(self: &Rc<Self>) {
        let turn = self.turn.borrow().clone();
        let session = self.active.borrow().clone();
        self.set_turn(None);
        if let Some(session) = session
            && self.runtime.is_available()
            && let Ok(client) = self.runtime.ensure_client().await
        {
            let pending: Vec<String> = self.pending_permissions.borrow_mut().drain().collect();
            let id = session.id.clone();
            let _ = call(&client, move |c| {
                let _ = c.interrupt(&id);
                for permission in &pending {
                    let _ = c.reply_permission(
                        &id,
                        permission,
                        PermissionReply::Reject,
                        Some("The user stopped the response."),
                    );
                }
                Ok(())
            })
            .await;
        }
        if let Some(turn) = turn {
            self.emit(Update::Cancelled(turn.message_id.clone()));
        }
    }

    /// The server went away; the next message starts a new one and a new session.
    fn server_stopped(&self) {
        self.close_session();
        if self.turn.borrow().is_some() {
            self.emit(Update::Error(
                "OpenCode stopped unexpectedly. Send again to restart it.".to_string(),
            ));
            self.set_turn(None);
        }
    }
}
