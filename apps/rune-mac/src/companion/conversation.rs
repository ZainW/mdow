//! The conversation shown in the companion pane: messages and the parts they stream in. Ported
//! from the Swift app's `CompanionConversation.swift`.

use super::types::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Conversation {
    pub messages: Vec<Message>,
    pub streaming: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub tags: Vec<ContextTag>,
    /// When the turn in flight was sent, in ms since 1970, for the working line's timer.
    pub turn_started: Option<u64>,
    streaming_assistant: Option<String>,
}

impl Conversation {
    // MARK: Tags

    pub fn add_tag(&mut self, tag: ContextTag) {
        if !self.tags.iter().any(|t| t.source_id == tag.source_id) {
            self.tags.push(tag);
        }
    }

    pub fn remove_tag(&mut self, source_id: &str) {
        self.tags.retain(|t| t.source_id != source_id);
    }

    // MARK: Turns

    pub fn append_user_message(&mut self, text: &str) {
        self.messages.push(Message::new(Role::User, text, Some(MessageStatus::Complete)));
        self.error = None;
    }

    pub fn begin_request(&mut self) {
        if self.streaming {
            return;
        }
        let id = self.ensure_assistant();
        self.streaming = true;
        self.error = None;
        self.turn_started = Some(now_ms());
        self.map_assistant(&id, |m| m.status = Some(MessageStatus::Streaming));
    }

    pub fn cancel_request(&mut self) {
        self.streaming_assistant = None;
        self.streaming = false;
        self.end_turn(MessageStatus::Cancelled, None);
    }

    pub fn reset(&mut self) {
        self.streaming_assistant = None;
        self.messages.clear();
        self.streaming = false;
        self.turn_started = None;
        self.warnings.clear();
        self.error = None;
    }

