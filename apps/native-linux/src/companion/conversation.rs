//! The conversation shown in the companion: messages and the parts they stream in.

use super::types::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Conversation {
    pub messages: Vec<Message>,
    pub streaming: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub tags: Vec<ContextTag>,
    streaming_assistant_id: Option<String>,
}

impl Conversation {
    pub fn add_tag(&mut self, tag: ContextTag) {
        if !self.tags.iter().any(|t| t.source_id == tag.source_id) {
            self.tags.push(tag);
        }
    }

    pub fn remove_tag(&mut self, source_id: &str) {
        self.tags.retain(|t| t.source_id != source_id);
    }

    pub fn append_user_message(&mut self, text: &str) {
        self.messages.push(Message {
            id: new_id(),
            role: MessageRole::User,
            content: text.to_string(),
            parts: vec![Part::Text(text.to_string())],
            citations: Vec::new(),
            status: Some(MessageStatus::Complete),
        });
        self.error = None;
    }

    pub fn begin_request(&mut self) {
        if self.streaming {
            return;
        }
        let id = self.ensure_assistant();
        self.streaming = true;
        self.error = None;
        self.map_assistant(&id, |m| m.status = Some(MessageStatus::Streaming));
    }

    pub fn cancel_request(&mut self) {
        self.streaming_assistant_id = None;
        self.streaming = false;
        self.end_turn(MessageStatus::Cancelled, None);
    }

    pub fn reset(&mut self) {
        self.streaming_assistant_id = None;
        self.messages.clear();
        self.streaming = false;
        self.warnings.clear();
        self.error = None;
    }

    /// The change card waiting on `permission_id`, if any.
    pub fn change(&self, permission_id: &str) -> Option<&ChangePart> {
        self.messages.iter().flat_map(|m| &m.parts).find_map(|part| match part {
            Part::Change(c) if c.permission_id.as_deref() == Some(permission_id) => Some(c),
            _ => None,
        })
    }

    /// Hides the review buttons while a decision is sent, or restores them when sending fails.
    /// OpenCode confirms with applied or rejected events.
    pub fn mark_reviewed(
        &mut self,
        tool_call_id: &str,
        status: ChangeStatus,
        permission_id: Option<String>,
    ) {
        for part in self.messages.iter_mut().flat_map(|m| &mut m.parts) {
            if let Part::Change(change) = part
                && change.tool_call_id == tool_call_id
            {
                change.status = status;
                change.permission_id = permission_id.clone();
            }
        }
    }

    /// The change in the latest response that is waiting for a decision.
    pub fn pending_change(&self) -> Option<&ChangePart> {
        let last = self.messages.last().filter(|m| m.role == MessageRole::Assistant)?;
        last.parts.iter().rev().find_map(|part| match part {
            Part::Change(c) if c.awaiting_review() => Some(c),
            _ => None,
        })
    }

    pub fn apply(&mut self, update: Update) {
        match update {
            Update::Delta(text) => {
                self.upsert_streaming(Part::Text(text));
                let id = self.ensure_assistant();
                self.map_assistant(&id, |m| m.content = text_of(&m.parts));
            }
            Update::Thinking(text) => self.upsert_streaming(Part::Thinking { text, done: false }),
            Update::ThinkingDone => {
                let Some(id) = self.streaming_assistant_id.clone() else { return };
                self.map_assistant(&id, |m| {
                    for part in &mut m.parts {
                        if let Part::Thinking { done, .. } = part {
                            *done = true;
                        }
                    }
                });
            }
            Update::Tool(tool) => self.upsert_streaming(Part::Tool(tool)),
            Update::Change { tool_call_id, permission_id, status, files, error } => {
                self.upsert_streaming(Part::Change(ChangePart {
                    tool_call_id,
                    permission_id,
                    status,
                    files: files.unwrap_or_default(),
                    error,
                }));
            }
            Update::Citation(citation) => {
                let Some(id) = self.streaming_assistant_id.clone() else { return };
                self.map_assistant(&id, |m| {
                    if !m.citations.iter().any(|c| c.source_id == citation.source_id) {
                        m.citations.push(citation);
                    }
                });
            }
            Update::Warning(text) => self.warnings.push(text),
            Update::Error(text) => {
                self.streaming_assistant_id = None;
                self.streaming = false;
                self.error = Some(text);
                self.end_turn(MessageStatus::Error, None);
            }
            Update::Done(id) => {
                self.streaming_assistant_id = None;
                self.streaming = false;
                self.end_turn(MessageStatus::Complete, Some(&id));
            }
            Update::Cancelled(id) => {
                self.streaming_assistant_id = None;
                self.streaming = false;
                self.end_turn(MessageStatus::Cancelled, Some(&id));
            }
        }
    }

