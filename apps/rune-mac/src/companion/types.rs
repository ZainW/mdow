//! The companion's data model, ported from the Swift app's `CompanionTypes.swift`.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Available,
    Missing,
    Outdated,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub availability: Availability,
    /// `opencode --version`, when the binary was found.
    pub version: Option<String>,
    pub detail: Option<String>,
}

impl RuntimeStatus {
    pub fn new(
        availability: Availability,
        version: Option<String>,
        detail: Option<String>,
    ) -> RuntimeStatus {
        RuntimeStatus { availability, version, detail }
    }
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
    pub current: Option<String>,
    pub stale: bool,
    pub unavailable_reason: Option<String>,
}

impl ModelState {
    pub fn loading() -> ModelState {
        ModelState {
            options: Vec::new(),
            current: None,
            stale: true,
            unavailable_reason: Some("Starting OpenCode…".into()),
        }
    }

    pub fn unavailable(reason: Option<String>) -> ModelState {
        ModelState { options: Vec::new(), current: None, stale: true, unavailable_reason: reason }
    }

    pub fn current_option(&self) -> Option<&ModelOption> {
        let value = self.current.as_ref()?;
        self.options.iter().find(|o| &o.value == value)
    }

    /// Providers in the order their models are listed.
    pub fn providers(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for o in &self.options {
            if !seen.contains(&o.provider_name) {
                seen.push(o.provider_name.clone());
            }
        }
        seen
    }
}

impl Default for ModelState {
    fn default() -> Self {
        ModelState::loading()
    }
}

/// A document attached to the message with @.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextTag {
    pub path: String,
    pub source_id: String,
}

impl ContextTag {
    pub fn new(path: impl Into<String>) -> ContextTag {
        let path = path.into();
        ContextTag { source_id: format!("tag:{path}"), path }
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
    pub fn new(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        state: ToolState,
    ) -> ToolPart {
        ToolPart {
            tool_call_id: tool_call_id.into(),
            name: name.into(),
            state,
            input: None,
            output: None,
            error: None,
        }
    }
    pub fn input(mut self, v: Option<String>) -> ToolPart {
        self.input = v;
        self
    }
    pub fn output(mut self, v: Option<String>) -> ToolPart {
        self.output = v;
        self
    }
    pub fn error(mut self, v: Option<String>) -> ToolPart {
        self.error = v;
        self
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
    Status(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
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
    pub role: Role,
    pub content: String,
    pub parts: Vec<Part>,
    pub citations: Vec<Citation>,
    pub status: Option<MessageStatus>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A fresh id for a message or a turn.
pub fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>, status: Option<MessageStatus>) -> Message {
        let content = content.into();
        let parts = if content.is_empty() { Vec::new() } else { vec![Part::Text(content.clone())] };
        Message { id: new_id("msg"), role, content, parts, citations: Vec::new(), status }
    }
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
    /// A status line in the reply; OpenCode 2 sends none yet.
    #[allow(dead_code)]
    Status(String),
    Citation(Citation),
    Warning(String),
    Error(String),
    Done(String),
    Cancelled(String),
}

impl Update {
    /// A change update with only a status, the common case.
    pub fn change(tool_call_id: impl Into<String>, status: ChangeStatus) -> Update {
        Update::Change {
            tool_call_id: tool_call_id.into(),
            permission_id: None,
            status,
            files: None,
            error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Approve,
    Reject,
}
