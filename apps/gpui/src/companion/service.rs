//! The companion service: one ACP agent subprocess per window, driven off the UI thread.
//!
//! [`ServiceCore`] is a deterministic state machine (commands and agent lines in, JSON lines
//! and [`CompanionUpdate`]s out) so every transition is unit-testable without processes.
//! [`ServiceHandle`] runs it on a dedicated thread with a real child process.

use super::{
    acp::{self, Incoming, RpcError, SessionConfig, SessionEvent, TextChannel},
    context::{
        BuildContextInput, CitationChunk, CitationStream, ContextLedger, append_retrieved_context,
        build_context, format_context_prompt, retrieve_markdown_ranges, should_retrieve,
    },
    provider::{self, ProviderCommand, SearchPath},
    types::{
        Citation, CompanionError, CompanionUpdate, ConnectionStatus, ContextTag, ErrorKind,
        ModelState, ProviderId, ProviderStatus, RetrievalMode, TagKind, select_available_provider,
    },
};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command as ProcessCommand, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const STDERR_TAIL_LINES: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServiceSettings {
    pub preferred: Option<ProviderId>,
    pub custom_command: Option<PathBuf>,
    pub last_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendPayload {
    pub text: String,
    pub active_path: Option<PathBuf>,
    pub open_folder: Option<PathBuf>,
    pub tags: Vec<ContextTag>,
    pub provider: Option<ProviderId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Configure(ServiceSettings),
    /// Re-run provider detection.
    Detect,
    /// Start (or reuse) a session so the model picker has live data.
    Connect {
        provider: Option<ProviderId>,
        cwd: PathBuf,
    },
    Send(SendPayload),
    Cancel,
    SetModel(String),
    Permission {
        request_key: String,
        option_id: Option<String>,
    },
    /// New chat: a fresh agent session with no memory of the last one.
    Reset,
    Shutdown,
}

/// The side effects the core asks for. The real host owns a child process; tests record.
pub trait Host {
    fn detect(&mut self, custom: Option<&Path>) -> Vec<ProviderStatus>;
    fn spawn(&mut self, command: &ProviderCommand, cwd: &Path) -> std::io::Result<()>;
    fn write(&mut self, line: &str) -> bool;
    fn kill(&mut self);
    fn emit(&mut self, update: CompanionUpdate);
    fn fff_server(&mut self) -> Option<acp::McpServer>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Initializing,
    CreatingSession,
    ApplyingModel,
    Ready,
}

struct Agent {
    provider: ProviderId,
    cwd: PathBuf,
    display: String,
    session_id: Option<String>,
    config: SessionConfig,
    fff: bool,
    phase: Phase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingKind {
    Initialize,
    NewSession,
    StartupModel,
    SetModel { method: &'static str, value: String },
    Prompt { turn: u64 },
}

struct Pending {
    kind: PendingKind,
    method: &'static str,
    deadline: Option<Instant>,
}

struct Source {
    path: PathBuf,
    heading_id: Option<String>,
    label: String,
}

struct Turn {
    id: u64,
    citations: CitationStream,
    sources: HashMap<String, Source>,
    last_channel: Option<TextChannel>,
}

pub struct ServiceCore<H: Host> {
    host: H,
    version: String,
    settings: ServiceSettings,
    providers: Vec<ProviderStatus>,
    agent: Option<Agent>,
    pending: HashMap<u64, Pending>,
    next_id: u64,
    turn: Option<Turn>,
    next_turn: u64,
    queued: Option<SendPayload>,
    permissions: HashMap<String, Value>,
    ledger: ContextLedger,
    tool_counter: u64,
    last_connect: Option<(ProviderId, PathBuf)>,
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn default_cwd() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

impl<H: Host> ServiceCore<H> {
    pub fn new(host: H, version: impl Into<String>) -> Self {
        Self {
            host,
            version: version.into(),
            settings: ServiceSettings::default(),
            providers: Vec::new(),
            agent: None,
            pending: HashMap::new(),
            next_id: 1,
            turn: None,
            next_turn: 1,
            queued: None,
            permissions: HashMap::new(),
            ledger: ContextLedger::default(),
            tool_counter: 0,
            last_connect: None,
        }
    }

    pub fn host(&self) -> &H {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    pub fn is_busy(&self) -> bool {
        self.turn.is_some() || self.queued.is_some()
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.pending
            .values()
            .filter_map(|pending| pending.deadline)
            .min()
    }

    pub fn command(&mut self, command: Command, now: Instant) {
        match command {
            Command::Configure(settings) => self.settings = settings,
            Command::Detect => {
                self.detect();
            }
            Command::Connect { provider, cwd } => self.connect(provider, cwd, now),
            Command::Send(payload) => self.send(payload, now),
            Command::Cancel => self.cancel(now),
            Command::SetModel(value) => self.set_model(value, now),
            Command::Permission {
                request_key,
                option_id,
            } => self.answer_permission(&request_key, option_id.as_deref()),
            Command::Reset => self.reset(now),
            Command::Shutdown => self.shutdown(),
        }
    }

    fn detect(&mut self) -> Vec<ProviderStatus> {
        self.providers = self.host.detect(self.settings.custom_command.as_deref());
        self.host
            .emit(CompanionUpdate::Providers(self.providers.clone()));
        self.providers.clone()
    }

    fn choose_provider(&mut self, requested: Option<ProviderId>) -> Option<ProviderId> {
        if self.providers.is_empty() {
            self.detect();
        }
        if let Some(requested) = requested
            && self
                .providers
                .iter()
                .any(|status| status.id == requested && status.is_available())
        {
            return Some(requested);
        }
        select_available_provider(&self.providers, self.settings.preferred)
    }

    fn no_provider_error() -> CompanionError {
        CompanionError::new(
            ErrorKind::NoProvider,
            "No agent found",
            "Install OpenCode, Codex ACP, Claude Code ACP or Gemini CLI, or choose a custom \
             ACP executable in Settings, then retry.",
        )
    }

    fn connect(&mut self, provider: Option<ProviderId>, cwd: PathBuf, now: Instant) {
        let Some(provider) = self.choose_provider(provider) else {
            self.host
                .emit(CompanionUpdate::Error(Self::no_provider_error()));
            return;
        };
        self.last_connect = Some((provider, cwd.clone()));
        if let Some(agent) = self.agent.as_ref()
            && agent.provider == provider
            && agent.cwd == cwd
        {
            if agent.phase == Phase::Ready {
                self.emit_ready_state();
            }
            return;
        }
        if self.turn.is_some() {
            // Never swap agents under a streaming answer.
            return;
        }
        self.start_agent(provider, cwd, now);
    }

    fn emit_ready_state(&mut self) {
        let Some(agent) = self.agent.as_ref() else {
            return;
        };
        let models = agent.config.model_state(agent.session_id.is_some());
        let provider = agent.provider;
        self.host.emit(CompanionUpdate::Connection {
            status: ConnectionStatus::Ready,
            provider: Some(provider),
        });
        self.host.emit(CompanionUpdate::Models(models));
    }

    fn start_agent(&mut self, provider: ProviderId, cwd: PathBuf, now: Instant) {
        self.stop_agent();
        let executable = self
            .providers
            .iter()
            .find(|status| status.id == provider)
            .and_then(|status| status.executable.clone());
        let custom = self.settings.custom_command.clone();
        let Some(mut command) =
            provider::resolve_command(provider, custom.as_deref(), &SearchPath::default())
        else {
            self.fail_start(CompanionError::new(
                ErrorKind::MissingCli,
                "Provider command is not configured",
                provider.install_hint(),
            ));
            return;
        };
        if let Some(executable) = executable {
            command.program = executable;
        }
        self.host.emit(CompanionUpdate::Connection {
            status: ConnectionStatus::Connecting,
            provider: Some(provider),
        });
        if let Err(error) = self.host.spawn(&command, &cwd) {
            let (title, message) = match error.kind() {
                std::io::ErrorKind::NotFound => (
                    format!("Couldn't find {}", command.display),
                    provider.install_hint().to_owned(),
                ),
                std::io::ErrorKind::PermissionDenied => (
                    format!("{} isn't executable", command.display),
                    "Check the file's permissions, or choose another executable.".to_owned(),
                ),
                _ => (
                    format!("Couldn't start {}", command.display),
                    error.to_string(),
                ),
            };
            self.fail_start(CompanionError::new(ErrorKind::MissingCli, title, message));
            return;
        }
        self.agent = Some(Agent {
            provider,
            cwd,
            display: command.display.clone(),
            session_id: None,
            config: SessionConfig::default(),
            fff: false,
            phase: Phase::Initializing,
        });
        self.ledger.clear();
        let params = acp::initialize_params(&self.version);
        self.request("initialize", params, PendingKind::Initialize, now, true);
    }

    fn stop_agent(&mut self) {
        let had_agent = self.agent.take().is_some();
        self.permissions.clear();
        self.pending.clear();
        self.ledger.clear();
        if had_agent {
            self.host.kill();
        }
    }

    fn fail_start(&mut self, error: CompanionError) {
        self.stop_agent();
        self.queued = None;
        self.turn = None;
        self.host.emit(CompanionUpdate::Connection {
            status: ConnectionStatus::Idle,
            provider: None,
        });
        self.host.emit(CompanionUpdate::Models(ModelState {
            unavailable_reason: Some(error.title.clone()),
            ..ModelState::not_started()
        }));
        self.host.emit(CompanionUpdate::Error(error));
    }

    fn request(
        &mut self,
        method: &'static str,
        params: Value,
        kind: PendingKind,
        now: Instant,
        timed: bool,
    ) -> bool {
        let id = self.next_id;
        self.next_id += 1;
        self.pending.insert(
            id,
            Pending {
                kind,
                method,
                deadline: timed.then(|| now + REQUEST_TIMEOUT),
            },
        );
        if self.host.write(&acp::encode_request(id, method, params)) {
            true
        } else {
            self.pending.remove(&id);
            false
        }
    }

    fn send(&mut self, payload: SendPayload, now: Instant) {
        if self.is_busy() {
            self.host.emit(CompanionUpdate::Warning(
                "Wait for the current response or cancel it first.".into(),
            ));
            return;
        }
        let Some(provider) = self.choose_provider(payload.provider) else {
            self.host
                .emit(CompanionUpdate::Error(Self::no_provider_error()));
            return;
        };
        let cwd = payload.open_folder.clone().unwrap_or_else(default_cwd);
        self.last_connect = Some((provider, cwd.clone()));
        match self.agent.as_ref() {
            Some(agent) if agent.provider == provider && agent.cwd == cwd => {
                if agent.phase == Phase::Ready {
                    self.begin_turn(payload, now);
                } else {
                    self.queued = Some(payload);
                }
            }
            _ => {
                self.queued = Some(payload);
                self.start_agent(provider, cwd, now);
            }
        }
    }

    fn begin_turn(&mut self, payload: SendPayload, now: Instant) {
        let (session_id, fff) = match self.agent.as_ref() {
            Some(agent) => (agent.session_id.clone().unwrap_or_default(), agent.fff),
            None => return,
        };
        let mut packet = build_context(
            BuildContextInput {
                active_path: payload.active_path.as_deref(),
                tags: &payload.tags,
                question: &payload.text,
            },
            &mut self.ledger,
        );
        if should_retrieve(&payload.text, payload.active_path.as_deref(), &payload.tags) {
            let roots = payload
                .tags
                .iter()
                .filter(|tag| tag.kind == TagKind::Folder)
                .map(|tag| tag.path.clone())
                .chain(payload.open_folder.clone())
                .collect::<Vec<_>>();
            let excluded = packet
                .sources
                .iter()
                .map(|source| source.path.clone())
                .collect::<Vec<_>>();
            let ranges = retrieve_markdown_ranges(&payload.text, &roots, &excluded);
            let mode = if fff {
                RetrievalMode::AdaptiveFff
            } else {
                RetrievalMode::AdaptiveLocal
            };
            packet = append_retrieved_context(packet, &ranges, mode);
        }
        let sources = packet
            .sources
            .iter()
            .map(|source| {
                (
                    source.source_id.clone(),
                    Source {
                        path: source.path.clone(),
                        heading_id: source.heading_id.clone(),
                        label: file_label(&source.path),
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        self.host.emit(CompanionUpdate::Context {
            summary: packet.summary.clone(),
            warnings: packet.warnings.clone(),
            trace: packet.trace.clone(),
        });
        let turn = self.next_turn;
        self.next_turn += 1;
        self.turn = Some(Turn {
            id: turn,
            citations: CitationStream::new(sources.keys().cloned()),
            sources,
            last_channel: None,
        });
        let prompt = format_context_prompt(&packet, &payload.text);
        if !self.request(
            "session/prompt",
            acp::prompt_params(&session_id, &prompt),
            PendingKind::Prompt { turn },
            now,
            false,
        ) {
            self.turn = None;
            self.agent_gone(None, String::new());
        }
    }

    fn cancel(&mut self, now: Instant) {
        let had_turn = self.turn.take().is_some() || self.queued.take().is_some();
        if let Some(session_id) = self
            .agent
            .as_ref()
            .and_then(|agent| agent.session_id.clone())
        {
            self.host.write(&acp::encode_notification(
                "session/cancel",
                acp::cancel_params(&session_id),
            ));
        }
        self.resolve_all_permissions();
        // As in Electron, a cancelled turn also retires the agent process: some agents keep
        // streaming after `session/cancel`, and a fresh session never mixes two answers.
        self.stop_agent();
        if had_turn {
            self.host.emit(CompanionUpdate::Cancelled);
        }
        self.host.emit(CompanionUpdate::Connection {
            status: ConnectionStatus::Idle,
            provider: None,
        });
        if let Some((provider, cwd)) = self.last_connect.clone() {
            self.start_agent(provider, cwd, now);
        }
    }

    fn reset(&mut self, now: Instant) {
        if self.turn.is_some() || self.queued.is_some() {
            self.cancel(now);
            return;
        }
        self.stop_agent();
        if let Some((provider, cwd)) = self.last_connect.clone() {
            self.start_agent(provider, cwd, now);
        }
    }

    pub fn shutdown(&mut self) {
        self.resolve_all_permissions();
        self.turn = None;
        self.queued = None;
        self.stop_agent();
    }

    fn set_model(&mut self, value: String, now: Instant) {
        let request = self.agent.as_ref().and_then(|agent| {
            let session = agent.session_id.as_deref()?;
            (agent.phase == Phase::Ready)
                .then(|| agent.config.model_request(session, &value))
                .flatten()
        });
        match request {
            Some((method, params)) => {
                self.request(
                    method,
                    params,
                    PendingKind::SetModel { method, value },
                    now,
                    true,
                );
            }
            None => self.host.emit(CompanionUpdate::Error(CompanionError::new(
                ErrorKind::Agent,
                "Couldn't change model",
                if self.agent.is_some() {
                    format!("Model is not available in this session: {value}")
                } else {
                    "Start Companion to choose a model".into()
                },
            ))),
        }
    }

    fn answer_permission(&mut self, request_key: &str, option_id: Option<&str>) {
        let Some(id) = self.permissions.remove(request_key) else {
            return;
        };
        let result = match option_id {
            Some(option) => acp::permission_selected(option),
            None => acp::permission_cancelled(),
        };
        self.host.write(&acp::encode_result(&id, result));
        self.host.emit(CompanionUpdate::PermissionResolved {
            request_key: request_key.to_owned(),
        });
    }

    fn resolve_all_permissions(&mut self) {
        let keys = self.permissions.keys().cloned().collect::<Vec<_>>();
        for key in keys {
            self.answer_permission(&key, None);
        }
    }

    pub fn tick(&mut self, now: Instant) {
        let expired = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.deadline.is_some_and(|deadline| deadline <= now))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in expired {
            let Some(pending) = self.pending.remove(&id) else {
                continue;
            };
            let error = CompanionError::new(
                ErrorKind::Timeout,
                "The agent stopped responding",
                format!(
                    "ACP {} timed out after {}s. Check that the CLI works in Terminal, then retry.",
                    pending.method,
                    REQUEST_TIMEOUT.as_secs()
                ),
            );
            match pending.kind {
                PendingKind::SetModel { .. } => self.host.emit(CompanionUpdate::Error(error)),
                _ => self.fail_start(error),
            }
        }
    }

    pub fn agent_line(&mut self, line: &str, now: Instant) {
        if line.trim().is_empty() {
            return;
        }
        match acp::parse_line(line) {
            Ok(Incoming::Response { id, result }) => self.on_response(&id, result, now),
            Ok(Incoming::Request { id, method, params }) => self.on_request(id, &method, &params),
            Ok(Incoming::Notification { method, params }) => {
                if method == "session/update" {
                    self.on_session_update(&params);
                }
            }
            Err(_) => self.host.emit(CompanionUpdate::Warning(
                "Ignored malformed ACP message".into(),
            )),
        }
    }

    fn on_response(&mut self, id: &Value, result: Result<Value, RpcError>, now: Instant) {
        let Some(pending) = id.as_u64().and_then(|id| self.pending.remove(&id)) else {
            return;
        };
        match (pending.kind, result) {
            (PendingKind::Initialize, Ok(_)) => {
                let Some(agent) = self.agent.as_mut() else {
                    return;
                };
                agent.phase = Phase::CreatingSession;
                let fff = if agent.provider == ProviderId::OpenCode {
                    self.host.fff_server()
                } else {
                    None
                };
                let agent = self.agent.as_mut().expect("agent is still starting");
                agent.fff = fff.is_some();
                let params = acp::new_session_params(&agent.cwd, fff.as_slice());
                self.request("session/new", params, PendingKind::NewSession, now, true);
            }
            (PendingKind::NewSession, Ok(result)) => {
                let Some(session_id) = result.get("sessionId").and_then(Value::as_str) else {
                    self.fail_start(CompanionError::agent("ACP session/new missing sessionId"));
                    return;
                };
                let Some(agent) = self.agent.as_mut() else {
                    return;
                };
                agent.session_id = Some(session_id.to_owned());
                agent.config = SessionConfig::from_result(&result);
                let startup_model = self.settings.last_model.clone().and_then(|model| {
                    agent
                        .config
                        .model_request(session_id, &model)
                        .filter(|_| {
                            agent.config.model_state(true).current_value.as_deref()
                                != Some(model.as_str())
                        })
                        .map(|(method, params)| (method, params, model))
                });
                match startup_model {
                    Some((method, params, _)) => {
                        agent.phase = Phase::ApplyingModel;
                        self.request(method, params, PendingKind::StartupModel, now, true);
                    }
                    None => self.became_ready(now),
                }
            }
            (PendingKind::StartupModel, result) => {
                if let (Ok(result), Some(agent), Some(model)) = (
                    result,
                    self.agent.as_mut(),
                    self.settings.last_model.clone(),
                ) {
                    let method = if result.get("configOptions").is_some() {
                        "session/set_config_option"
                    } else {
                        "session/set_model"
                    };
                    agent.config.apply_model_result(method, &model, &result);
                }
                // A live agent may change its configuration between setup and selection;
                // the session still works with its default model.
                self.became_ready(now);
            }
            (PendingKind::Initialize | PendingKind::NewSession, Err(error)) => {
                self.fail_start(self.start_error(&error));
            }
            (PendingKind::SetModel { method, value }, Ok(result)) => {
                if let Some(agent) = self.agent.as_mut() {
                    agent.config.apply_model_result(method, &value, &result);
                    let state = agent.config.model_state(true);
                    let confirmed = state.current_value.clone();
                    self.host.emit(CompanionUpdate::Models(state));
                    if let Some(confirmed) = confirmed {
                        self.settings.last_model = Some(confirmed.clone());
                        self.host.emit(CompanionUpdate::ModelSelected(confirmed));
                    }
                }
            }
            (PendingKind::SetModel { .. }, Err(error)) => {
                self.host.emit(CompanionUpdate::Error(CompanionError::new(
                    ErrorKind::Agent,
                    "Couldn't change model",
                    error.message,
                )));
            }
            (PendingKind::Prompt { turn }, result) => {
                if self.turn.as_ref().map(|active| active.id) != Some(turn) {
                    return;
                }
                match result {
                    Ok(_) => {
                        let mut active = self.turn.take().expect("active turn");
                        let chunk = active.citations.flush();
                        self.emit_chunk(chunk, &active.sources);
                        if active.last_channel == Some(TextChannel::Thinking) {
                            self.host.emit(CompanionUpdate::ThinkingDone);
                        }
                        self.resolve_all_permissions();
                        self.host.emit(CompanionUpdate::Done);
                    }
                    Err(error) => {
                        self.turn = None;
                        self.resolve_all_permissions();
                        let error = if error.is_auth_required() {
                            self.auth_error()
                        } else {
                            CompanionError::agent(error.message)
                        };
                        self.host.emit(CompanionUpdate::Error(error));
                    }
                }
            }
        }
    }

    fn auth_error(&self) -> CompanionError {
        let provider = self
            .agent
            .as_ref()
            .map(|agent| agent.provider)
            .unwrap_or(ProviderId::Custom);
        CompanionError::new(
            ErrorKind::AuthRequired,
            format!("{} needs you to sign in", provider.subject()),
            provider.sign_in_hint(),
        )
    }

    fn start_error(&self, error: &RpcError) -> CompanionError {
        if error.is_auth_required() {
            return self.auth_error();
        }
        let display = self
            .agent
            .as_ref()
            .map(|agent| agent.display.clone())
            .unwrap_or_default();
        CompanionError::new(
            ErrorKind::Agent,
            format!("Couldn't start {display}"),
            error.message.clone(),
        )
    }

    fn became_ready(&mut self, now: Instant) {
        if let Some(agent) = self.agent.as_mut() {
            agent.phase = Phase::Ready;
        }
        self.emit_ready_state();
        if let Some(payload) = self.queued.take() {
            self.begin_turn(payload, now);
        }
    }

    fn on_request(&mut self, id: Value, method: &str, params: &Value) {
        if method == "fs/write_text_file" || method.starts_with("terminal/") {
            self.host.write(&acp::encode_error(
                &id,
                acp::SERVER_ERROR,
                "Refused in read-only companion mode",
            ));
            self.host.emit(CompanionUpdate::Warning(format!(
                "Refused agent request: {method}"
            )));
            return;
        }
        if method == "fs/read_text_file" {
            self.host.write(&acp::encode_error(
                &id,
                acp::SERVER_ERROR,
                "Use provided docs context; direct fs reads are not enabled yet",
            ));
            return;
        }
        if method == "session/request_permission" {
            if self.turn.is_none() {
                self.host
                    .write(&acp::encode_result(&id, acp::permission_cancelled()));
                return;
            }
            let key = acp::id_key(&id);
            let request = acp::parse_permission_request(params, key.clone());
            self.permissions.insert(key, id);
            self.host.emit(CompanionUpdate::Permission(request));
            return;
        }
        self.host.write(&acp::encode_error(
            &id,
            acp::METHOD_NOT_FOUND,
            &format!("Method not supported: {method}"),
        ));
    }

    fn on_session_update(&mut self, params: &Value) {
        self.tool_counter += 1;
        let fallback = format!("tool-{}", self.tool_counter);
        match acp::classify_session_update(params, &fallback) {
            SessionEvent::ConfigOptions(options) => {
                if let Some(agent) = self.agent.as_mut() {
                    agent.config.config_options = options;
                    let state = agent.config.model_state(agent.session_id.is_some());
                    self.host.emit(CompanionUpdate::Models(state));
                }
            }
            SessionEvent::Tool(tool) => {
                let Some(turn) = self.turn.as_mut() else {
                    return;
                };
                let thinking = turn.last_channel == Some(TextChannel::Thinking);
                turn.last_channel = None;
                if thinking {
                    self.host.emit(CompanionUpdate::ThinkingDone);
                }
                self.host.emit(CompanionUpdate::Tool(tool));
            }
            SessionEvent::Text { channel, text } => {
                let Some(turn) = self.turn.as_mut() else {
                    return;
                };
                let previous = turn.last_channel.replace(channel);
                match channel {
                    TextChannel::Thinking => self.host.emit(CompanionUpdate::Thinking(text)),
                    TextChannel::Message => {
                        if previous == Some(TextChannel::Thinking) {
                            self.host.emit(CompanionUpdate::ThinkingDone);
                        }
                        let turn = self.turn.as_mut().expect("active turn");
                        let chunk = turn.citations.consume(&text);
                        let sources = std::mem::take(&mut turn.sources);
                        self.emit_chunk(chunk, &sources);
                        if let Some(turn) = self.turn.as_mut() {
                            turn.sources = sources;
                        }
                    }
                }
            }
            SessionEvent::Ignored => {}
        }
    }

    fn emit_chunk(&mut self, chunk: CitationChunk, sources: &HashMap<String, Source>) {
        if !chunk.text.is_empty() {
            self.host.emit(CompanionUpdate::Delta(chunk.text));
        }
        for id in chunk.citation_ids {
            if let Some(source) = sources.get(&id) {
                self.host.emit(CompanionUpdate::Citation(Citation {
                    source_id: id,
                    path: source.path.clone(),
                    heading_id: source.heading_id.clone(),
                    label: source.label.clone(),
                }));
            }
        }
    }

    /// The agent process exited (or its pipe closed).
    pub fn agent_exited(&mut self, code: Option<i32>, stderr_tail: String) {
        if self.agent.is_none() {
            return;
        }
        self.agent_gone(code, stderr_tail);
    }

    fn agent_gone(&mut self, code: Option<i32>, stderr_tail: String) {
        let starting = self
            .agent
            .as_ref()
            .is_some_and(|agent| agent.phase != Phase::Ready);
        let mid_turn = self.turn.is_some() || self.queued.is_some();
        let display = self
            .agent
            .as_ref()
            .map(|agent| agent.display.clone())
            .unwrap_or_default();
        let status = code
            .map(|code| format!("exit code {code}"))
            .unwrap_or_else(|| "no exit code".into());
        let detail = stderr_tail.trim();
        let message = if detail.is_empty() {
            format!("{display} exited ({status}). Check that it works in Terminal, then retry.")
        } else {
            format!("{display} exited ({status}): {detail}")
        };
        let lowered = detail.to_lowercase();
        let error = if lowered.contains("auth")
            || lowered.contains("log in")
            || lowered.contains("login")
        {
            self.auth_error()
        } else {
            CompanionError::new(
                ErrorKind::AgentExited,
                "The agent stopped unexpectedly",
                message,
            )
        };
        if starting || mid_turn {
            self.turn = None;
            self.fail_start(error);
        } else {
            self.stop_agent();
            self.host.emit(CompanionUpdate::Connection {
                status: ConnectionStatus::Idle,
                provider: None,
            });
            self.host
                .emit(CompanionUpdate::Models(ModelState::not_started()));
        }
    }
}

enum Input {
    Command(Command),
    Line { generation: u64, line: String },
    Closed { generation: u64 },
}

type SharedChild = Arc<Mutex<Option<Child>>>;

/// Kills the process group so adapters' own subprocesses (MCP servers) go too.
fn terminate(child: &mut Child) {
    #[cfg(unix)]
    {
        // Negative pid: the whole group the agent leads (spawned with `process_group(0)`).
        let pid = child.id() as i32;
        // SAFETY: plain signal delivery to a group this process created.
        unsafe {
            libc::kill(-pid, libc::SIGTERM);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

struct ProcessHost {
    child: SharedChild,
    stdin: Option<ChildStdin>,
    generation: u64,
    stderr_tail: Arc<Mutex<Vec<String>>>,
    input: mpsc::Sender<Input>,
    updates: async_channel::Sender<CompanionUpdate>,
    search: Option<SearchPath>,
}

impl ProcessHost {
    fn search(&mut self) -> &SearchPath {
        self.search.get_or_insert_with(SearchPath::system)
    }

    fn exit_details(&mut self) -> (Option<i32>, String) {
        let mut code = None;
        if let Ok(mut guard) = self.child.lock()
            && let Some(child) = guard.as_mut()
        {
            for _ in 0..20 {
                if let Ok(Some(status)) = child.try_wait() {
                    code = status.code();
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        let tail = self
            .stderr_tail
            .lock()
            .map(|lines| lines.join("\n"))
            .unwrap_or_default();
        (code, tail)
    }
}

impl Host for ProcessHost {
    fn detect(&mut self, custom: Option<&Path>) -> Vec<ProviderStatus> {
        let search = self.search().clone();
        provider::detect_providers(custom, &search)
    }

    fn spawn(&mut self, command: &ProviderCommand, cwd: &Path) -> std::io::Result<()> {
        // The search path first, then the inherited PATH so the agent's own tools still resolve.
        let mut dirs = self.search().dirs().to_vec();
        if let Some(path) = std::env::var_os("PATH") {
            dirs.extend(std::env::split_paths(&path));
        }
        dirs.extend(["/usr/bin", "/bin", "/usr/sbin", "/sbin"].map(PathBuf::from));
        let path_var = SearchPath::new(dirs).to_path_var();
        let mut process = ProcessCommand::new(&command.program);
        process
            .args(&command.args)
            .current_dir(if cwd.is_dir() { cwd } else { Path::new("/") })
            .env("PATH", path_var)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            process.process_group(0);
        }
        let mut child = process.spawn()?;
        self.generation += 1;
        let generation = self.generation;
        self.stdin = child.stdin.take();
        if let Ok(mut tail) = self.stderr_tail.lock() {
            tail.clear();
        }
        if let Some(stdout) = child.stdout.take() {
            let input = self.input.clone();
            std::thread::Builder::new()
                .name("companion-agent-stdout".into())
                .spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        let Ok(line) = line else { break };
                        if input.send(Input::Line { generation, line }).is_err() {
                            return;
                        }
                    }
                    let _ = input.send(Input::Closed { generation });
                })?;
        }
        if let Some(stderr) = child.stderr.take() {
            let tail = self.stderr_tail.clone();
            std::thread::Builder::new()
                .name("companion-agent-stderr".into())
                .spawn(move || {
                    for line in BufReader::new(stderr).lines() {
                        let Ok(line) = line else { break };
                        if let Ok(mut tail) = tail.lock() {
                            tail.push(line);
                            let excess = tail.len().saturating_sub(STDERR_TAIL_LINES);
                            tail.drain(..excess);
                        }
                    }
                })?;
        }
        if let Ok(mut guard) = self.child.lock() {
            *guard = Some(child);
        }
        Ok(())
    }

    fn write(&mut self, line: &str) -> bool {
        let Some(stdin) = self.stdin.as_mut() else {
            return false;
        };
        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.flush())
            .is_ok()
    }

    fn kill(&mut self) {
        self.stdin = None;
        // Retire the generation so the dying process's last lines are ignored.
        self.generation += 1;
        if let Ok(mut guard) = self.child.lock()
            && let Some(mut child) = guard.take()
        {
            terminate(&mut child);
        }
    }

    fn emit(&mut self, update: CompanionUpdate) {
        let _ = self.updates.try_send(update);
    }

    fn fff_server(&mut self) -> Option<acp::McpServer> {
        let search = self.search().clone();
        provider::resolve_fff_mcp(&search)
    }
}

/// The UI's end of a running service. Dropping it stops the agent.
pub struct ServiceHandle {
    input: mpsc::Sender<Input>,
    child: SharedChild,
}

impl ServiceHandle {
    /// Starts the service thread. `search` overrides executable lookup (tests).
    pub fn spawn(
        version: impl Into<String>,
        search: Option<SearchPath>,
    ) -> (Self, async_channel::Receiver<CompanionUpdate>) {
        let (input_sender, input_receiver) = mpsc::channel();
        let (update_sender, update_receiver) = async_channel::unbounded();
        let child: SharedChild = Arc::new(Mutex::new(None));
        let host = ProcessHost {
            child: child.clone(),
            stdin: None,
            generation: 0,
            stderr_tail: Arc::new(Mutex::new(Vec::new())),
            input: input_sender.clone(),
            updates: update_sender,
            search,
        };
        let version = version.into();
        let spawned = std::thread::Builder::new()
            .name("companion-service".into())
            .spawn(move || run(ServiceCore::new(host, version), input_receiver));
        if let Err(error) = spawned {
            eprintln!("Mdow: companion service could not start: {error}");
        }
        (
            Self {
                input: input_sender,
                child,
            },
            update_receiver,
        )
    }

    pub fn send(&self, command: Command) {
        let _ = self.input.send(Input::Command(command));
    }

    /// Stops the agent immediately, from any thread (app quit).
    pub fn kill_now(&self) {
        if let Ok(mut guard) = self.child.lock()
            && let Some(mut child) = guard.take()
        {
            terminate(&mut child);
        }
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        self.send(Command::Shutdown);
        self.kill_now();
    }
}

fn run(mut core: ServiceCore<ProcessHost>, input: mpsc::Receiver<Input>) {
    loop {
        let now = Instant::now();
        let wait = core
            .next_deadline()
            .map(|deadline| deadline.saturating_duration_since(now))
            .unwrap_or(Duration::from_secs(3600));
        match input.recv_timeout(wait) {
            Ok(Input::Command(Command::Shutdown)) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                core.shutdown();
                return;
            }
            Ok(Input::Command(command)) => core.command(command, Instant::now()),
            Ok(Input::Line { generation, line }) => {
                if generation == core.host().generation {
                    core.agent_line(&line, Instant::now());
                }
            }
            Ok(Input::Closed { generation }) => {
                if generation == core.host().generation {
                    let (code, tail) = core.host_mut().exit_details();
                    core.agent_exited(code, tail);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => core.tick(Instant::now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::companion::types::{Availability, PermissionKind, ToolState};
    use serde_json::json;

    #[derive(Default)]
    struct FakeHost {
        providers: Vec<ProviderStatus>,
        spawn_error: Option<std::io::ErrorKind>,
        spawned: Vec<(PathBuf, Vec<String>, PathBuf)>,
        written: Vec<Value>,
        kills: usize,
        updates: Vec<CompanionUpdate>,
        alive: bool,
    }

    impl Host for FakeHost {
        fn detect(&mut self, _: Option<&Path>) -> Vec<ProviderStatus> {
            self.providers.clone()
        }

        fn spawn(&mut self, command: &ProviderCommand, cwd: &Path) -> std::io::Result<()> {
            if let Some(kind) = self.spawn_error {
                return Err(std::io::Error::from(kind));
            }
            self.alive = true;
            self.spawned.push((
                command.program.clone(),
                command.args.clone(),
                cwd.to_owned(),
            ));
            Ok(())
        }

        fn write(&mut self, line: &str) -> bool {
            assert!(line.ends_with('\n'));
            self.written.push(serde_json::from_str(line).unwrap());
            self.alive
        }

        fn kill(&mut self) {
            self.alive = false;
            self.kills += 1;
        }

        fn emit(&mut self, update: CompanionUpdate) {
            self.updates.push(update);
        }

        fn fff_server(&mut self) -> Option<acp::McpServer> {
            None
        }
    }

    fn opencode_available() -> ProviderStatus {
        ProviderStatus {
            id: ProviderId::OpenCode,
            label: "OpenCode",
            command_display: "opencode acp".into(),
            executable: Some(PathBuf::from("/fake/bin/opencode")),
            availability: Availability::Available,
            detail: None,
        }
    }

    fn core() -> ServiceCore<FakeHost> {
        ServiceCore::new(
            FakeHost {
                providers: vec![opencode_available()],
                ..FakeHost::default()
            },
            "0.0.0-test",
        )
    }

    fn last_request(core: &ServiceCore<FakeHost>, method: &str) -> Value {
        core.host()
            .written
            .iter()
            .rev()
            .find(|message| message["method"] == method)
            .cloned()
            .unwrap_or_else(|| panic!("{method} should have been sent"))
    }

    fn reply(core: &mut ServiceCore<FakeHost>, method: &str, result: Value) {
        let id = last_request(core, method)["id"].clone();
        core.agent_line(
            &json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string(),
            Instant::now(),
        );
    }

    fn update(core: &mut ServiceCore<FakeHost>, update: Value) {
        core.agent_line(
            &json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": "sess", "update": update } })
                .to_string(),
            Instant::now(),
        );
    }

    fn model_config(current: &str) -> Value {
        json!([{
            "id": "model", "name": "Model", "category": "model", "type": "select",
            "currentValue": current,
            "options": [
                { "value": "openai/gpt-5.4", "name": "GPT-5.4" },
                { "value": "opencode/claude-sonnet-4-5", "name": "Claude Sonnet 4.5" },
            ],
        }])
    }

    fn start_session(core: &mut ServiceCore<FakeHost>, cwd: &Path) {
        core.command(
            Command::Connect {
                provider: None,
                cwd: cwd.to_owned(),
            },
            Instant::now(),
        );
        reply(
            core,
            "initialize",
            json!({ "protocolVersion": 1, "agentCapabilities": {} }),
        );
        reply(
            core,
            "session/new",
            json!({ "sessionId": "sess", "configOptions": model_config("opencode/claude-sonnet-4-5") }),
        );
    }

    fn payload(text: &str, active: Option<&Path>, folder: &Path) -> SendPayload {
        SendPayload {
            text: text.into(),
            active_path: active.map(Path::to_owned),
            open_folder: Some(folder.to_owned()),
            tags: Vec::new(),
            provider: None,
        }
    }

    #[test]
    fn detection_is_reported() {
        let mut core = core();
        core.command(Command::Detect, Instant::now());
        assert!(matches!(
            core.host().updates.last(),
            Some(CompanionUpdate::Providers(list)) if list[0].id == ProviderId::OpenCode
        ));
    }

    #[test]
    fn no_installed_provider_is_an_actionable_error() {
        let mut core = ServiceCore::new(FakeHost::default(), "test");
        core.command(
            Command::Send(payload("hi", None, Path::new("/tmp"))),
            Instant::now(),
        );
        assert!(matches!(
            core.host().updates.last(),
            Some(CompanionUpdate::Error(error)) if error.kind == ErrorKind::NoProvider
        ));
        assert!(core.host().spawned.is_empty());
    }

    #[test]
    fn a_missing_cli_explains_what_to_install() {
        let mut core = core();
        core.host_mut().spawn_error = Some(std::io::ErrorKind::NotFound);
        core.command(
            Command::Send(payload("hi", None, Path::new("/tmp"))),
            Instant::now(),
        );
        let error = core
            .host()
            .updates
            .iter()
            .find_map(|update| match update {
                CompanionUpdate::Error(error) => Some(error.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(error.kind, ErrorKind::MissingCli);
        assert!(error.title.contains("opencode acp"));
        assert!(error.message.contains("Install OpenCode"));
        assert!(!core.is_busy());
    }

    #[test]
    fn starts_streams_and_completes_a_turn() {
        let dir = tempfile::tempdir().unwrap();
        let active = dir.path().join("overview.md");
        std::fs::write(&active, "# Overview\nLaunch is October 14.").unwrap();
        let active = std::fs::canonicalize(active).unwrap();
        let mut core = core();
        core.command(
            Command::Send(payload("When is launch?", Some(&active), dir.path())),
            Instant::now(),
        );
        assert_eq!(
            core.host().spawned[0].0,
            PathBuf::from("/fake/bin/opencode")
        );
        assert_eq!(core.host().spawned[0].1, vec!["acp".to_string()]);
        assert!(core.is_busy());
        let init = last_request(&core, "initialize");
        assert_eq!(init["params"]["clientInfo"]["version"], "0.0.0-test");
        reply(&mut core, "initialize", json!({ "protocolVersion": 1 }));
        assert_eq!(
            last_request(&core, "session/new")["params"]["cwd"],
            dir.path().to_string_lossy().as_ref()
        );
        reply(
            &mut core,
            "session/new",
            json!({ "sessionId": "sess", "configOptions": model_config("openai/gpt-5.4") }),
        );
        let prompt = last_request(&core, "session/prompt");
        let text = prompt["params"]["prompt"][0]["text"].as_str().unwrap();
        assert!(text.contains("Launch is October 14."));
        assert!(text.contains("When is launch?"));

        update(
            &mut core,
            json!({ "sessionUpdate": "agent_thought_chunk", "content": { "type": "text", "text": "Looking" } }),
        );
        update(
            &mut core,
            json!({ "sessionUpdate": "tool_call", "toolCallId": "t1", "title": "read", "status": "completed" }),
        );
        let citation = format!("src:{}", active.display());
        update(
            &mut core,
            json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": format!("It is October 14 ({citation}).") } }),
        );
        reply(
            &mut core,
            "session/prompt",
            json!({ "stopReason": "end_turn" }),
        );

        let updates = &core.host().updates;
        let position = |predicate: &dyn Fn(&CompanionUpdate) -> bool| {
            updates.iter().position(predicate).unwrap()
        };
        let connected = position(&|u| {
            matches!(
                u,
                CompanionUpdate::Connection {
                    status: ConnectionStatus::Ready,
                    ..
                }
            )
        });
        let context = position(
            &|u| matches!(u, CompanionUpdate::Context { trace, .. } if trace.focused_count == 1),
        );
        let thinking =
            position(&|u| matches!(u, CompanionUpdate::Thinking(text) if text == "Looking"));
        let thinking_done = position(&|u| matches!(u, CompanionUpdate::ThinkingDone));
        let tool = position(
            &|u| matches!(u, CompanionUpdate::Tool(tool) if tool.state == ToolState::Completed),
        );
        let delta =
            position(&|u| matches!(u, CompanionUpdate::Delta(text) if text == "It is October 14."));
        let cited =
            position(&|u| matches!(u, CompanionUpdate::Citation(c) if c.label == "overview.md"));
        let done = position(&|u| matches!(u, CompanionUpdate::Done));
        assert!(connected < context && context < thinking && thinking < thinking_done);
        assert!(thinking_done < tool && tool < delta && delta < cited && cited < done);
        assert!(!core.is_busy());
    }

    #[test]
    fn cancelling_emits_cancelled_retires_the_agent_and_drops_late_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        core.command(
            Command::Send(payload("Summarize", None, dir.path())),
            Instant::now(),
        );
        let prompt_id = last_request(&core, "session/prompt")["id"].clone();
        core.command(Command::Cancel, Instant::now());
        assert_eq!(
            last_request(&core, "session/cancel")["params"]["sessionId"],
            "sess"
        );
        assert!(core.host().kills >= 1);
        update(
            &mut core,
            json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "late chunk" } }),
        );
        core.agent_line(
            &json!({ "jsonrpc": "2.0", "id": prompt_id, "result": { "stopReason": "cancelled" } })
                .to_string(),
            Instant::now(),
        );
        let updates = &core.host().updates;
        assert!(updates.contains(&CompanionUpdate::Cancelled));
        assert!(!updates.contains(&CompanionUpdate::Delta("late chunk".into())));
        assert!(!updates.contains(&CompanionUpdate::Done));
        // A fresh agent is already starting so the model picker comes back.
        assert_eq!(core.host().spawned.len(), 2);
        assert!(!core.is_busy());
    }

    #[test]
    fn a_second_prompt_waits_for_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        core.command(
            Command::Send(payload("One", None, dir.path())),
            Instant::now(),
        );
        core.command(
            Command::Send(payload("Two", None, dir.path())),
            Instant::now(),
        );
        assert!(core.host().updates.contains(&CompanionUpdate::Warning(
            "Wait for the current response or cancel it first.".into()
        )));
        assert_eq!(core.host().spawned.len(), 1);
    }

    #[test]
    fn unchanged_focused_content_is_sent_by_hash_within_a_session() {
        let dir = tempfile::tempdir().unwrap();
        let active = dir.path().join("active.md");
        std::fs::write(
            &active,
            format!("# Active\n{}", "important detail ".repeat(300)),
        )
        .unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        for _ in 0..2 {
            core.command(
                Command::Send(payload("Summarize this", Some(&active), dir.path())),
                Instant::now(),
            );
            reply(
                &mut core,
                "session/prompt",
                json!({ "stopReason": "end_turn" }),
            );
        }
        let prompts = core
            .host()
            .written
            .iter()
            .filter(|message| message["method"] == "session/prompt")
            .map(|message| {
                message["params"]["prompt"][0]["text"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert!(prompts[0].contains("important detail"));
        assert!(prompts[1].contains("Content unchanged from earlier in this session"));
        assert!(!prompts[1].contains("important detail"));
    }

    #[test]
    fn model_changes_persist_only_after_the_agent_confirms() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        assert!(core.host().updates.iter().any(|u| matches!(
            u,
            CompanionUpdate::Models(state) if state.current_value.as_deref() == Some("opencode/claude-sonnet-4-5")
        )));
        core.command(Command::SetModel("openai/gpt-5.4".into()), Instant::now());
        let request = last_request(&core, "session/set_config_option");
        assert_eq!(request["params"]["configId"], "model");
        assert!(
            !core
                .host()
                .updates
                .iter()
                .any(|u| matches!(u, CompanionUpdate::ModelSelected(_)))
        );
        reply(
            &mut core,
            "session/set_config_option",
            json!({ "configOptions": model_config("openai/gpt-5.4") }),
        );
        assert!(
            core.host()
                .updates
                .contains(&CompanionUpdate::ModelSelected("openai/gpt-5.4".into()))
        );

        core.command(Command::SetModel("anthropic/nope".into()), Instant::now());
        assert!(matches!(
            core.host().updates.last(),
            Some(CompanionUpdate::Error(error)) if error.message.contains("not available")
        ));
    }

    #[test]
    fn the_last_model_is_reapplied_to_a_new_session() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        core.command(
            Command::Configure(ServiceSettings {
                last_model: Some("openai/gpt-5.4".into()),
                ..ServiceSettings::default()
            }),
            Instant::now(),
        );
        core.command(
            Command::Send(payload("hi", None, dir.path())),
            Instant::now(),
        );
        reply(&mut core, "initialize", json!({}));
        reply(
            &mut core,
            "session/new",
            json!({ "sessionId": "sess", "configOptions": model_config("opencode/claude-sonnet-4-5") }),
        );
        assert_eq!(
            last_request(&core, "session/set_config_option")["params"]["value"],
            "openai/gpt-5.4"
        );
        assert!(
            core.host()
                .written
                .iter()
                .all(|m| m["method"] != "session/prompt")
        );
        reply(
            &mut core,
            "session/set_config_option",
            json!({ "configOptions": model_config("openai/gpt-5.4") }),
        );
        assert!(
            core.host()
                .written
                .iter()
                .any(|m| m["method"] == "session/prompt")
        );
    }

    #[test]
    fn authentication_errors_tell_the_reader_how_to_sign_in() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        core.command(
            Command::Send(payload("hi", None, dir.path())),
            Instant::now(),
        );
        reply(&mut core, "initialize", json!({}));
        let id = last_request(&core, "session/new")["id"].clone();
        core.agent_line(
            &json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": "Authentication required" } }).to_string(),
            Instant::now(),
        );
        let error = core
            .host()
            .updates
            .iter()
            .find_map(|u| match u {
                CompanionUpdate::Error(error) => Some(error.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(error.kind, ErrorKind::AuthRequired);
        assert!(error.message.contains("opencode auth login"));
        assert_eq!(core.host().kills, 1);
        assert!(!core.is_busy());
    }

    #[test]
    fn setup_requests_time_out() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        let start = Instant::now();
        core.command(Command::Send(payload("hi", None, dir.path())), start);
        assert!(core.next_deadline().is_some());
        core.tick(start + Duration::from_secs(29));
        assert!(core.is_busy());
        core.tick(start + REQUEST_TIMEOUT + Duration::from_millis(1));
        assert!(matches!(
            core.host().updates.last(),
            Some(CompanionUpdate::Error(error)) if error.kind == ErrorKind::Timeout && error.message.contains("initialize")
        ));
        assert!(!core.is_busy());
    }

    #[test]
    fn prompts_have_no_setup_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        core.command(
            Command::Send(payload("Take your time", None, dir.path())),
            Instant::now(),
        );
        assert_eq!(core.next_deadline(), None);
    }

    #[test]
    fn permission_requests_wait_for_the_reader() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        core.command(
            Command::Send(payload("Edit it", None, dir.path())),
            Instant::now(),
        );
        core.agent_line(
            &json!({ "jsonrpc": "2.0", "id": 41, "method": "session/request_permission", "params": {
                "sessionId": "sess",
                "toolCall": { "toolCallId": "c", "title": "Write notes.md" },
                "options": [
                    { "optionId": "yes", "name": "Allow", "kind": "allow_once" },
                    { "optionId": "no", "name": "Reject", "kind": "reject_once" },
                ],
            }}).to_string(),
            Instant::now(),
        );
        let request = core
            .host()
            .updates
            .iter()
            .find_map(|u| match u {
                CompanionUpdate::Permission(request) => Some(request.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(request.title, "Write notes.md");
        assert_eq!(request.options[1].kind, PermissionKind::RejectOnce);
        assert!(!core.host().written.iter().any(|m| m["id"] == 41));
        core.command(
            Command::Permission {
                request_key: request.request_key.clone(),
                option_id: Some("no".into()),
            },
            Instant::now(),
        );
        let answer = core.host().written.iter().find(|m| m["id"] == 41).unwrap();
        assert_eq!(answer["result"]["outcome"]["optionId"], "no");
        assert!(
            core.host()
                .updates
                .contains(&CompanionUpdate::PermissionResolved {
                    request_key: request.request_key
                })
        );
    }

    #[test]
    fn cancelling_answers_open_permission_requests() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        core.command(
            Command::Send(payload("Edit it", None, dir.path())),
            Instant::now(),
        );
        core.agent_line(
            &json!({ "jsonrpc": "2.0", "id": "p1", "method": "session/request_permission", "params": { "options": [] } }).to_string(),
            Instant::now(),
        );
        core.command(Command::Cancel, Instant::now());
        let answer = core
            .host()
            .written
            .iter()
            .find(|m| m["id"] == "p1")
            .unwrap();
        assert_eq!(answer["result"]["outcome"]["outcome"], "cancelled");
    }

    #[test]
    fn write_and_terminal_requests_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        core.agent_line(
            &json!({ "jsonrpc": "2.0", "id": 99, "method": "fs/write_text_file", "params": { "path": "/tmp/x.md", "content": "nope" } }).to_string(),
            Instant::now(),
        );
        core.agent_line(
            &json!({ "jsonrpc": "2.0", "id": 100, "method": "terminal/create", "params": {} })
                .to_string(),
            Instant::now(),
        );
        let refusal = core.host().written.iter().find(|m| m["id"] == 99).unwrap();
        assert!(
            refusal["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Refused")
        );
        assert!(
            core.host()
                .written
                .iter()
                .any(|m| m["id"] == 100 && m["error"].is_object())
        );
        assert!(core.host().updates.contains(&CompanionUpdate::Warning(
            "Refused agent request: fs/write_text_file".into()
        )));
    }

    #[test]
    fn an_agent_crash_mid_turn_surfaces_its_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        core.command(
            Command::Send(payload("hi", None, dir.path())),
            Instant::now(),
        );
        core.agent_exited(Some(3), "panic: out of tokens".into());
        assert!(matches!(
            core.host().updates.last(),
            Some(CompanionUpdate::Error(error)) if error.kind == ErrorKind::AgentExited && error.message.contains("out of tokens")
        ));
        assert!(!core.is_busy());
    }

    #[test]
    fn malformed_lines_are_warnings_not_failures() {
        let mut core = core();
        core.agent_line("{not json", Instant::now());
        assert_eq!(
            core.host().updates.last(),
            Some(&CompanionUpdate::Warning(
                "Ignored malformed ACP message".into()
            ))
        );
    }

    #[test]
    fn reset_starts_a_fresh_session() {
        let dir = tempfile::tempdir().unwrap();
        let mut core = core();
        start_session(&mut core, dir.path());
        core.command(Command::Reset, Instant::now());
        assert_eq!(core.host().kills, 1);
        assert_eq!(core.host().spawned.len(), 2);
    }
}
