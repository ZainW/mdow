//! Plain data shared by the companion service, its state model and the panel.

use std::path::PathBuf;

/// A local agent CLI that speaks the Agent Client Protocol over stdio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    OpenCode,
    CodexAcp,
    ClaudeCode,
    Gemini,
    Custom,
}

impl ProviderId {
    pub const ALL: [Self; 5] = [
        Self::OpenCode,
        Self::CodexAcp,
        Self::ClaudeCode,
        Self::Gemini,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OpenCode => "OpenCode",
            Self::CodexAcp => "Codex",
            Self::ClaudeCode => "Claude Code",
            Self::Gemini => "Gemini CLI",
            Self::Custom => "Custom",
        }
    }

    /// How sentences name the agent ("Your custom agent needs you to sign in").
    pub fn subject(self) -> &'static str {
        match self {
            Self::Custom => "Your custom agent",
            other => other.label(),
        }
    }

    /// Short label for segmented controls.
    pub fn short_label(self) -> &'static str {
        match self {
            Self::OpenCode => "OpenCode",
            Self::CodexAcp => "Codex",
            Self::ClaudeCode => "Claude",
            Self::Gemini => "Gemini",
            Self::Custom => "Custom",
        }
    }

    /// Electron's `CompanionProviderId` strings, extended with the native-only adapters.
    pub fn wire(self) -> &'static str {
        match self {
            Self::OpenCode => "opencode",
            Self::CodexAcp => "codex-acp",
            Self::ClaudeCode => "claude-code",
            Self::Gemini => "gemini",
            Self::Custom => "custom",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.wire() == value)
    }

    /// What to run in a terminal so the agent has credentials.
    pub fn sign_in_hint(self) -> &'static str {
        match self {
            Self::OpenCode => "Run `opencode auth login` in Terminal, then try again.",
            Self::CodexAcp => "Run `codex login` in Terminal, then try again.",
            Self::ClaudeCode => "Run `claude` in Terminal and sign in with /login, then try again.",
            Self::Gemini => "Run `gemini` in Terminal and sign in, then try again.",
            Self::Custom => "Sign in with your agent's own CLI, then try again.",
        }
    }

    /// Setup copy shown when the CLI cannot be found.
    pub fn install_hint(self) -> &'static str {
        match self {
            Self::OpenCode => {
                "Install OpenCode, then retry. OpenCode Go models are configured inside OpenCode."
            }
            Self::CodexAcp => "Install a Codex ACP adapter binary first. Mdow will not run npx.",
            Self::ClaudeCode => "Install the Claude Code ACP adapter (claude-code-acp) first.",
            Self::Gemini => "Install Gemini CLI first. Mdow runs it with --experimental-acp.",
            Self::Custom => "Choose an executable that speaks ACP over stdio.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStatus {
    pub id: ProviderId,
    pub label: &'static str,
    pub command_display: String,
    /// The resolved executable when the provider is available.
    pub executable: Option<PathBuf>,
    pub availability: Availability,
    pub detail: Option<String>,
}

impl ProviderStatus {
    pub fn is_available(&self) -> bool {
        self.availability == Availability::Available
    }
}

