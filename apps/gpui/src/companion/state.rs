//! The panel's conversation model, ported from Electron's `companion-slice.ts`.

use super::types::{
    Citation, CompanionError, CompanionUpdate, ConnectionStatus, ContextTrace, ModelState,
    PermissionRequest, ProviderId, ProviderStatus, ToolState, ToolUpdate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    Streaming,
    Complete,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPart {
    pub tool_call_id: String,
    pub name: String,
    pub state: ToolState,
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAnswer {
    Allowed,
    Denied,
    /// The turn ended (cancelled or finished) before the reader answered.
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionPart {
    pub request: PermissionRequest,
    pub answer: Option<PermissionAnswer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Text(String),
    Thinking { text: String, done: bool },
    Tool(ToolPart),
    Permission(PermissionPart),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: u64,
    pub role: Role,
    pub parts: Vec<Part>,
    pub status: MessageStatus,
    pub citations: Vec<Citation>,
    /// Bumped on every change so views can cache per-message layout work.
    pub revision: u64,
}

impl Message {
    /// The visible answer text (what "Copy" copies).
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conversation {
    pub messages: Vec<Message>,
    pub streaming: bool,
    streaming_id: Option<u64>,
    next_id: u64,
    pub error: Option<CompanionError>,
    pub warnings: Vec<String>,
    pub context_summary: String,
    pub context_trace: Option<ContextTrace>,
    pub models: ModelState,
    pub providers: Vec<ProviderStatus>,
    pub providers_loaded: bool,
    pub connection: ConnectionStatus,
    pub active_provider: Option<ProviderId>,
}

impl Default for Conversation {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            streaming: false,
            streaming_id: None,
            next_id: 1,
            error: None,
            warnings: Vec::new(),
            context_summary: String::new(),
            context_trace: None,
            models: ModelState::not_started(),
            providers: Vec::new(),
            providers_loaded: false,
            connection: ConnectionStatus::Idle,
            active_provider: None,
        }
    }
}

fn close_thinking(parts: &mut [Part]) {
    for part in parts {
        if let Part::Thinking { done, .. } = part {
            *done = true;
        }
    }
}

fn expire_permissions(parts: &mut [Part]) {
    for part in parts {
        if let Part::Permission(permission) = part
            && permission.answer.is_none()
        {
            permission.answer = Some(PermissionAnswer::Expired);
        }
    }
}

impl Conversation {
    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn push_user(&mut self, text: impl Into<String>) -> u64 {
        let id = self.alloc_id();
        self.messages.push(Message {
            id,
            role: Role::User,
            parts: vec![Part::Text(text.into())],
            status: MessageStatus::Complete,
            citations: Vec::new(),
            revision: 0,
        });
        self.error = None;
        id
    }

    fn ensure_assistant(&mut self) -> &mut Message {
        let id = match self.streaming_id {
            Some(id) if self.messages.iter().any(|message| message.id == id) => id,
            _ => {
                let id = self.alloc_id();
                self.streaming_id = Some(id);
                self.messages.push(Message {
                    id,
                    role: Role::Assistant,
                    parts: Vec::new(),
                    status: MessageStatus::Streaming,
                    citations: Vec::new(),
                    revision: 0,
                });
                id
            }
        };
        let message = self
            .messages
            .iter_mut()
            .find(|message| message.id == id)
            .expect("streaming message exists");
        message.revision += 1;
        message
    }

    /// Adds the (empty) streaming answer so "Connecting to local agent…" shows immediately.
    pub fn begin_request(&mut self) {
        if self.streaming {
            return;
        }
        self.streaming = true;
        self.error = None;
        self.ensure_assistant().status = MessageStatus::Streaming;
    }

    fn finish_streaming(&mut self, status: MessageStatus) {
        self.streaming = false;
        self.streaming_id = None;
        for message in &mut self.messages {
            if message.status == MessageStatus::Streaming {
                message.status = status;
                close_thinking(&mut message.parts);
                expire_permissions(&mut message.parts);
                message.revision += 1;
            }
        }
    }

    /// Marks a permission answered in the UI right away; the service confirms later.
    pub fn answer_permission(&mut self, request_key: &str, answer: PermissionAnswer) {
        for message in &mut self.messages {
            for part in &mut message.parts {
                if let Part::Permission(permission) = part
                    && permission.request.request_key == request_key
                    && permission.answer.is_none()
                {
                    permission.answer = Some(answer);
                    message.revision += 1;
                }
            }
        }
    }

    pub fn pending_permission(&self) -> Option<&PermissionRequest> {
        self.messages.iter().rev().find_map(|message| {
            message.parts.iter().find_map(|part| match part {
                Part::Permission(permission) if permission.answer.is_none() => {
                    Some(&permission.request)
                }
                _ => None,
            })
        })
    }

    pub fn apply(&mut self, update: CompanionUpdate) {
        match update {
            CompanionUpdate::Providers(providers) => {
                self.providers = providers;
                self.providers_loaded = true;
            }
            CompanionUpdate::Connection { status, provider } => {
                self.connection = status;
                if provider.is_some() || status == ConnectionStatus::Idle {
                    self.active_provider = provider;
                }
            }
            CompanionUpdate::Models(models) => self.models = models,
            CompanionUpdate::ModelSelected(_) => {}
            CompanionUpdate::Context {
                summary,
                warnings,
                trace,
            } => {
                self.context_summary = summary;
                self.context_trace = Some(trace);
                self.warnings = warnings;
            }
            CompanionUpdate::Delta(text) => {
                self.streaming = true;
                let message = self.ensure_assistant();
                message.status = MessageStatus::Streaming;
                match message.parts.last_mut() {
                    Some(Part::Text(existing)) => existing.push_str(&text),
                    _ => message.parts.push(Part::Text(text)),
                }
            }
            CompanionUpdate::Thinking(text) => {
                self.streaming = true;
                let message = self.ensure_assistant();
                message.status = MessageStatus::Streaming;
                let open = message.parts.iter_mut().rev().find_map(|part| match part {
                    Part::Thinking { text, done: false } => Some(text),
                    _ => None,
                });
                match open {
                    Some(existing) => existing.push_str(&text),
                    None => message.parts.push(Part::Thinking { text, done: false }),
                }
            }
            CompanionUpdate::ThinkingDone => {
                if self.streaming_id.is_some() {
                    close_thinking(&mut self.ensure_assistant().parts);
                }
            }
            CompanionUpdate::Tool(tool) => {
                self.streaming = true;
                let message = self.ensure_assistant();
                message.status = MessageStatus::Streaming;
                upsert_tool(&mut message.parts, tool);
            }
            CompanionUpdate::Citation(citation) => {
                if self.streaming_id.is_some() {
                    let message = self.ensure_assistant();
                    if !message
                        .citations
                        .iter()
                        .any(|existing| existing.source_id == citation.source_id)
                    {
                        message.citations.push(citation);
                    }
                }
            }
            CompanionUpdate::Permission(request) => {
                let message = self.ensure_assistant();
                message.parts.push(Part::Permission(PermissionPart {
                    request,
                    answer: None,
                }));
            }
            CompanionUpdate::PermissionResolved { request_key } => {
                self.answer_permission(&request_key, PermissionAnswer::Expired);
            }
            CompanionUpdate::Warning(message) => self.warnings.push(message),
            CompanionUpdate::Error(error) => {
                self.finish_streaming(MessageStatus::Error);
                // A turn that failed before any output leaves no empty bubble behind.
                self.messages.retain(|message| {
                    !(message.role == Role::Assistant
                        && message.status == MessageStatus::Error
                        && message.parts.is_empty())
                });
                self.error = Some(error);
            }
            CompanionUpdate::Done => self.finish_streaming(MessageStatus::Complete),
            CompanionUpdate::Cancelled => self.finish_streaming(MessageStatus::Cancelled),
        }
    }

    /// Stops locally without waiting for the service (Electron's `cancelCompanionRequest`).
    pub fn cancel_request(&mut self) {
        self.finish_streaming(MessageStatus::Cancelled);
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// New chat: the conversation goes, the provider list and models stay.
    pub fn reset(&mut self) {
        self.messages.clear();
        self.streaming = false;
        self.streaming_id = None;
        self.error = None;
        self.warnings.clear();
        self.context_summary.clear();
        self.context_trace = None;
    }
}

fn upsert_tool(parts: &mut Vec<Part>, tool: ToolUpdate) {
    let existing = parts.iter_mut().find_map(|part| match part {
        Part::Tool(existing) if existing.tool_call_id == tool.tool_call_id => Some(existing),
        _ => None,
    });
    match existing {
        Some(existing) => {
            if let Some(name) = tool.name {
                existing.name = name;
            }
            existing.state = tool.state;
            if tool.input.is_some() {
                existing.input = tool.input;
            }
            if tool.output.is_some() {
                existing.output = tool.output;
            }
        }
        None => parts.push(Part::Tool(ToolPart {
            tool_call_id: tool.tool_call_id,
            name: tool.name.unwrap_or_else(|| "tool".into()),
            state: tool.state,
            input: tool.input,
            output: tool.output,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::companion::types::{ErrorKind, PermissionKind, PermissionOption};
    use std::path::PathBuf;

    fn tool(id: &str, name: Option<&str>, state: ToolState) -> CompanionUpdate {
        CompanionUpdate::Tool(ToolUpdate {
            tool_call_id: id.into(),
            name: name.map(str::to_owned),
            state,
            input: None,
            output: None,
        })
    }

    #[test]
    fn streams_text_thinking_and_tools_into_one_assistant_message() {
        let mut conversation = Conversation::default();
        conversation.push_user("What is this?");
        conversation.begin_request();
        assert_eq!(conversation.messages.len(), 2);
        assert!(conversation.streaming);

        conversation.apply(CompanionUpdate::Thinking("Read".into()));
        conversation.apply(CompanionUpdate::Thinking("ing".into()));
        conversation.apply(tool("t1", Some("read"), ToolState::Pending));
        conversation.apply(tool("t1", None, ToolState::Completed));
        conversation.apply(CompanionUpdate::ThinkingDone);
        conversation.apply(CompanionUpdate::Delta("Hello ".into()));
        conversation.apply(CompanionUpdate::Delta("world".into()));
        conversation.apply(CompanionUpdate::Done);

        let answer = &conversation.messages[1];
        assert_eq!(answer.status, MessageStatus::Complete);
        assert_eq!(
            answer.parts,
            vec![
                Part::Thinking {
                    text: "Reading".into(),
                    done: true
                },
                Part::Tool(ToolPart {
                    tool_call_id: "t1".into(),
                    name: "read".into(),
                    state: ToolState::Completed,
                    input: None,
                    output: None,
                }),
                Part::Text("Hello world".into()),
            ]
        );
        assert_eq!(answer.text(), "Hello world");
        assert!(!conversation.streaming);

        conversation.push_user("Again");
        conversation.apply(CompanionUpdate::Delta("Second".into()));
        assert_eq!(conversation.messages.len(), 4);
    }

    #[test]
    fn cancel_and_error_finish_the_streaming_message() {
        let mut conversation = Conversation::default();
        conversation.push_user("Q");
        conversation.begin_request();
        conversation.apply(CompanionUpdate::Thinking("hmm".into()));
        conversation.apply(CompanionUpdate::Cancelled);
        assert_eq!(conversation.messages[1].status, MessageStatus::Cancelled);
        assert!(matches!(
            conversation.messages[1].parts[0],
            Part::Thinking { done: true, .. }
        ));

        conversation.push_user("Q2");
        conversation.begin_request();
        conversation.apply(CompanionUpdate::Error(CompanionError::new(
            ErrorKind::MissingCli,
            "Couldn't find opencode acp",
            "Install OpenCode",
        )));
        assert_eq!(
            conversation.messages.len(),
            3,
            "the empty failed answer is dropped"
        );
        assert_eq!(
            conversation.error.as_ref().unwrap().kind,
            ErrorKind::MissingCli
        );
        conversation.push_user("Q3");
        assert!(conversation.error.is_none());
    }

    #[test]
    fn citations_attach_once_to_the_streaming_answer() {
        let mut conversation = Conversation::default();
        conversation.begin_request();
        let citation = Citation {
            source_id: "src:/a.md".into(),
            path: PathBuf::from("/a.md"),
            heading_id: None,
            label: "a.md".into(),
        };
        conversation.apply(CompanionUpdate::Citation(citation.clone()));
        conversation.apply(CompanionUpdate::Citation(citation));
        assert_eq!(conversation.messages[0].citations.len(), 1);
    }

    #[test]
    fn permissions_are_answered_once_and_expire_with_the_turn() {
        let mut conversation = Conversation::default();
        conversation.begin_request();
        let request = PermissionRequest {
            request_key: "1".into(),
            title: "Write notes.md".into(),
            detail: None,
            options: vec![PermissionOption {
                option_id: "yes".into(),
                name: "Allow".into(),
                kind: PermissionKind::AllowOnce,
            }],
        };
        conversation.apply(CompanionUpdate::Permission(request.clone()));
        assert_eq!(conversation.pending_permission(), Some(&request));
        conversation.answer_permission("1", PermissionAnswer::Allowed);
        conversation.apply(CompanionUpdate::PermissionResolved {
            request_key: "1".into(),
        });
        assert!(matches!(
            &conversation.messages[0].parts[0],
            Part::Permission(PermissionPart {
                answer: Some(PermissionAnswer::Allowed),
                ..
            })
        ));
        conversation.apply(CompanionUpdate::Permission(PermissionRequest {
            request_key: "2".into(),
            ..request
        }));
        conversation.apply(CompanionUpdate::Done);
        assert_eq!(conversation.pending_permission(), None);
    }

    #[test]
    fn context_and_reset() {
        let mut conversation = Conversation::default();
        conversation.push_user("hi");
        conversation.apply(CompanionUpdate::Warning("careful".into()));
        conversation.reset();
        assert!(conversation.messages.is_empty());
        assert!(conversation.warnings.is_empty());
        assert!(conversation.context_trace.is_none());
    }
}
