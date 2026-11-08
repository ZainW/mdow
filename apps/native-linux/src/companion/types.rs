//! The companion's data model.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeAvailability {
    Available,
    Missing,
    Outdated,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub availability: RuntimeAvailability,
    /// `opencode --version`, when the binary was found.
    pub version: Option<String>,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelOption {
    /// `providerID/modelID`, the form OpenCode uses everywhere.
    pub value: String,
    pub name: String,
    pub provider_id: String,
    pub provider_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelState {
    pub options: Vec<ModelOption>,
    pub current_value: Option<String>,
    pub stale: bool,
    pub unavailable_reason: Option<String>,
}

impl ModelState {
    pub fn loading() -> ModelState {
        ModelState {
            options: Vec::new(),
            current_value: None,
            stale: true,
            unavailable_reason: Some("Starting OpenCode…".to_string()),
        }
    }

    pub fn current(&self) -> Option<&ModelOption> {
        let value = self.current_value.as_ref()?;
        self.options.iter().find(|o| &o.value == value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextTag {
    pub path: String,
    pub source_id: String,
}

impl ContextTag {
    pub fn new(path: &str) -> ContextTag {
        ContextTag { path: path.to_string(), source_id: format!("tag:{path}") }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    pub source_id: String,
    pub path: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolState {
    Pending,
    Running,
    Completed,
    Error,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    /// Absolute path of the changed document.
    pub path: String,
    /// Path relative to the session folder, for display.
    pub display_path: String,
    /// Unified diff of the change.
    pub patch: String,
    pub additions: i64,
    pub deletions: i64,
    pub status: FileStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeStatus {
    Pending,
    Applied,
    Rejected,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolPart {
    pub tool_call_id: String,
    pub name: String,
    pub state: ToolState,
    pub input: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
}

impl ToolPart {
    pub fn new(tool_call_id: &str, name: &str, state: ToolState) -> ToolPart {
        ToolPart {
            tool_call_id: tool_call_id.to_string(),
            name: name.to_string(),
            state,
            input: None,
            output: None,
            error: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangePart {
    pub tool_call_id: String,
    /// Set while OpenCode waits for the user's decision.
    pub permission_id: Option<String>,
    pub status: ChangeStatus,
    pub files: Vec<FileChange>,
    pub error: Option<String>,
}

impl ChangePart {
    /// OpenCode is waiting for the user to accept or decline.
    pub fn awaiting_review(&self) -> bool {
        self.status == ChangeStatus::Pending && self.permission_id.is_some()
    }

    /// Accepted, and OpenCode has not confirmed the write yet.
    pub fn applying(&self) -> bool {
        self.status == ChangeStatus::Pending && self.permission_id.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Text(String),
    Thinking { text: String, done: bool },
    Tool(ToolPart),
    Change(ChangePart),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageStatus {
    Streaming,
    Complete,
    Error,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub parts: Vec<Part>,
    pub citations: Vec<Citation>,
    pub status: Option<MessageStatus>,
}

/// A process-unique id for a message or turn.
pub fn new_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    format!("m{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// What the service tells the conversation as a turn streams in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Update {
    Delta(String),
    Thinking(String),
    ThinkingDone,
    Tool(ToolPart),
    Change {
        tool_call_id: String,
        permission_id: Option<String>,
        status: ChangeStatus,
        files: Option<Vec<FileChange>>,
        error: Option<String>,
    },
    Citation(Citation),
    Warning(String),
    Error(String),
    Done(String),
    Cancelled(String),
}

impl Update {
    pub fn change(tool_call_id: &str, status: ChangeStatus) -> Update {
        Update::Change {
            tool_call_id: tool_call_id.to_string(),
            permission_id: None,
            status,
            files: None,
            error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionDecision {
    Approve,
    Reject,
}