    /// The change card waiting on `permission_id`, if any.
    pub fn change(&self, permission_id: &str) -> Option<&ChangePart> {
        self.messages.iter().flat_map(|m| m.parts.iter()).find_map(|p| match p {
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
        for m in &mut self.messages {
            for p in &mut m.parts {
                if let Part::Change(c) = p
                    && c.tool_call_id == tool_call_id
                {
                    c.status = status;
                    c.permission_id = permission_id.clone();
                }
            }
        }
    }

    /// Documents with an edit waiting for a decision in the latest response.
    pub fn pending_review_paths(&self) -> Vec<String> {
        let Some(last) = self.messages.last().filter(|m| m.role == Role::Assistant) else {
            return Vec::new();
        };
        let mut paths: Vec<String> = Vec::new();
        for p in &last.parts {
            if let Part::Change(c) = p
                && c.awaiting_review()
            {
                for f in &c.files {
                    if !paths.contains(&f.path) {
                        paths.push(f.path.clone());
                    }
                }
            }
        }
        paths
    }

    /// The change in the latest response that is waiting for a decision.
    pub fn pending_change(&self) -> Option<&ChangePart> {
        let last = self.messages.last().filter(|m| m.role == Role::Assistant)?;
        last.parts.iter().rev().find_map(|p| match p {
            Part::Change(c) if c.awaiting_review() => Some(c),
            _ => None,
        })
    }

    pub fn apply(&mut self, update: Update) {
        match update {
            Update::Delta(text) => {
                let id = self.ensure_assistant();
                self.streaming = true;
                self.map_assistant(&id, |m| {
                    upsert(&mut m.parts, Part::Text(text));
                    m.content = text_of(&m.parts);
                    m.status = Some(MessageStatus::Streaming);
                });
            }
            Update::Thinking(text) => {
                let id = self.ensure_assistant();
                self.streaming = true;
                self.map_assistant(&id, |m| {
                    upsert(&mut m.parts, Part::Thinking { text, done: false });
                    let now = now_ms();
                    if let Some(i) =
                        m.parts.iter().rposition(|p| matches!(p, Part::Thinking { .. }))
                    {
                        m.thinking.entry(i).or_insert((now, now)).1 = now;
                    }
                    m.status = Some(MessageStatus::Streaming);
                });
            }
            Update::ThinkingDone => {
                let Some(id) = self.streaming_assistant.clone() else { return };
                self.map_assistant(&id, |m| {
                    for p in &mut m.parts {
                        if let Part::Thinking { done, .. } = p {
                            *done = true;
                        }
                    }
                });
            }
            Update::Tool(tool) => {
                let id = self.ensure_assistant();
                self.streaming = true;
                self.map_assistant(&id, |m| {
                    upsert(&mut m.parts, Part::Tool(tool));
                    m.status = Some(MessageStatus::Streaming);
                });
            }
            Update::Change { tool_call_id, permission_id, status, files, error } => {
                let id = self.ensure_assistant();
                self.streaming = true;
                let part = ChangePart {
                    tool_call_id,
                    permission_id,
                    status,
                    files: files.unwrap_or_default(),
                    error,
                };
                self.map_assistant(&id, |m| {
                    upsert(&mut m.parts, Part::Change(part));
                    m.status = Some(MessageStatus::Streaming);
                });
            }
            Update::Status(text) => {
                let id = self.ensure_assistant();
                self.map_assistant(&id, |m| upsert(&mut m.parts, Part::Status(text)));
            }
            Update::Citation(citation) => {
                let Some(id) = self.streaming_assistant.clone() else { return };
                self.map_assistant(&id, |m| {
                    if !m.citations.iter().any(|c| c.source_id == citation.source_id) {
                        m.citations.push(citation);
                    }
                });
            }
            Update::Warning(text) => self.warnings.push(text),
            Update::Error(text) => {
                self.streaming_assistant = None;
                self.streaming = false;
                self.error = Some(text);
                self.end_turn(MessageStatus::Error, None);
            }
            Update::Done(message_id) => {
                self.streaming_assistant = None;
                self.streaming = false;
                self.end_turn(MessageStatus::Complete, Some(&message_id));
            }
            Update::Cancelled(message_id) => {
                self.streaming_assistant = None;
                self.streaming = false;
                self.end_turn(MessageStatus::Cancelled, Some(&message_id));
            }
        }
    }

    // MARK: Helpers

    fn ensure_assistant(&mut self) -> String {
        if let Some(id) = &self.streaming_assistant {
            return id.clone();
        }
        let message = Message::new(Role::Assistant, "", Some(MessageStatus::Streaming));
        let id = message.id.clone();
        self.streaming_assistant = Some(id.clone());
        self.messages.push(message);
        id
    }

    fn map_assistant(&mut self, id: &str, f: impl FnOnce(&mut Message)) {
        if let Some(m) = self.messages.iter_mut().find(|m| m.id == id) {
            f(m);
        }
    }

    fn end_turn(&mut self, status: MessageStatus, message_id: Option<&str>) {
        self.turn_started = None;
        for m in &mut self.messages {
            if Some(m.id.as_str()) == message_id || m.status == Some(MessageStatus::Streaming) {
                m.status = Some(status);
                settle(&mut m.parts);
            }
        }
    }
}

pub fn text_of(parts: &[Part]) -> String {
    parts
        .iter()
        .filter_map(|p| if let Part::Text(t) = p { Some(t.as_str()) } else { None })
        .collect()
}

pub fn upsert(parts: &mut Vec<Part>, part: Part) {
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
            let index = parts.iter().rposition(|p| matches!(p, Part::Thinking { .. }));
            if let Some(i) = index
                && let Part::Thinking { text: existing, done: existing_done } = &mut parts[i]
                && !*existing_done
            {
                existing.push_str(&text);
                *existing_done = done;
            } else {
                parts.push(Part::Thinking { text, done });
            }
        }
        Part::Tool(tool) => {
            let existing = parts.iter_mut().find_map(|p| match p {
                Part::Tool(t) if t.tool_call_id == tool.tool_call_id => Some(t),
                _ => None,
            });
            match existing {
                Some(e) => {
                    let mut merged = tool;
                    merged.input = merged.input.or_else(|| e.input.take());
                    merged.output = merged.output.or_else(|| e.output.take());
                    merged.error = merged.error.or_else(|| e.error.take());
                    *e = merged;
                }
                None => parts.push(Part::Tool(tool)),
            }
        }
        Part::Change(change) => {
            let existing = parts.iter_mut().find_map(|p| match p {
                Part::Change(c) if c.tool_call_id == change.tool_call_id => Some(c),
                _ => None,
            });
            match existing {
                Some(e) => {
                    e.status = change.status;
                    // The review id only means something while the change waits for a decision.
                    e.permission_id = if change.status == ChangeStatus::Pending {
                        change.permission_id.or(e.permission_id.take())
                    } else {
                        None
                    };
                    if !change.files.is_empty() {
                        e.files = change.files;
                    }
                    e.error = change.error;
                }
                None => parts.push(Part::Change(change)),
            }
        }
        Part::Status(_) => parts.push(part),
    }
}