/// The preferred provider when it is installed, otherwise the first installed one.
pub fn select_available_provider(
    providers: &[ProviderStatus],
    preferred: Option<ProviderId>,
) -> Option<ProviderId> {
    if let Some(status) = providers
        .iter()
        .find(|status| Some(status.id) == preferred)
        .filter(|status| status.is_available())
    {
        return Some(status.id);
    }
    providers
        .iter()
        .find(|status| status.is_available())
        .map(|status| status.id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOption {
    pub value: String,
    pub name: String,
    pub description: Option<String>,
    /// Picker group ("ChatGPT subscription", "OpenCode Zen", ...), from the value's prefix.
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelState {
    pub options: Vec<ModelOption>,
    pub current_value: Option<String>,
    pub stale: bool,
    pub unavailable_reason: Option<String>,
}

impl Default for ModelState {
    fn default() -> Self {
        Self::not_started()
    }
}

impl ModelState {
    pub fn not_started() -> Self {
        Self {
            options: Vec::new(),
            current_value: None,
            stale: true,
            unavailable_reason: Some("Start Companion to load models".into()),
        }
    }

    pub fn current_name(&self) -> Option<&str> {
        let current = self.current_value.as_deref()?;
        self.options
            .iter()
            .find(|option| option.value == current)
            .map(|option| option.name.as_str())
    }

    pub fn is_selectable(&self) -> bool {
        !self.stale && !self.options.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    File,
    Folder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextTag {
    pub kind: TagKind,
    pub path: PathBuf,
}

impl ContextTag {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: TagKind::File,
            path: path.into(),
        }
    }

    pub fn folder(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: TagKind::Folder,
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceReason {
    Focused,
    Attached,
    Retrieved,
}

impl TraceReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::Focused => "focused",
            Self::Attached => "attached",
            Self::Retrieved => "retrieved",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceItem {
    pub path: PathBuf,
    pub reason: TraceReason,
    pub bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    FocusedOnly,
    AdaptiveLocal,
    AdaptiveFff,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextTrace {
    pub focused_count: usize,
    pub attached_count: usize,
    pub searched_count: usize,
    pub read_range_count: usize,
    pub injected_bytes: usize,
    pub estimated_tokens: usize,
    pub retrieval_mode: RetrievalMode,
    pub items: Vec<TraceItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub source_id: String,
    pub path: PathBuf,
    pub heading_id: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolState {
    Pending,
    Running,
    Completed,
    Error,
    Cancelled,
}

impl ToolState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Error => "Error",
            Self::Cancelled => "Cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUpdate {
    pub tool_call_id: String,
    /// `None` on progress updates that do not repeat the title.
    pub name: Option<String>,
    pub state: ToolState,
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
    Other,
}

impl PermissionKind {
    pub fn from_wire(value: &str) -> Self {
        match value {
            "allow_once" => Self::AllowOnce,
            "allow_always" => Self::AllowAlways,
            "reject_once" => Self::RejectOnce,
            "reject_always" => Self::RejectAlways,
            _ => Self::Other,
        }
    }

    pub fn allows(self) -> bool {
        matches!(self, Self::AllowOnce | Self::AllowAlways)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionOption {
    pub option_id: String,
    pub name: String,
    pub kind: PermissionKind,
}

/// An agent's `session/request_permission`, awaiting the reader's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    /// Opaque key for the JSON-RPC request id.
    pub request_key: String,
    pub title: String,
    pub detail: Option<String>,
    pub options: Vec<PermissionOption>,
}

impl PermissionRequest {
    pub fn allow_option(&self) -> Option<&PermissionOption> {
        self.options
            .iter()
            .find(|option| option.kind == PermissionKind::AllowOnce)
            .or_else(|| self.options.iter().find(|option| option.kind.allows()))
    }

    pub fn reject_option(&self) -> Option<&PermissionOption> {
        self.options
            .iter()
            .find(|option| option.kind == PermissionKind::RejectOnce)
            .or_else(|| {
                self.options
                    .iter()
                    .find(|option| option.kind == PermissionKind::RejectAlways)
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// No provider is installed or configured.
    NoProvider,
    /// The configured executable could not be spawned.
    MissingCli,
    /// The agent needs the reader to sign in first.
    AuthRequired,
    /// The agent process exited or stopped answering.
    AgentExited,
    Timeout,
    /// Any other failure the agent reported.
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionError {
    pub kind: ErrorKind,
    pub title: String,
    pub message: String,
}

impl CompanionError {
    pub fn new(kind: ErrorKind, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn agent(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Agent, "The agent reported an error", message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Idle,
    Connecting,
    Ready,
}

/// Everything the service tells the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionUpdate {
    Providers(Vec<ProviderStatus>),
    Connection {
        status: ConnectionStatus,
        provider: Option<ProviderId>,
    },
    Models(ModelState),
    /// The agent confirmed a model change; the panel remembers it for the next session.
    ModelSelected(String),
    Context {
        summary: String,
        warnings: Vec<String>,
        trace: ContextTrace,
    },
    Delta(String),
    Thinking(String),
    ThinkingDone,
    Tool(ToolUpdate),
    Citation(Citation),
    Permission(PermissionRequest),
    PermissionResolved {
        request_key: String,
    },
    Warning(String),
    Error(CompanionError),
    Done,
    Cancelled,
}
