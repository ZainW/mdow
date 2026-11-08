//! The OpenCode companion: a trailing panel to chat with OpenCode about the document and its
//! folder, ported from the Swift app's `Companion` sources.
//!
//! - `types`, `conversation`, `events` and `logic` are the pure parts (and their tests).
//! - `client` speaks OpenCode 2's HTTP API, `process` finds and runs `opencode serve`.
//! - `service` owns the shared server and each window's session actor, off the UI thread.
//! - `view` draws the panel and `settings` the section in Settings.
//!
//! The window keeps a [`Companion`]: the conversation as it streams in, the composer and the
//! panel's own state. Its actor reports through a channel and the window's waker; `poll` folds
//! the reports in on the UI thread.

mod client;
mod conversation;
mod events;
mod icons;
mod logic;
mod picker;
mod process;
mod service;
mod settings;
mod types;
mod view;

pub use settings::settings_group_layered;
pub use view::panel;
pub use view::panel_key as window_key;

use conversation::Conversation;
use rune::{TextEdit, Tx};
use service::{Cmd, Handle, Msg};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use types::*;

pub struct Companion {
    pub open: bool,
    conversation: Conversation,
    composer: TextEdit,
    status: Option<RuntimeStatus>,
    models: ModelState,
    revision: u64,
    handle: Option<Handle>,
    /// Whether this pane counts as visible to the shared runtime.
    visible: bool,
    /// The folder OpenCode last worked in, for reconnecting.
    directory: Option<String>,
    /// Disclosures (reasoning, tool calls, sources) the user opened.
    expanded: HashSet<String>,
    /// Diffs shown in full (true) or cut to their preview (false), by card and file.
    diff_open: HashMap<String, bool>,
    /// The `@` query being typed, and the highlighted result.
    mention: Option<String>,
    mention_index: usize,
    /// The message whose Copy button just copied, and when.
    copied: Option<(String, Instant)>,
    /// Headless screenshots with `MDOW_COMPANION_DEMO` show a sample transcript.
    demo: bool,
    demo_filled: bool,
    /// The open model picker, its search and its highlighted row.
    picker: Option<picker::PickerFor>,
    picker_query: TextEdit,
    picker_index: usize,
    /// The panel's width as the user dragged it (clamped when drawn).
    width: f32,
}

impl Default for Companion {
    fn default() -> Self {
        Companion {
            open: false,
            conversation: Conversation::default(),
            composer: TextEdit::new(""),
            status: None,
            models: ModelState::loading(),
            revision: 0,
            handle: None,
            visible: false,
            directory: None,
            expanded: HashSet::new(),
            diff_open: HashMap::new(),
            mention: None,
            mention_index: 0,
            copied: None,
            demo: rune::is_headless() && std::env::var_os("MDOW_COMPANION_DEMO").is_some(),
            demo_filled: false,
            picker: None,
            picker_query: TextEdit::new(""),
            picker_index: 0,
            width: saved_width(),
        }
    }
}

impl Companion {
    /// Called once per window: starts this window's session actor.
    pub fn setup(&mut self, waker: rune::Waker) {
        if self.handle.is_none() {
            self.handle = Some(service::spawn(waker));
        }
    }

    /// Folds in what the actor and the shared runtime reported. True when anything shown
    /// changed.
    pub fn poll(&mut self, tx: &mut Tx) -> bool {
        let mut changed = false;
        if self.demo && !self.demo_filled {
            self.demo_filled = true;
            self.fill_demo();
            changed = true;
        }
        if let Some(h) = &self.handle {
            let msgs: Vec<Msg> = h.rx.try_iter().collect();
            for msg in msgs {
                changed = true;
                match msg {
                    Msg::Update(u) => self.conversation.apply(u),
                    Msg::Error(e) => self.conversation.error = Some(e),
                    Msg::Restore { tool_call_id, permission_id } => self
                        .conversation
                        .mark_reviewed(&tool_call_id, ChangeStatus::Pending, Some(permission_id)),
                }
            }
        }
        let revision = service::revision();
        if revision != self.revision && self.handle.is_some() && !self.demo {
            self.revision = revision;
            let (status, models) = service::snapshot();
            if status != self.status || models != self.models {
                self.status = status;
                self.models = models;
                changed = true;
            }
        }
        if self.open != self.visible && self.handle.is_some() {
            self.visible = self.open;
            service::pane_visibility_changed(self.visible);
            if self.visible && !self.is_available() {
                self.connect();
            }
        }
        if let Some((_, at)) = &self.copied {
            let left = Duration::from_millis(1500).saturating_sub(at.elapsed());
            if left.is_zero() {
                self.copied = None;
                changed = true;
            } else {
                tx.wake_after(left);
            }
        }
        changed
    }