    fn upsert_streaming(&mut self, part: Part) {
        let id = self.ensure_assistant();
        self.streaming = true;
        self.map_assistant(&id, |m| {
            upsert(&mut m.parts, part);
            m.status = Some(MessageStatus::Streaming);
        });
    }

    fn ensure_assistant(&mut self) -> String {
        if let Some(id) = &self.streaming_assistant_id {
            return id.clone();
        }
        let id = new_id();
        self.streaming_assistant_id = Some(id.clone());
        self.messages.push(Message {
            id: id.clone(),
            role: MessageRole::Assistant,
            content: String::new(),
            parts: Vec::new(),
            citations: Vec::new(),
            status: Some(MessageStatus::Streaming),
        });
        id
    }

    fn map_assistant(&mut self, id: &str, change: impl FnOnce(&mut Message)) {
        if let Some(message) = self.messages.iter_mut().find(|m| m.id == id) {
            change(message);
        }
    }

    fn end_turn(&mut self, status: MessageStatus, message_id: Option<&str>) {
        for message in &mut self.messages {
            if Some(message.id.as_str()) == message_id
                || message.status == Some(MessageStatus::Streaming)
            {
                message.status = Some(status);
                settle(&mut message.parts);
            }
        }
    }
}

fn text_of(parts: &[Part]) -> String {
    parts
        .iter()
        .filter_map(|p| match p {
            Part::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn upsert(parts: &mut Vec<Part>, part: Part) {
    match part {
        Part::Text(text) => {
            if let Some(Part::Text(previous)) = parts.last_mut() {
                previous.push_str(&text);
            } else {
                parts.push(Part::Text(text));
            }
        }
        Part::Thinking { text, done } => {
            // Reasoning that resumes right after earlier reasoning reads as one block.
            if let Some(Part::Thinking { text: previous, done: previous_done }) = parts.last_mut()
                && *previous_done
            {
                *previous = format!("{previous}\n\n{text}");
                *previous_done = done;
                return;
            }
            let open = parts.iter_mut().rev().find_map(|p| match p {
                Part::Thinking { text, done: false } => Some(text),
                _ => None,
            });
            match open {
                Some(existing) => existing.push_str(&text),
                None => parts.push(Part::Thinking { text, done }),
            }
        }
        Part::Tool(tool) => {
            let existing = parts.iter_mut().find_map(|p| match p {
                Part::Tool(e) if e.tool_call_id == tool.tool_call_id => Some(e),
                _ => None,
            });
            match existing {
                Some(existing) => {
                    let input = tool.input.clone().or_else(|| existing.input.take());
                    let output = tool.output.clone().or_else(|| existing.output.take());
                    let error = tool.error.clone().or_else(|| existing.error.take());
                    *existing = ToolPart { input, output, error, ..tool };
                }
                None => parts.push(Part::Tool(tool)),
            }
        }
        Part::Change(change) => {
            let existing = parts.iter_mut().find_map(|p| match p {
                Part::Change(e) if e.tool_call_id == change.tool_call_id => Some(e),
                _ => None,
            });
            match existing {
                Some(existing) => {
                    existing.status = change.status;
                    // The review id only means something while the change waits for a decision.
                    existing.permission_id = if change.status == ChangeStatus::Pending {
                        change.permission_id.or_else(|| existing.permission_id.take())
                    } else {
                        None
                    };
                    if !change.files.is_empty() {
                        existing.files = change.files;
                    }
                    existing.error = change.error;
                }
                None => parts.push(Part::Change(change)),
            }
        }
    }
}

/// Settles parts left open when a turn ends early: reasoning closes, reviews expire.
fn settle(parts: &mut [Part]) {
    for part in parts {
        match part {
            Part::Thinking { done, .. } => *done = true,
            Part::Change(change) if change.status == ChangeStatus::Pending => {
                change.status = ChangeStatus::Rejected;
                change.permission_id = None;
            }
            Part::Tool(tool) if matches!(tool.state, ToolState::Pending | ToolState::Running) => {
                tool.state = ToolState::Cancelled;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = "--- notes.md\n+++ notes.md\n@@ -1 +1 @@\n-teh\n+the\n";

    #[test]
    fn streams_text_into_one_assistant_message() {
        let mut c = Conversation::default();
        c.append_user_message("Hi");
        c.begin_request();
        c.apply(Update::Delta("Hello".to_string()));
        c.apply(Update::Delta(", there".to_string()));
        assert_eq!(c.messages.len(), 2);
        assert_eq!(c.messages[1].content, "Hello, there");
        assert!(c.streaming);
        c.apply(Update::Done("x".to_string()));
        assert!(!c.streaming);
        assert_eq!(c.messages[1].status, Some(MessageStatus::Complete));
    }

    #[test]
    fn merges_reasoning_and_tool_updates() {
        let mut c = Conversation::default();
        c.apply(Update::Thinking("A".to_string()));
        c.apply(Update::Thinking("B".to_string()));
        c.apply(Update::ThinkingDone);
        c.apply(Update::Thinking("C".to_string()));
        let mut running = ToolPart::new("t", "Read", ToolState::Running);
        running.input = Some("{}".to_string());
        c.apply(Update::Tool(running));
        let mut completed = ToolPart::new("t", "Read a.md", ToolState::Completed);
        completed.output = Some("ok".to_string());
        c.apply(Update::Tool(completed));
        let parts = &c.messages[0].parts;
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], Part::Thinking { text: "AB\n\nC".to_string(), done: false });
        let mut merged = ToolPart::new("t", "Read a.md", ToolState::Completed);
        merged.input = Some("{}".to_string());
        merged.output = Some("ok".to_string());
        assert_eq!(parts[1], Part::Tool(merged));
    }

    #[test]
    fn tracks_pending_reviews_and_settles_them_when_the_turn_ends() {
        let mut c = Conversation::default();
        let file = FileChange {
            path: "/docs/a.md".to_string(),
            display_path: "a.md".to_string(),
            patch: PATCH.to_string(),
            additions: 1,
            deletions: 1,
            status: FileStatus::Modified,
        };
        let pending = |id: &str, permission: &str| Update::Change {
            tool_call_id: id.to_string(),
            permission_id: Some(permission.to_string()),
            status: ChangeStatus::Pending,
            files: Some(vec![file.clone()]),
            error: None,
        };
        c.apply(pending("c", "per_1"));
        assert!(c.pending_change().is_some());
        assert_eq!(c.change("per_1").map(|p| p.tool_call_id.as_str()), Some("c"));

        // Accepting hides the buttons until OpenCode confirms.
        c.mark_reviewed("c", ChangeStatus::Pending, None);
        assert!(c.pending_change().is_none());
        c.apply(Update::change("c", ChangeStatus::Applied));
        let Part::Change(applied) = &c.messages[0].parts[0] else { panic!("expected a change") };
        assert_eq!(applied.status, ChangeStatus::Applied);
        assert_eq!(applied.files, std::slice::from_ref(&file));

        c.apply(pending("d", "per_2"));
        c.apply(Update::Tool(ToolPart::new("t", "Read", ToolState::Running)));
        c.cancel_request();
        let parts = &c.messages[0].parts;
        assert_eq!(
            parts[1],
            Part::Change(ChangePart {
                tool_call_id: "d".to_string(),
                permission_id: None,
                status: ChangeStatus::Rejected,
                files: vec![file.clone()],
                error: None,
            })
        );
        assert_eq!(parts[2], Part::Tool(ToolPart::new("t", "Read", ToolState::Cancelled)));
        assert_eq!(c.messages[0].status, Some(MessageStatus::Cancelled));
    }

    #[test]
    fn errors_end_the_turn_and_citations_are_deduplicated() {
        let mut c = Conversation::default();
        c.apply(Update::Delta("x".to_string()));
        let citation = Citation {
            source_id: "read:/a.md".to_string(),
            path: "/a.md".to_string(),
            label: "a.md".to_string(),
        };
        c.apply(Update::Citation(citation.clone()));
        c.apply(Update::Citation(citation));
        assert_eq!(c.messages[0].citations.len(), 1);
        c.apply(Update::Error("Sign in".to_string()));
        assert_eq!(c.error.as_deref(), Some("Sign in"));
        assert_eq!(c.messages[0].status, Some(MessageStatus::Error));
        // The next turn starts a fresh assistant message.
        c.apply(Update::Delta("y".to_string()));
        assert_eq!(c.messages.len(), 2);
    }

    #[test]
    fn tags_are_unique_and_removable() {
        let mut c = Conversation::default();
        c.add_tag(ContextTag::new("/a.md"));
        c.add_tag(ContextTag::new("/a.md"));
        assert_eq!(c.tags.len(), 1);
        c.remove_tag("tag:/a.md");
        assert!(c.tags.is_empty());
    }
}