/// Settles parts left open when a turn ends early: reasoning closes, reviews expire.
pub fn settle(parts: &mut [Part]) {
    for p in parts {
        match p {
            Part::Thinking { done, .. } => *done = true,
            Part::Change(c) if c.status == ChangeStatus::Pending => {
                c.status = ChangeStatus::Rejected;
                c.permission_id = None;
            }
            Part::Tool(t) if matches!(t.state, ToolState::Pending | ToolState::Running) => {
                t.state = ToolState::Cancelled
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = "--- notes.md\n+++ notes.md\n@@ -1 +1 @@\n-teh\n+the\n";

    fn file() -> FileChange {
        FileChange {
            path: "/docs/a.md".into(),
            display_path: "a.md".into(),
            patch: PATCH.into(),
            additions: 1,
            deletions: 1,
            status: FileStatus::Modified,
        }
    }

    fn pending(tool: &str, permission: &str) -> Update {
        Update::Change {
            tool_call_id: tool.into(),
            permission_id: Some(permission.into()),
            status: ChangeStatus::Pending,
            files: Some(vec![file()]),
            error: None,
        }
    }

    #[test]
    fn streams_text_into_one_assistant_message() {
        let mut c = Conversation::default();
        c.append_user_message("Hi");
        c.begin_request();
        c.apply(Update::Delta("Hello".into()));
        c.apply(Update::Delta(", there".into()));
        assert_eq!(c.messages.len(), 2);
        assert_eq!(c.messages[1].content, "Hello, there");
        assert!(c.streaming);
        c.apply(Update::Done("x".into()));
        assert!(!c.streaming);
        assert_eq!(c.messages[1].status, Some(MessageStatus::Complete));
    }

    #[test]
    fn merges_reasoning_and_tool_updates() {
        let mut c = Conversation::default();
        c.apply(Update::Thinking("A".into()));
        c.apply(Update::Thinking("B".into()));
        c.apply(Update::ThinkingDone);
        c.apply(Update::Thinking("C".into()));
        c.apply(Update::Tool(
            ToolPart::new("t", "Read", ToolState::Running).input(Some("{}".into())),
        ));
        c.apply(Update::Tool(
            ToolPart::new("t", "Read a.md", ToolState::Completed).output(Some("ok".into())),
        ));
        let parts = &c.messages[0].parts;
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], Part::Thinking { text: "AB\n\nC".into(), done: false });
        assert_eq!(
            parts[1],
            Part::Tool(
                ToolPart::new("t", "Read a.md", ToolState::Completed)
                    .input(Some("{}".into()))
                    .output(Some("ok".into()))
            )
        );
    }

    #[test]
    fn tracks_pending_reviews_and_settles_them_when_the_turn_ends() {
        let mut c = Conversation::default();
        c.apply(pending("c", "per_1"));
        assert_eq!(c.pending_review_paths(), vec!["/docs/a.md".to_string()]);
        assert_eq!(c.change("per_1").map(|c| c.tool_call_id.as_str()), Some("c"));

        // Accepting hides the buttons until OpenCode confirms.
        c.mark_reviewed("c", ChangeStatus::Pending, None);
        assert!(c.pending_review_paths().is_empty());
        assert!(c.pending_change().is_none());
        c.apply(Update::change("c", ChangeStatus::Applied));
        let Part::Change(applied) = &c.messages[0].parts[0] else { panic!("expected a change") };
        assert_eq!(applied.status, ChangeStatus::Applied);
        assert_eq!(applied.files, vec![file()]);

        c.apply(pending("d", "per_2"));
        c.apply(Update::Tool(ToolPart::new("t", "Read", ToolState::Running)));
        c.cancel_request();
        let parts = &c.messages[0].parts;
        assert_eq!(
            parts[1],
            Part::Change(ChangePart {
                tool_call_id: "d".into(),
                permission_id: None,
                status: ChangeStatus::Rejected,
                files: vec![file()],
                error: None
            })
        );
        assert_eq!(parts[2], Part::Tool(ToolPart::new("t", "Read", ToolState::Cancelled)));
        assert_eq!(c.messages[0].status, Some(MessageStatus::Cancelled));
    }

    #[test]
    fn errors_end_the_turn_and_citations_are_deduplicated() {
        let mut c = Conversation::default();
        c.apply(Update::Delta("x".into()));
        let citation =
            Citation { source_id: "read:/a.md".into(), path: "/a.md".into(), label: "a.md".into() };
        c.apply(Update::Citation(citation.clone()));
        c.apply(Update::Citation(citation));
        assert_eq!(c.messages[0].citations.len(), 1);
        c.apply(Update::Error("Sign in".into()));
        assert_eq!(c.error.as_deref(), Some("Sign in"));
        assert_eq!(c.messages[0].status, Some(MessageStatus::Error));
        // The next turn starts a fresh assistant message.
        c.apply(Update::Delta("y".into()));
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