    /// Shows or hides the panel. `doc` and `folder` are what the window shows; OpenCode works
    /// in the folder, else the document's.
    pub fn toggle(&mut self, doc: Option<&Path>, folder: Option<&Path>) {
        self.open = !self.open;
        if !self.open && self.picker == Some(picker::PickerFor::Composer) {
            self.picker = None;
        }
        self.directory = Some(logic::session_directory(doc, folder, &logic::home()));
        if self.open {
            self.composer.focused = true;
        }
    }

    /// Stops whatever runs for this window (it is closing or starting over).
    #[allow(dead_code)]
    pub fn shutdown(&mut self) {
        if self.conversation.streaming {
            self.stop();
        }
        self.new_chat();
        self.open = false;
        self.picker = None;
    }

    /// Documents with an edit waiting for review, for marking their tabs.
    #[allow(dead_code)]
    pub fn pending_review_paths(&self) -> Vec<String> {
        self.conversation.pending_review_paths()
    }

    fn is_available(&self) -> bool {
        self.status.as_ref().is_some_and(|s| s.availability == Availability::Available)
    }

    fn send_cmd(&self, cmd: Cmd) {
        if let Some(h) = &self.handle {
            h.send(cmd);
        }
    }

    /// Checks for OpenCode again and loads the models. Headless runs (snapshots, tests) never
    /// start a server.
    fn connect(&mut self) {
        if rune::is_headless() {
            return;
        }
        let directory = self.directory.clone().unwrap_or_else(logic::home);
        self.send_cmd(Cmd::Connect { directory });
    }

    /// Sends `text` with what the window shows as context.
    fn send(&mut self, text: &str, doc: Option<&Path>, folder: Option<&Path>) {
        let trimmed = text.trim();
        if trimmed.is_empty() || self.conversation.streaming {
            return;
        }
        let directory = logic::session_directory(doc, folder, &logic::home());
        self.directory = Some(directory.clone());
        self.conversation.append_user_message(trimmed);
        self.conversation.begin_request();
        let Some(h) = &self.handle else { return };
        let epoch = h.epoch.load(Ordering::SeqCst);
        h.send(Cmd::Send {
            text: trimmed.to_string(),
            active_path: doc.map(|d| d.to_string_lossy().into_owned()),
            tags: self.conversation.tags.clone(),
            directory,
            epoch,
        });
    }

    fn bump_epoch(&self) {
        if let Some(h) = &self.handle {
            h.epoch.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn stop(&mut self) {
        self.bump_epoch();
        self.conversation.cancel_request();
        self.send_cmd(Cmd::Stop);
    }

    fn new_chat(&mut self) {
        self.bump_epoch();
        self.conversation.reset();
        self.expanded.clear();
        self.diff_open.clear();
        self.send_cmd(Cmd::NewChat);
    }

    /// Accepts (`once`) or declines a suggested edit.
    fn review(&mut self, permission_id: &str, decision: Decision) {
        let Some(change) = self.conversation.change(permission_id) else { return };
        let tool_call_id = change.tool_call_id.clone();
        // Hide the buttons right away; OpenCode confirms with applied or rejected events.
        let status = if decision == Decision::Reject {
            ChangeStatus::Rejected
        } else {
            ChangeStatus::Pending
        };
        self.conversation.mark_reviewed(&tool_call_id, status, None);
        self.send_cmd(Cmd::Review {
            permission_id: permission_id.to_string(),
            tool_call_id,
            decision,
        });
    }

    fn select_model(&mut self, value: &str) {
        match service::select_model(value) {
            Ok(()) => {
                self.conversation.error = None;
                self.models.current = Some(value.to_string());
                self.send_cmd(Cmd::SwitchModel(value.to_string()));
            }
            Err(e) => self.conversation.error = Some(e),
        }
    }

    fn toggle_expanded(&mut self, id: &str) {
        if !self.expanded.remove(id) {
            self.expanded.insert(id.to_string());
        }
    }

    /// Opens the panel on the sample transcript (the `companion-demo` scene).
    pub fn demo(&mut self) {
        self.open = true;
        self.demo = true;
        self.demo_filled = true;
        self.fill_demo();
    }

    /// A transcript with every kind of part, for screenshots (`MDOW_COMPANION_DEMO=1` with the
    /// `companion` scene).
    fn fill_demo(&mut self) {
        if let Some(which) = std::env::var_os("MDOW_COMPANION_PICKER") {
            self.picker = Some(if which == "settings" {
                picker::PickerFor::Settings
            } else {
                picker::PickerFor::Composer
            });
            self.picker_query = TextEdit::new("");
            self.picker_query.focused = true;
        }
        self.status = Some(RuntimeStatus::new(Availability::Available, Some("2.2.6".into()), None));
        let option = |provider: &str, provider_name: &str, id: &str, name: &str| ModelOption {
            value: format!("{provider}/{id}"),
            name: name.into(),
            provider_id: provider.into(),
            provider_name: provider_name.into(),
        };
        self.models = ModelState {
            options: vec![
                option("opencode", "OpenCode Zen", "claude-sonnet-5-5", "Claude Sonnet 5.5"),
                option("opencode", "OpenCode Zen", "gpt-6", "GPT-6"),
                option("opencode", "OpenCode Zen", "gemini-4-pro", "Gemini 4 Pro"),
                option("openrouter", "OpenRouter", "deepseek-v4", "DeepSeek V4"),
                option("openrouter", "OpenRouter", "ling", "Ling"),
            ],
            current: Some("opencode/claude-sonnet-5-5".into()),
            stale: false,
            unavailable_reason: None,
        };
        let c = &mut self.conversation;
        c.append_user_message("Summarize this document, then fix the typo in the intro.");
        c.begin_request();
        c.apply(Update::Thinking(
            "The user wants a summary and a fix. I'll read the README first.".into(),
        ));
        c.apply(Update::ThinkingDone);
        c.apply(Update::Tool(
            ToolPart::new("t1", "Read README.md", ToolState::Completed)
                .input(Some("{\n  \"path\": \"README.md\"\n}".into()))
                .output(Some("1: # Mdow".into())),
        ));
        c.apply(Update::Citation(Citation {
            source_id: "read:/README.md".into(),
            path: "/README.md".into(),
            label: "README.md".into(),
        }));
        c.apply(Update::Delta(
            "**Mdow** is a calm reader for Markdown and HTML.\n\n- Native apps for *Mac* and *Linux*\n- Renders `mermaid` diagrams and math\n\n```sh\npnpm run dev\n```\n\nI fixed the typo below.".into(),
        ));
        let patch = "--- README.md\n+++ README.md\n@@ -1,4 +1,4 @@\n # Mdow\n \n-A quiet reader for Markdwon.\n+A quiet reader for Markdown.\n Open a folder to browse it.\n";
        c.apply(Update::Change {
            tool_call_id: "e1".into(),
            permission_id: Some("per_demo".into()),
            status: ChangeStatus::Pending,
            files: Some(vec![FileChange {
                path: "/README.md".into(),
                display_path: "README.md".into(),
                patch: patch.into(),
                additions: 1,
                deletions: 1,
                status: FileStatus::Modified,
            }]),
            error: None,
        });
    }
}

/// The window's document and folder, the companion's context.
fn context(s: &crate::app::App) -> (Option<PathBuf>, Option<PathBuf>) {
    let tab = s.tab();
    (tab.pane().path().map(Path::to_path_buf), tab.folder.clone())
}

/// The panel's width, as Swift's split item allows: 300 to 560 points, 380 by default.
pub const MIN_WIDTH: f32 = 300.0;
pub const MAX_WIDTH: f32 = 560.0;
pub const DEFAULT_WIDTH: f32 = 380.0;

/// The width every window's panel starts at: the last one the user dragged to this session.
fn saved_width() -> f32 {
    crate::prefs::get().companion_width.clamp(MIN_WIDTH, MAX_WIDTH)
}

fn save_width(width: f32) {
    crate::prefs::update(|p| p.companion_width = width.clamp(MIN_WIDTH, MAX_WIDTH));
}
