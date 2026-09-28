//! The companion panel: a resizable column at the right of the window with the agent picker,
//! the conversation, a context bar and the composer. Structure follows Electron's
//! `CompanionPanel` / `CompanionHeader` / `CompanionMessages` / `CompanionContextBar` /
//! `CompanionComposer`.

use super::{
    NewCompanionChat,
    composer::{
        Composer, ComposerEvent, MoveDown as ComposerMoveDown, MoveUp as ComposerMoveUp,
        Send as ComposerSend,
    },
    markdown::{self, HighlightCache, LinkHandler, MarkdownView},
    provider::SearchPath,
    service::{Command, SendPayload, ServiceHandle, ServiceSettings},
    settings::{self, CompanionSettings, CompanionStore},
    state::{Conversation, Message, MessageStatus, Part, PermissionAnswer, Role},
    types::{
        Availability, CompanionError, CompanionUpdate, ConnectionStatus, ContextTag, ErrorKind,
        PermissionRequest, ProviderId, RetrievalMode, TagKind, ToolState, TraceReason,
        select_available_provider,
    },
};
use crate::{
    actions::Dismiss,
    app::MdowApp,
    document::{ParsedDocument, is_supported_markdown},
    overlay::OverlayKind,
    prefs::{CompanionPrefs, PrefEdit},
    theme::{ColorScheme, Metrics, Theme},
    ui::{
        field::Cancel as FieldCancel,
        primitives::{compact_icon_button, icon, icon_button, tabular_nums},
    },
};
use gpui::{
    AnyElement, App, ClickEvent, CursorStyle, Div, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, Global, KeyDownEvent, ListAlignment, ListState, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathPromptOptions, SharedString, Stateful, Subscription, Task,
    Window, canvas, deferred, div, list, prelude::*, px, relative,
};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

const HEADER_HEIGHT: f32 = Metrics::TAB_BAR_HEIGHT;
const RESIZE_HANDLE: f32 = 6.0;
const RESIZE_STEP: f32 = 16.0;
const MENTION_LIMIT: usize = 8;
const MENTION_SCAN_LIMIT: usize = 2_000;

const SUGGESTED_PROMPTS: [&str; 4] = [
    "Summarize this document",
    "What are the key open questions?",
    "Find related notes",
    "Explain the architecture",
];

/// The custom executable, shared with Settings so its row can show the chosen path.
#[derive(Default)]
pub struct CompanionCustomCommand(pub Option<PathBuf>);

impl Global for CompanionCustomCommand {}

pub fn custom_command_label(cx: &App) -> Option<String> {
    cx.try_global::<CompanionCustomCommand>()
        .and_then(|global| global.0.as_ref())
        .map(|path| path.to_string_lossy().into_owned())
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompanionEvent {
    /// Escape: give focus back to the reader.
    FocusReader,
    Close,
    OpenPath(PathBuf),
    OpenSettings,
    Pref(PrefEdit),
}

/// What the window currently shows, for context and link resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentContext {
    pub active_path: Option<PathBuf>,
    pub workspace_root: Option<PathBuf>,
}

impl DocumentContext {
    fn cwd(&self) -> PathBuf {
        self.workspace_root.clone().unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/"))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuKind {
    Agent,
}

struct MenuRow {
    label: SharedString,
    detail: Option<SharedString>,
    selected: bool,
    enabled: bool,
    heading: bool,
    action: Option<MenuAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MenuAction {
    Provider(ProviderId),
    Model(String),
}

struct OpenMenu {
    #[allow(dead_code)]
    kind: MenuKind,
    rows: Vec<MenuRow>,
    highlighted: Option<usize>,
    focus: FocusHandle,
}

struct Mention {
    query: String,
    results: Vec<PathBuf>,
    highlighted: usize,
}

struct CachedAnswer {
    revision: u64,
    document: Arc<ParsedDocument>,
}

pub struct CompanionPanel {
    service: ServiceHandle,
    conversation: Conversation,
    composer: Entity<Composer>,
    theme: Theme,
    prefs: CompanionPrefs,
    store: CompanionStore,
    settings: CompanionSettings,
    context: DocumentContext,
    tags: Vec<ContextTag>,
    list_state: ListState,
    listed_messages: usize,
    answers: HashMap<u64, CachedAnswer>,
    highlights: HighlightCache,
    expanded: HashSet<(u64, usize)>,
    context_open: bool,
    menu: Option<OpenMenu>,
    mention: Option<Mention>,
    workspace_files: Option<(PathBuf, Arc<Vec<PathBuf>>)>,
    copied: Option<(u64, Instant)>,
    connect_requested: bool,
    last_prompt: Option<String>,
    resizing: Option<f32>,
    window_width: f32,
    /// The saved width clamped to this window.
    display_width: f32,
    focus_handle: FocusHandle,
    resize_focus: FocusHandle,
    _composer_events: Subscription,
    _updates: Task<()>,
    _quit: Subscription,
}

impl EventEmitter<CompanionEvent> for CompanionPanel {}

impl Focusable for CompanionPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn format_tokens(tokens: usize) -> String {
    if tokens < 1_000 {
        tokens.to_string()
    } else if tokens >= 10_000 {
        format!("{}k", (tokens as f32 / 1_000.0).round())
    } else {
        format!("{:.1}k", tokens as f32 / 1_000.0)
    }
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1_024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} KB", bytes as f32 / 1_024.0)
    }
}

/// Case-insensitive subsequence match on the file name; earlier and denser matches rank first.
pub fn mention_score(query: &str, name: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(name.len());
    }
    let name_lower = name.to_lowercase();
    let mut position = 0;
    let mut first = None;
    let mut last = 0;
    for ch in query.to_lowercase().chars() {
        let found = name_lower[position..].find(ch)? + position;
        first.get_or_insert(found);
        last = found;
        position = found + ch.len_utf8();
    }
    let first = first.unwrap_or(0);
    Some(first * 4 + (last - first) * 2 + name.len())
}

/// The `@query` being typed at the end of the prompt, as in Electron (`/@([\w./\\-]*)$/`).
pub fn trailing_mention(text: &str) -> Option<&str> {
    let at = text.rfind('@')?;
    let query = &text[at + 1..];
    query
        .chars()
        .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '.' | '/' | '\\' | '-'))
        .then_some(query)
}

fn collect_markdown_files(root: &Path, output: &mut Vec<PathBuf>) {
    if output.len() >= MENTION_SCAN_LIMIT {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') || name == "node_modules" {
            continue;
        }
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect_markdown_files(&path, output),
            Ok(_) if is_supported_markdown(&path) => output.push(path),
            _ => {}
        }
        if output.len() >= MENTION_SCAN_LIMIT {
            return;
        }
    }
}

impl CompanionPanel {
    pub fn new(
        prefs: CompanionPrefs,
        store: CompanionStore,
        search: Option<SearchPath>,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = store.load();
        cx.set_global(CompanionCustomCommand(settings.custom_command.clone()));
        let version =
            crate::sparkle::app_version().unwrap_or_else(|| env!("CARGO_PKG_VERSION").into());
        let (service, updates) = ServiceHandle::spawn(version, search);
        let composer = cx.new(|cx| Composer::new("Ask about these docs…", theme, cx));
        let composer_events = cx.subscribe_in(&composer, window, Self::on_composer_event);
        let updates_task = cx.spawn(async move |this, cx| {
            while let Ok(first) = updates.recv().await {
                let mut batch = vec![first];
                while let Ok(next) = updates.try_recv() {
                    batch.push(next);
                }
                if this
                    .update(cx, |this, cx| this.apply_updates(batch, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        let quit = cx.on_app_quit(|this, _| {
            this.service.kill_now();
            async {}
        });
        let panel = Self {
            service,
            conversation: Conversation::default(),
            composer,
            theme,
            prefs,
            store,
            settings,
            context: DocumentContext::default(),
            tags: Vec::new(),
            list_state: ListState::new(0, ListAlignment::Bottom, px(480.0)),
            listed_messages: 0,
            answers: HashMap::new(),
            highlights: HighlightCache::default(),
            expanded: HashSet::new(),
            context_open: false,
            menu: None,
            mention: None,
            workspace_files: None,
            copied: None,
            connect_requested: false,
            last_prompt: None,
            resizing: None,
            window_width: 0.0,
            display_width: settings::DEFAULT_PANEL_WIDTH,
            focus_handle: cx.focus_handle(),
            resize_focus: cx.focus_handle().tab_index(0).tab_stop(true),
            _composer_events: composer_events,
            _updates: updates_task,
            _quit: quit,
        };
        panel.configure();
        panel.service.send(Command::Detect);
        panel
    }

    fn service_settings(&self) -> ServiceSettings {
        ServiceSettings {
            preferred: self.prefs.provider,
            custom_command: self.settings.custom_command.clone(),
            last_model: self.settings.last_model.clone(),
        }
    }

    fn configure(&self) {
        self.service
            .send(Command::Configure(self.service_settings()));
    }

    pub fn conversation(&self) -> &Conversation {
        &self.conversation
    }

    pub fn width(&self) -> f32 {
        self.settings.panel_width
    }

    pub fn composer(&self) -> &Entity<Composer> {
        &self.composer
    }

    /// Called every frame the panel is visible.
    pub fn sync(
        &mut self,
        prefs: CompanionPrefs,
        theme: Theme,
        context: DocumentContext,
        cx: &mut Context<Self>,
    ) {
        if self.prefs != prefs {
            let provider_changed = self.prefs.provider != prefs.provider;
            self.prefs = prefs;
            self.configure();
            if provider_changed {
                self.connect(cx);
            }
        }
        if self.theme != theme {
            self.theme = theme;
            self.composer
                .update(cx, |composer, _| composer.set_theme(theme));
        }
        self.context = context;
    }

    /// The panel became visible: focus the prompt and make sure an agent is warming up.
    pub fn opened(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.composer.read(cx).focus(window);
        self.service.send(Command::Detect);
        self.connect_requested = false;
        cx.notify();
    }

    fn connect(&mut self, _: &mut Context<Self>) {
        self.connect_requested = true;
        self.service.send(Command::Connect {
            provider: self.prefs.provider,
            cwd: self.context.cwd(),
        });
    }

    fn has_available_provider(&self) -> bool {
        select_available_provider(&self.conversation.providers, self.prefs.provider).is_some()
    }

    fn apply_updates(&mut self, batch: Vec<CompanionUpdate>, cx: &mut Context<Self>) {
        for update in batch {
            match update {
                CompanionUpdate::ModelSelected(value) => {
                    self.settings.last_model = Some(value);
                    self.store.save(&self.settings);
                }
                CompanionUpdate::Providers(providers) => {
                    self.conversation
                        .apply(CompanionUpdate::Providers(providers));
                    if !self.connect_requested
                        && self.conversation.connection == ConnectionStatus::Idle
                        && self.has_available_provider()
                    {
                        self.connect(cx);
                    }
                }
                other => self.conversation.apply(other),
            }
        }
        self.sync_list();
        cx.notify();
    }

    fn sync_list(&mut self) {
        let count = self.conversation.messages.len();
        if count < self.listed_messages || self.listed_messages == 0 {
            self.list_state.reset(count);
        } else if count > self.listed_messages {
            let changed_from = self.listed_messages.saturating_sub(1);
            self.list_state
                .splice(changed_from..self.listed_messages, count - changed_from);
        } else if count > 0 {
            self.list_state.splice(count - 1..count, 1);
        }
        self.listed_messages = count;
        let scheme = self.theme.color_scheme;
        let live = self
            .conversation
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<HashSet<_>>();
        self.answers.retain(|id, _| live.contains(id));
        let finished = self
            .conversation
            .messages
            .iter()
            .filter(|message| {
                message.role == Role::Assistant && message.status != MessageStatus::Streaming
            })
            .cloned()
            .collect::<Vec<_>>();
        for message in finished {
            let document = self.answer_document(&message);
            self.highlights.prepare(&document, scheme);
        }
    }

    fn answer_document(&mut self, message: &Message) -> Arc<ParsedDocument> {
        if let Some(cached) = self.answers.get(&message.id)
            && cached.revision == message.revision
        {
            return cached.document.clone();
        }
        let document = markdown::parse(&message.text());
        self.answers.insert(
            message.id,
            CachedAnswer {
                revision: message.revision,
                document: document.clone(),
            },
        );
        document
    }

    pub fn send_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let trimmed = text.trim();
        if trimmed.is_empty() || self.conversation.streaming {
            return;
        }
        self.conversation.push_user(trimmed);
        self.conversation.begin_request();
        self.last_prompt = Some(trimmed.to_owned());
        self.menu = None;
        self.mention = None;
        self.service.send(Command::Send(SendPayload {
            text: trimmed.to_owned(),
            active_path: self.context.active_path.clone(),
            open_folder: self.context.workspace_root.clone(),
            tags: self.tags.clone(),
            provider: self.prefs.provider,
        }));
        self.sync_list();
        cx.notify();
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.conversation.streaming {
            return;
        }
        let text = self.composer.read(cx).text().to_owned();
        if text.trim().is_empty() {
            return;
        }
        self.composer
            .update(cx, |composer, cx| composer.set_text("", cx));
        self.send_text(&text, cx);
        self.composer.read(cx).focus(window);
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if !self.conversation.streaming {
            return;
        }
        self.service.send(Command::Cancel);
        self.conversation.cancel_request();
        self.sync_list();
        cx.notify();
    }

    pub fn new_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.service.send(Command::Reset);
        self.conversation.reset();
        self.tags.clear();
        self.expanded.clear();
        self.answers.clear();
        self.context_open = false;
        self.sync_list();
        self.composer
            .update(cx, |composer, cx| composer.set_text("", cx));
        self.composer.read(cx).focus(window);
        cx.notify();
    }

    fn retry(&mut self, cx: &mut Context<Self>) {
        self.conversation.clear_error();
        self.connect_requested = false;
        self.service.send(Command::Detect);
        if let Some(prompt) = self.last_prompt.clone() {
            // Resend without repeating the question bubble.
            self.conversation.begin_request();
            self.service.send(Command::Send(SendPayload {
                text: prompt,
                active_path: self.context.active_path.clone(),
                open_folder: self.context.workspace_root.clone(),
                tags: self.tags.clone(),
                provider: self.prefs.provider,
            }));
            self.sync_list();
        }
        cx.notify();
    }

    fn answer_permission(
        &mut self,
        request: &PermissionRequest,
        allow: bool,
        cx: &mut Context<Self>,
    ) {
        let option = if allow {
            request.allow_option()
        } else {
            request.reject_option()
        };
        self.conversation.answer_permission(
            &request.request_key,
            if allow {
                PermissionAnswer::Allowed
            } else {
                PermissionAnswer::Denied
            },
        );
        self.service.send(Command::Permission {
            request_key: request.request_key.clone(),
            option_id: option.map(|option| option.option_id.clone()),
        });
        self.sync_list();
        cx.notify();
    }

    pub fn choose_custom_executable(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Use Executable".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update(cx, |this, cx| this.set_custom_command(path, cx))
                    .ok();
            }
        })
        .detach();
    }

    fn set_custom_command(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.settings.custom_command = Some(path.clone());
        self.store.save(&self.settings);
        cx.set_global(CompanionCustomCommand(Some(path)));
        self.configure();
        self.service.send(Command::Detect);
        cx.emit(CompanionEvent::Pref(PrefEdit::CompanionProvider(Some(
            ProviderId::Custom,
        ))));
        cx.notify();
    }

    fn set_width(&mut self, width: f32, cx: &mut Context<Self>) {
        let width = settings::clamp_panel_width(width, self.window_width);
        if (width - self.settings.panel_width).abs() >= 0.5 {
            self.settings.panel_width = width;
            self.display_width = width;
            cx.notify();
        }
    }

    fn finish_resize(&mut self) {
        if self.resizing.take().is_some() {
            self.store.save(&self.settings);
        }
    }

    fn open_link(&mut self, target: &str, cx: &mut Context<Self>) {
        let lowered = target.to_lowercase();
        if ["http://", "https://", "mailto:"]
            .iter()
            .any(|scheme| lowered.starts_with(scheme))
        {
            let _ = open::that(target);
            return;
        }
        let target = target.split('#').next().unwrap_or(target);
        let target = target.strip_prefix("file://").unwrap_or(target);
        let path = PathBuf::from(target);
        let candidates = if path.is_absolute() {
            vec![path]
        } else {
            [
                self.context
                    .active_path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_owned),
                self.context.workspace_root.clone(),
            ]
            .into_iter()
            .flatten()
            .map(|base| base.join(&path))
            .collect()
        };
        if let Some(found) = candidates.into_iter().find(|candidate| candidate.is_file()) {
            cx.emit(CompanionEvent::OpenPath(found));
        }
    }

    fn copy_answer(&mut self, message_id: u64, cx: &mut Context<Self>) {
        let Some(message) = self
            .conversation
            .messages
            .iter()
            .find(|message| message.id == message_id)
        else {
            return;
        };
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(message.text()));
        let stamp = Instant::now();
        self.copied = Some((message_id, stamp));
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1500))
                .await;
            this.update(cx, |this, cx| {
                if this.copied.is_some_and(|(_, at)| at == stamp) {
                    this.copied = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn on_composer_event(
        &mut self,
        _: &Entity<Composer>,
        event: &ComposerEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ComposerEvent::Submitted => {
                if self.mention.as_ref().is_some_and(|m| !m.results.is_empty()) {
                    self.accept_mention(cx);
                } else {
                    self.submit(window, cx);
                }
            }
            ComposerEvent::Cancelled => {
                if self.mention.take().is_some() || self.menu.take().is_some() {
                    cx.notify();
                } else {
                    cx.emit(CompanionEvent::FocusReader);
                }
            }
            ComposerEvent::Edited => self.update_mention(cx),
        }
    }

    fn update_mention(&mut self, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).text().to_owned();
        let Some(query) = trailing_mention(&text).map(str::to_owned) else {
            if self.mention.take().is_some() {
                cx.notify();
            }
            return;
        };
        let Some(root) = self.context.workspace_root.clone() else {
            self.mention = None;
            return;
        };
        let files = match &self.workspace_files {
            Some((cached_root, files)) if *cached_root == root => files.clone(),
            _ => {
                let scan_root = root.clone();
                cx.spawn(async move |this, cx| {
                    let files = cx
                        .background_spawn(async move {
                            let mut files = Vec::new();
                            collect_markdown_files(&scan_root, &mut files);
                            files
                        })
                        .await;
                    this.update(cx, |this, cx| {
                        this.workspace_files = Some((root, Arc::new(files)));
                        this.update_mention(cx);
                    })
                    .ok();
                })
                .detach();
                Arc::new(Vec::new())
            }
        };
        let mut ranked = files
            .iter()
            .filter(|path| !self.tags.iter().any(|tag| &tag.path == *path))
            .filter_map(|path| mention_score(&query, &file_name(path)).map(|score| (score, path)))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)));
        let results = ranked
            .into_iter()
            .take(MENTION_LIMIT)
            .map(|(_, path)| path.clone())
            .collect::<Vec<_>>();
        let highlighted = self
            .mention
            .as_ref()
            .filter(|mention| mention.query == query)
            .map_or(0, |mention| mention.highlighted)
            .min(results.len().saturating_sub(1));
        self.mention = Some(Mention {
            query,
            results,
            highlighted,
        });
        cx.notify();
    }

    fn accept_mention(&mut self, cx: &mut Context<Self>) {
        let Some(mention) = self.mention.take() else {
            return;
        };
        let Some(path) = mention.results.get(mention.highlighted).cloned() else {
            return;
        };
        self.add_tag(ContextTag::file(path), cx);
        self.composer.update(cx, |composer, cx| {
            let text = composer.text().to_owned();
            if let Some(at) = text.rfind('@') {
                composer.set_text(&text[..at], cx);
            }
        });
        self.mention = None;
        cx.notify();
    }

    fn add_tag(&mut self, tag: ContextTag, cx: &mut Context<Self>) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            cx.notify();
        }
    }

    fn move_mention(&mut self, step: isize, cx: &mut Context<Self>) -> bool {
        let Some(mention) = self.mention.as_mut().filter(|m| !m.results.is_empty()) else {
            return false;
        };
        let count = mention.results.len() as isize;
        mention.highlighted = (mention.highlighted as isize + step).rem_euclid(count) as usize;
        cx.notify();
        true
    }

    fn agent_menu_rows(&self) -> Vec<MenuRow> {
        let active = self.conversation.active_provider.or_else(|| {
            select_available_provider(&self.conversation.providers, self.prefs.provider)
        });
        let mut rows = vec![MenuRow {
            label: "Agent".into(),
            detail: None,
            selected: false,
            enabled: false,
            heading: true,
            action: None,
        }];
        for status in &self.conversation.providers {
            let available = status.availability == Availability::Available;
            rows.push(MenuRow {
                label: status.label.into(),
                detail: (!available).then(|| "Not installed".into()),
                selected: Some(status.id) == active,
                enabled: available,
                heading: false,
                action: Some(MenuAction::Provider(status.id)),
            });
        }
        let models = &self.conversation.models;
        let grouped = models.is_selectable() && models.options.iter().any(|o| o.group.is_some());
        if !grouped {
            rows.push(MenuRow {
                label: "Model".into(),
                detail: None,
                selected: false,
                enabled: false,
                heading: true,
                action: None,
            });
        }
        if models.is_selectable() {
            let mut group: Option<String> = None;
            for option in &models.options {
                if option.group != group {
                    group = option.group.clone();
                    if let Some(name) = group.as_ref() {
                        rows.push(MenuRow {
                            label: name.clone().into(),
                            detail: None,
                            selected: false,
                            enabled: false,
                            heading: true,
                            action: None,
                        });
                    }
                }
                rows.push(MenuRow {
                    label: option.name.clone().into(),
                    detail: option.description.clone().map(Into::into),
                    selected: models.current_value.as_ref() == Some(&option.value),
                    enabled: true,
                    heading: false,
                    action: Some(MenuAction::Model(option.value.clone())),
                });
            }
        } else {
            rows.push(MenuRow {
                label: models
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "No models".into())
                    .into(),
                detail: None,
                selected: false,
                enabled: false,
                heading: false,
                action: None,
            });
        }
        rows
    }

    fn toggle_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.take().is_some() {
            self.composer.read(cx).focus(window);
            cx.notify();
            return;
        }
        self.service.send(Command::Detect);
        let rows = self.agent_menu_rows();
        let highlighted = rows
            .iter()
            .position(|row| row.selected && row.enabled)
            .or_else(|| rows.iter().position(|row| row.enabled));
        let focus = cx.focus_handle();
        focus.focus(window);
        self.menu = Some(OpenMenu {
            kind: MenuKind::Agent,
            rows,
            highlighted,
            focus,
        });
        cx.notify();
    }

    fn confirm_menu(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.take() else {
            return;
        };
        let action = menu
            .rows
            .get(index)
            .filter(|row| row.enabled)
            .and_then(|row| row.action.clone());
        match action {
            Some(MenuAction::Provider(provider)) => {
                cx.emit(CompanionEvent::Pref(PrefEdit::CompanionProvider(Some(
                    provider,
                ))));
                self.prefs.provider = Some(provider);
                self.configure();
                self.connect(cx);
            }
            Some(MenuAction::Model(value)) => self.service.send(Command::SetModel(value)),
            None => {}
        }
        self.composer.read(cx).focus(window);
        cx.notify();
    }

    fn move_menu(&mut self, step: isize, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.as_mut() else {
            return;
        };
        let enabled = menu
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.enabled)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if enabled.is_empty() {
            return;
        }
        let next = match menu
            .highlighted
            .and_then(|current| enabled.iter().position(|index| *index == current))
        {
            Some(position) => {
                enabled[(position as isize + step).rem_euclid(enabled.len() as isize) as usize]
            }
            None => enabled[0],
        };
        menu.highlighted = Some(next);
        cx.notify();
    }

    fn picker_label(&self) -> String {
        let provider = self.conversation.active_provider.or_else(|| {
            select_available_provider(&self.conversation.providers, self.prefs.provider)
        });
        let provider = provider.map(ProviderId::label).unwrap_or("No agent");
        match (
            self.conversation.connection,
            self.conversation.models.current_name(),
        ) {
            (ConnectionStatus::Connecting, _) => format!("{provider} · Connecting…"),
            (_, Some(model)) => format!("{provider} · {model}"),
            _ => provider.to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Rendering

fn chip(theme: Theme) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(5.0))
        .h(px(22.0))
        .px(px(8.0))
        .rounded(px(11.0))
        .bg(theme.surface_well)
        .text_size(px(11.5))
        .text_color(theme.muted_foreground)
}

fn button_base(id: impl Into<gpui::ElementId>, theme: Theme) -> Stateful<Div> {
    div()
        .id(id)
        .tab_index(0)
        .focusable()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(26.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .border_1()
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .active(|style| style.opacity(0.85))
        .focus(move |style| style.border_color(theme.primary))
}

fn quiet_button(id: impl Into<gpui::ElementId>, theme: Theme) -> Stateful<Div> {
    button_base(id, theme)
        .border_color(theme.border)
        .bg(match theme.color_scheme {
            ColorScheme::Dark => theme.muted,
            ColorScheme::Light => theme.surface_raised,
        })
        .text_color(theme.foreground)
        .hover(move |style| style.bg(theme.sidebar_accent))
}

fn primary_button(id: impl Into<gpui::ElementId>, theme: Theme) -> Stateful<Div> {
    button_base(id, theme)
        .bg(theme.foreground)
        .text_color(theme.background)
        .border_color(theme.foreground.opacity(0.0))
        .hover(|style| style.opacity(0.9))
}

impl CompanionPanel {
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let open = self.menu.is_some();
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.0))
            .h(px(HEADER_HEIGHT))
            .pl(px(12.0))
            .pr(px(6.0))
            .border_b_1()
            .border_color(theme.border_subtle)
            .child(icon(
                "icons/message-square.svg",
                theme.muted_foreground,
                14.0,
            ))
            .child(
                div()
                    .ml(px(4.0))
                    .flex_none()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(13.0))
                    .child("Companion"),
            )
            .child(div().flex_1().min_w_0())
            .child(
                div()
                    .id("companion-agent-picker")
                    .debug_selector(|| "companion-agent-picker".into())
                    .tab_index(0)
                    .focusable()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .min_w_0()
                    .max_w(px(200.0))
                    .h(px(26.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.border.opacity(0.0))
                    .when(open, |button| button.bg(theme.muted))
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
                    .focus(move |style| style.border_color(theme.primary))
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.toggle_menu(window, cx)
                        }),
                    )
                    .child(icon("icons/cpu.svg", theme.muted_foreground, 13.0))
                    .child(div().min_w_0().truncate().child(self.picker_label()))
                    .child(icon("icons/chevron-down.svg", theme.muted_foreground, 12.0)),
            )
            .child(icon_button(
                "companion-new-chat",
                "icons/square-pen.svg",
                theme,
                cx.listener(|this, _, window, cx| this.new_chat(window, cx)),
            ))
            .child(icon_button(
                "companion-close",
                "icons/x.svg",
                theme,
                cx.listener(|_, _, _, cx| cx.emit(CompanionEvent::Close)),
            ))
    }

    fn render_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.menu.as_ref()?;
        let theme = self.theme;
        let mut column = div()
            .id("companion-agent-menu")
            .debug_selector(|| "companion-agent-menu".into())
            .track_focus(&menu.focus)
            .occlude()
            .absolute()
            .top(px(HEADER_HEIGHT - 2.0))
            .right(px(8.0))
            .w(px(260.0))
            .max_h(px(420.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .p(px(4.0))
            .rounded(px(Metrics::RADIUS))
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface_raised)
            .shadow_lg()
            .text_size(px(12.0))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                if modifiers.platform || modifiers.control || modifiers.alt {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "down" => this.move_menu(1, cx),
                    "up" => this.move_menu(-1, cx),
                    "enter" | "space" => {
                        if let Some(index) = this.menu.as_ref().and_then(|menu| menu.highlighted) {
                            this.confirm_menu(index, window, cx);
                        }
                    }
                    "escape" => {
                        this.menu = None;
                        this.composer.read(cx).focus(window);
                        cx.notify();
                    }
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.menu = None;
                cx.notify();
            }));
        for (index, row) in menu.rows.iter().enumerate() {
            if row.heading {
                column = column.child(
                    div()
                        .px(px(8.0))
                        .pt(px(if index == 0 { 4.0 } else { 8.0 }))
                        .pb(px(2.0))
                        .text_size(px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child(row.label.to_uppercase()),
                );
                continue;
            }
            let highlighted = menu.highlighted == Some(index);
            let enabled = row.enabled;
            column = column.child(
                div()
                    .id(("companion-menu-row", index))
                    .debug_selector(move || format!("companion-menu-row-{index}"))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.0))
                    .min_h(px(Metrics::MENU_ROW_HEIGHT))
                    .py(px(4.0))
                    .px(px(8.0))
                    .rounded(px(Metrics::MENU_ROW_RADIUS))
                    .bg(if highlighted {
                        theme.muted
                    } else {
                        theme.muted.opacity(0.0)
                    })
                    .text_color(if enabled {
                        theme.foreground
                    } else {
                        theme.muted_foreground.opacity(0.7)
                    })
                    .when(enabled, |row| {
                        row.cursor_pointer()
                            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                if let Some(menu) = this.menu.as_mut()
                                    && menu.highlighted != Some(index)
                                {
                                    menu.highlighted = Some(index);
                                    cx.notify();
                                }
                            }))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.confirm_menu(index, window, cx)
                            }))
                    })
                    .child(div().flex_1().min_w_0().truncate().child(row.label.clone()))
                    .children(row.detail.clone().map(|detail| {
                        div()
                            .flex_none()
                            .max_w(px(120.0))
                            .truncate()
                            .text_size(px(11.0))
                            .text_color(theme.muted_foreground)
                            .child(detail)
                    }))
                    .child(div().size(px(14.0)).flex_none().when(row.selected, |slot| {
                        slot.child(icon("icons/check.svg", theme.primary, 14.0))
                    })),
            );
        }
        Some(deferred(column).with_priority(3).into_any_element())
    }

    fn render_setup(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut list = div().flex().flex_col().gap(px(8.0));
        for status in &self.conversation.providers {
            let available = status.availability == Availability::Available;
            let id = status.id;
            let selected = Some(id) == self.prefs.provider;
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .p(px(10.0))
                    .rounded(px(Metrics::RADIUS))
                    .border_1()
                    .border_color(if selected {
                        theme.primary.opacity(0.5)
                    } else {
                        theme.border_subtle
                    })
                    .bg(theme.surface_raised)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div().font_weight(FontWeight::MEDIUM).child(status.label),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .font_family(Metrics::FONT_MONO)
                                            .text_size(px(11.0))
                                            .text_color(theme.muted_foreground)
                                            .child(status.command_display.clone()),
                                    ),
                            )
                            .child(
                                chip(theme)
                                    .when(available, |chip| {
                                        chip.bg(theme.alert_tip.opacity(0.14))
                                            .text_color(theme.alert_tip)
                                    })
                                    .child(if available { "Installed" } else { "Not found" }),
                            ),
                    )
                    .children(status.detail.clone().map(|detail| {
                        div()
                            .text_size(px(11.5))
                            .text_color(theme.muted_foreground)
                            .child(detail)
                    }))
                    .when(available, |card| {
                        card.child(
                            quiet_button(("companion-use-provider", id as usize), theme)
                                .mt(px(4.0))
                                .child(format!("Use {}", status.label))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.menu = None;
                                    cx.emit(CompanionEvent::Pref(PrefEdit::CompanionProvider(
                                        Some(id),
                                    )));
                                    this.prefs.provider = Some(id);
                                    this.configure();
                                    this.connect(cx);
                                    this.composer.read(cx).focus(window);
                                })),
                        )
                    }),
            );
        }
        div()
            .id("companion-setup")
            .debug_selector(|| "companion-setup".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .gap(px(12.0))
            .p(px(12.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Connect a local agent"),
                    )
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .child(
                                "Companion talks to an ACP agent CLI already on this Mac. Mdow \
                                 never installs packages for you.",
                            ),
                    ),
            )
            .child(list)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child("Custom ACP executable"),
                    )
                    .children(self.settings.custom_command.as_ref().map(|path| {
                        div()
                            .p(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_well)
                            .font_family(Metrics::FONT_MONO)
                            .text_size(px(11.0))
                            .child(path.to_string_lossy().into_owned())
                    }))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(theme.muted_foreground)
                            .child("Choose one executable. Arguments and shell commands are not accepted."),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(
                                quiet_button("companion-choose-executable", theme)
                                    .child("Choose executable…")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.choose_custom_executable(cx)
                                    })),
                            )
                            .child(
                                quiet_button("companion-retry-detection", theme)
                                    .child(icon("icons/rotate-ccw.svg", theme.muted_foreground, 12.0))
                                    .child("Retry detection")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.connect_requested = false;
                                        this.service.send(Command::Detect);
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let provider = self.conversation.active_provider.or_else(|| {
            select_available_provider(&self.conversation.providers, self.prefs.provider)
        });
        let installed = self
            .conversation
            .providers
            .iter()
            .filter(|status| status.availability == Availability::Available)
            .map(|status| status.label)
            .collect::<Vec<_>>();
        let mut prompts = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .w_full()
            .max_w(px(260.0));
        for (index, prompt) in SUGGESTED_PROMPTS.iter().enumerate() {
            prompts = prompts.child(
                div()
                    .id(("companion-suggestion", index))
                    .debug_selector(move || format!("companion-suggestion-{index}"))
                    .tab_index(0)
                    .focusable()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(Metrics::RADIUS))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .hover(move |style| {
                        style
                            .border_color(theme.border)
                            .text_color(theme.foreground)
                    })
                    .focus(move |style| style.border_color(theme.primary))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.composer
                            .update(cx, |composer, cx| composer.set_text(*prompt, cx));
                        this.composer.read(cx).focus(window);
                    }))
                    .child(*prompt),
            );
        }
        div()
            .id("companion-empty")
            .debug_selector(|| "companion-empty".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .p(px(20.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(36.0))
                    .rounded(px(10.0))
                    .bg(theme.surface_well)
                    .child(icon(
                        "icons/message-square.svg",
                        theme.muted_foreground,
                        18.0,
                    )),
            )
            .child(
                div()
                    .mt(px(6.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(14.0))
                    .child("Ask about these docs"),
            )
            .child(
                div()
                    .max_w(px(280.0))
                    .text_center()
                    .text_size(px(12.5))
                    .text_color(theme.muted_foreground)
                    .child(
                        "The focused document stays lean. Add files with @ or ask across the \
                         folder when needed.",
                    ),
            )
            .child(
                div()
                    .max_w(px(280.0))
                    .text_center()
                    .text_size(px(11.5))
                    .text_color(theme.muted_foreground.opacity(0.8))
                    .child(match provider {
                        Some(provider) if installed.len() > 1 => format!(
                            "Answers come from {} on this Mac. Also installed: {}.",
                            agent_phrase(provider),
                            installed
                                .iter()
                                .filter(|label| **label != provider.label())
                                .copied()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        Some(provider) => {
                            format!("Answers come from {} on this Mac.", agent_phrase(provider))
                        }
                        None => "Looking for agent CLIs…".into(),
                    }),
            )
            .child(div().mt(px(10.0)).child(prompts))
            .into_any_element()
    }

    fn render_message(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(message) = self.conversation.messages.get(index).cloned() else {
            return div().into_any_element();
        };
        let theme = self.theme;
        let last = index + 1 == self.conversation.messages.len();
        let body = match message.role {
            Role::User => div()
                .flex()
                .justify_end()
                .w_full()
                .child(
                    div()
                        .max_w(relative(0.85))
                        .px(px(12.0))
                        .py(px(7.0))
                        .rounded(px(12.0))
                        .bg(match theme.color_scheme {
                            ColorScheme::Dark => theme.muted,
                            ColorScheme::Light => theme.surface_well,
                        })
                        .text_size(px(markdown::FONT_SIZE))
                        .line_height(px(markdown::FONT_SIZE * markdown::LINE_HEIGHT))
                        .child(message.text()),
                )
                .into_any_element(),
            Role::Assistant => self.render_answer(&message, cx),
        };
        div()
            .id(("companion-message", message.id as usize))
            .debug_selector(move || format!("companion-message-{index}"))
            .w_full()
            .px(px(14.0))
            .pt(px(if index == 0 { 14.0 } else { 6.0 }))
            .pb(px(if last { 14.0 } else { 6.0 }))
            .child(body)
            .into_any_element()
    }

    fn render_answer(&mut self, message: &Message, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let streaming = message.status == MessageStatus::Streaming;
        let weak = cx.weak_entity();
        let on_link: LinkHandler = Rc::new(move |target: &str, _: &mut Window, cx: &mut App| {
            let target = target.to_owned();
            weak.update(cx, |this, cx| this.open_link(&target, cx)).ok();
        });
        let mut column = div().flex().flex_col().gap(px(8.0)).w_full().min_w_0();
        if streaming && message.parts.is_empty() {
            column = column.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .child(icon("icons/clock.svg", theme.muted_foreground, 13.0))
                    .child(match self.conversation.connection {
                        ConnectionStatus::Ready => "Reading your docs…",
                        _ => "Connecting to local agent…",
                    }),
            );
        }
        // Text parts render as one markdown document so lists and fences span chunk boundaries.
        let document = self.answer_document(message);
        let mut text_rendered = false;
        for (part_index, part) in message.parts.iter().enumerate() {
            match part {
                Part::Text(_) => {
                    if text_rendered {
                        continue;
                    }
                    text_rendered = true;
                    column = column.child(
                        MarkdownView {
                            theme,
                            id: format!("companion-answer-{}", message.id).into(),
                            highlights: &self.highlights,
                            on_link: on_link.clone(),
                            muted: false,
                        }
                        .render(&document),
                    );
                }
                Part::Thinking { text, done } => {
                    let key = (message.id, part_index);
                    let expanded = self.expanded.contains(&key);
                    let active = streaming && !done;
                    column = column.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                disclosure_row(
                                    ("companion-thinking", message.id as usize * 64 + part_index),
                                    theme,
                                    expanded,
                                )
                                .child(icon("icons/lightbulb.svg", theme.muted_foreground, 13.0))
                                .child(if active {
                                    "Thinking…"
                                } else {
                                    "Thought process"
                                })
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if !this.expanded.remove(&key) {
                                            this.expanded.insert(key);
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .when(expanded, |section| {
                                section.child(
                                    div()
                                        .pl(px(10.0))
                                        .border_l_2()
                                        .border_color(theme.border_subtle)
                                        .child(
                                            MarkdownView {
                                                theme,
                                                id: format!(
                                                    "companion-thinking-{}-{part_index}",
                                                    message.id
                                                )
                                                .into(),
                                                highlights: &self.highlights,
                                                on_link: on_link.clone(),
                                                muted: true,
                                            }
                                            .render(&markdown::parse(text)),
                                        ),
                                )
                            }),
                    );
                }
                Part::Tool(tool) => {
                    let key = (message.id, part_index);
                    let expanded = self.expanded.contains(&key);
                    let (state_color, state_bg) = match tool.state {
                        ToolState::Completed => (theme.alert_tip, theme.alert_tip.opacity(0.12)),
                        ToolState::Error => (theme.destructive, theme.destructive.opacity(0.12)),
                        ToolState::Cancelled => (theme.muted_foreground, theme.surface_well),
                        ToolState::Pending | ToolState::Running => {
                            (theme.alert_note, theme.alert_note.opacity(0.12))
                        }
                    };
                    column = column.child(
                        div()
                            .flex()
                            .flex_col()
                            .rounded(px(Metrics::RADIUS))
                            .border_1()
                            .border_color(theme.border_subtle)
                            .overflow_hidden()
                            .child(
                                disclosure_row(
                                    ("companion-tool", message.id as usize * 64 + part_index),
                                    theme,
                                    expanded,
                                )
                                .px(px(8.0))
                                .h(px(30.0))
                                .child(icon("icons/wrench.svg", theme.muted_foreground, 13.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_color(theme.foreground)
                                        .child(tool.name.clone()),
                                )
                                .child(
                                    chip(theme)
                                        .h(px(18.0))
                                        .bg(state_bg)
                                        .text_color(state_color)
                                        .text_size(px(10.5))
                                        .child(tool.state.label()),
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if !this.expanded.remove(&key) {
                                            this.expanded.insert(key);
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .when(expanded, |card| {
                                card.child(tool_io("Input", tool.input.as_deref(), theme))
                                    .child(tool_io("Output", tool.output.as_deref(), theme))
                            }),
                    );
                }
                Part::Permission(permission) => {
                    column = column.child(self.render_permission(
                        &permission.request,
                        permission.answer,
                        cx,
                    ));
                }
            }
        }
        if message.status == MessageStatus::Cancelled {
            column = column.child(
                div()
                    .text_size(px(11.5))
                    .text_color(theme.muted_foreground)
                    .child("Response stopped"),
            );
        }
        if !message.citations.is_empty() {
            let mut sources = div().flex().flex_wrap().gap(px(6.0));
            for (index, citation) in message.citations.iter().enumerate() {
                let path = citation.path.clone();
                sources = sources.child(
                    chip(theme)
                        .id(("companion-source", message.id as usize * 64 + index))
                        .debug_selector(move || format!("companion-source-{index}"))
                        .tab_index(0)
                        .focusable()
                        .cursor_pointer()
                        .hover(move |style| style.text_color(theme.foreground))
                        .focus(move |style| style.border_1().border_color(theme.primary))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.emit(CompanionEvent::OpenPath(path.clone()))
                        }))
                        .child(icon("icons/file-text.svg", theme.muted_foreground, 12.0))
                        .child(citation.label.clone()),
                );
            }
            column = column.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{} {}",
                                message.citations.len(),
                                if message.citations.len() == 1 {
                                    "source"
                                } else {
                                    "sources"
                                }
                            )),
                    )
                    .child(sources),
            );
        }
        if !streaming && !message.text().trim().is_empty() {
            let id = message.id;
            let copied = self.copied.is_some_and(|(copied, _)| copied == id);
            column = column.child(
                div().flex().child(
                    div()
                        .id(("companion-copy", id as usize))
                        .debug_selector(move || format!("companion-copy-{id}"))
                        .tab_index(0)
                        .focusable()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .h(px(22.0))
                        .px(px(6.0))
                        .ml(px(-6.0))
                        .rounded(px(5.0))
                        .border_1()
                        .border_color(theme.primary.opacity(0.0))
                        .text_size(px(11.0))
                        .text_color(theme.muted_foreground)
                        .cursor_pointer()
                        .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
                        .focus(move |style| style.border_color(theme.primary))
                        .on_click(cx.listener(move |this, _, _, cx| this.copy_answer(id, cx)))
                        .child(icon(
                            if copied {
                                "icons/check.svg"
                            } else {
                                "icons/copy.svg"
                            },
                            if copied {
                                theme.alert_tip
                            } else {
                                theme.muted_foreground
                            },
                            12.0,
                        ))
                        .child(if copied { "Copied" } else { "Copy" }),
                ),
            );
        }
        column.into_any_element()
    }

    fn render_permission(
        &self,
        request: &PermissionRequest,
        answer: Option<PermissionAnswer>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let mut card = div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(10.0))
            .rounded(px(Metrics::RADIUS))
            .border_1()
            .border_color(if answer.is_none() {
                theme.alert_warning.opacity(0.5)
            } else {
                theme.border_subtle
            })
            .bg(theme
                .alert_warning
                .opacity(if answer.is_none() { 0.06 } else { 0.0 }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon("icons/shield.svg", theme.alert_warning, 14.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Permission needed"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("The agent wants to: {}", request.title)),
                            ),
                    ),
            )
            .children(request.detail.clone().map(|detail| {
                let clipped = detail.lines().take(6).collect::<Vec<_>>().join("\n");
                div()
                    .p(px(8.0))
                    .rounded(px(6.0))
                    .bg(theme.code_surface)
                    .font_family(Metrics::FONT_MONO)
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .child(clipped)
            }));
        card = match answer {
            None => {
                let allow = request.clone();
                let deny = request.clone();
                card.child(
                    div()
                        .text_size(px(11.5))
                        .text_color(theme.muted_foreground)
                        .child("Companion is read-only by default. Allow only if you expect this."),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(8.0))
                        .child(
                            quiet_button("companion-permission-deny", theme)
                                .debug_selector(|| "companion-permission-deny".into())
                                .child("Deny")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.answer_permission(&deny, false, cx)
                                })),
                        )
                        .when(request.allow_option().is_some(), |row| {
                            row.child(
                                primary_button("companion-permission-allow", theme)
                                    .debug_selector(|| "companion-permission-allow".into())
                                    .child("Allow once")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.answer_permission(&allow, true, cx)
                                    })),
                            )
                        }),
                )
            }
            Some(answer) => card.child(
                div()
                    .text_size(px(11.5))
                    .text_color(theme.muted_foreground)
                    .child(match answer {
                        PermissionAnswer::Allowed => "Allowed once",
                        PermissionAnswer::Denied => "Denied",
                        PermissionAnswer::Expired => "No longer needed",
                    }),
            ),
        };
        card.into_any_element()
    }

    fn render_error(&self, error: &CompanionError, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let settings_action = matches!(error.kind, ErrorKind::NoProvider | ErrorKind::MissingCli);
        let retry_label = match error.kind {
            ErrorKind::NoProvider | ErrorKind::MissingCli => "Retry detection",
            _ => "Try again",
        };
        div()
            .id("companion-error")
            .debug_selector(|| "companion-error".into())
            .flex()
            .flex_col()
            .gap(px(6.0))
            .mx(px(10.0))
            .mb(px(8.0))
            .p(px(10.0))
            .rounded(px(Metrics::RADIUS))
            .border_1()
            .border_color(theme.destructive.opacity(0.35))
            .bg(theme.destructive.opacity(0.07))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.0))
                    .child(div().mt(px(1.0)).child(icon(
                        "icons/octagon-alert.svg",
                        theme.destructive,
                        14.0,
                    )))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(error.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.muted_foreground)
                                    .child(error.message.clone()),
                            ),
                    )
                    .child(compact_icon_button(
                        "companion-error-dismiss",
                        "icons/x.svg",
                        22.0,
                        12.0,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.conversation.clear_error();
                            cx.notify();
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .pl(px(22.0))
                    .child(
                        quiet_button("companion-error-retry", theme)
                            .debug_selector(|| "companion-error-retry".into())
                            .child(icon("icons/rotate-ccw.svg", theme.muted_foreground, 12.0))
                            .child(retry_label)
                            .on_click(cx.listener(|this, _, _, cx| this.retry(cx))),
                    )
                    .when(settings_action, |row| {
                        row.child(
                            quiet_button("companion-error-settings", theme)
                                .child("Open Settings")
                                .on_click(
                                    cx.listener(|_, _, _, cx| {
                                        cx.emit(CompanionEvent::OpenSettings)
                                    }),
                                ),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_context_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut bar = div()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.0))
            .min_h(px(32.0))
            .px(px(12.0))
            .pt(px(8.0))
            .overflow_hidden();
        match self.conversation.context_trace.as_ref() {
            Some(trace) => {
                let focused = trace
                    .items
                    .iter()
                    .find(|item| item.reason == TraceReason::Focused)
                    .map(|item| file_name(&item.path));
                let adaptive = match trace.retrieval_mode {
                    RetrievalMode::AdaptiveFff => "Adaptive · FFF",
                    RetrievalMode::AdaptiveLocal => "Adaptive · local",
                    RetrievalMode::FocusedOnly => "Adaptive",
                };
                bar = bar
                    .child(
                        chip(theme)
                            .id("companion-context-chip")
                            .debug_selector(|| "companion-context-chip".into())
                            .tab_index(0)
                            .focusable()
                            .flex_shrink()
                            .min_w_0()
                            .max_w(px(200.0))
                            .cursor_pointer()
                            .when(self.context_open, |chip| chip.bg(theme.muted))
                            .hover(move |style| style.text_color(theme.foreground))
                            .focus(move |style| style.border_1().border_color(theme.primary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.context_open = !this.context_open;
                                cx.notify();
                            }))
                            .child(icon("icons/file-text.svg", theme.muted_foreground, 12.0))
                            .child(
                                div()
                                    .flex_none()
                                    .max_w(px(120.0))
                                    .truncate()
                                    .child(focused.unwrap_or_else(|| "No focused doc".into())),
                            )
                            .child(tabular_nums(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .child(format!("· {} focused", trace.focused_count)),
                            )),
                    )
                    .child(
                        chip(theme)
                            .flex_shrink()
                            .min_w_0()
                            .overflow_hidden()
                            .child(icon("icons/search.svg", theme.muted_foreground, 12.0))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .child(if trace.searched_count > 0 {
                                        format!("{adaptive} · {} read", trace.read_range_count)
                                    } else {
                                        adaptive.to_owned()
                                    }),
                            ),
                    )
                    .child(div().flex_1())
                    .child(tabular_nums(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(11.5))
                            .text_color(theme.muted_foreground)
                            .child(icon("icons/gauge.svg", theme.muted_foreground, 12.0))
                            .child(format!("≈{} added", format_tokens(trace.estimated_tokens))),
                    ));
            }
            None => {
                let label = self
                    .context
                    .active_path
                    .as_deref()
                    .filter(|path| is_supported_markdown(path))
                    .map(file_name);
                bar = bar.child(
                    chip(theme)
                        .min_w_0()
                        .max_w(px(240.0))
                        .child(icon("icons/file-text.svg", theme.muted_foreground, 12.0))
                        .child(div().min_w_0().truncate().child(match label {
                            Some(name) => format!("{name} · focused"),
                            None => "No document in context".into(),
                        })),
                );
            }
        }
        for (index, tag) in self.tags.iter().enumerate() {
            let remove = tag.clone();
            bar = bar.child(
                chip(theme)
                    .min_w_0()
                    .max_w(px(160.0))
                    .child(icon(
                        if tag.kind == TagKind::Folder {
                            "icons/folder.svg"
                        } else {
                            "icons/at-sign.svg"
                        },
                        theme.muted_foreground,
                        11.0,
                    ))
                    .child(div().min_w_0().truncate().child(file_name(&tag.path)))
                    .child(
                        div()
                            .id(("companion-tag-remove", index))
                            .debug_selector(move || format!("companion-tag-remove-{index}"))
                            .tab_index(0)
                            .focusable()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(16.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(theme.primary.opacity(0.0))
                            .cursor_pointer()
                            .hover(move |style| style.bg(theme.muted))
                            .focus(move |style| style.border_color(theme.primary))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.tags.retain(|tag| *tag != remove);
                                cx.notify();
                            }))
                            .child(icon("icons/x.svg", theme.muted_foreground, 10.0)),
                    ),
            );
        }
        if self.context_open
            && let Some(trace) = self.conversation.context_trace.as_ref()
        {
            let mut grouped: Vec<(PathBuf, TraceReason, usize)> = Vec::new();
            for item in &trace.items {
                match grouped
                    .iter_mut()
                    .find(|(path, reason, _)| *path == item.path && *reason == item.reason)
                {
                    Some(entry) => entry.2 += item.bytes,
                    None => grouped.push((item.path.clone(), item.reason, item.bytes)),
                }
            }
            let mut warnings = Vec::<&String>::new();
            for warning in &self.conversation.warnings {
                if !warnings.contains(&warning) {
                    warnings.push(warning);
                }
            }
            let mut popover = div()
                .id("companion-context-popover")
                .debug_selector(|| "companion-context-popover".into())
                .occlude()
                .absolute()
                .left(px(10.0))
                .bottom(relative(1.0))
                .w(px(300.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .p(px(12.0))
                .rounded(px(Metrics::RADIUS))
                .border_1()
                .border_color(theme.border)
                .bg(theme.surface_raised)
                .shadow_lg()
                .text_size(px(12.0))
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.context_open = false;
                    cx.notify();
                }))
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Context added by Mdow"))
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(theme.muted_foreground)
                        .child("Estimates cover Mdow source injection, not the provider's full context window."),
                );
            let mut rows = div().flex().flex_col().gap(px(4.0));
            for (path, reason, bytes) in grouped {
                rows = rows.child(
                    div()
                        .flex()
                        .gap(px(8.0))
                        .child(div().flex_1().min_w_0().truncate().child(format!(
                            "{} · {}",
                            file_name(&path),
                            reason.label()
                        )))
                        .child(tabular_nums(
                            div()
                                .flex_none()
                                .text_color(theme.muted_foreground)
                                .child(format_bytes(bytes)),
                        )),
                );
            }
            popover = popover.child(rows);
            if !warnings.is_empty() {
                let mut notes = div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .pt(px(8.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .text_color(theme.alert_warning);
                for warning in warnings {
                    notes = notes.child(warning.clone());
                }
                popover = popover.child(notes);
            }
            bar = bar.child(deferred(popover).with_priority(3));
        }
        bar.into_any_element()
    }

    fn render_composer(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let streaming = self.conversation.streaming;
        let focused = self.composer.read(cx).is_focused(window);
        let has_text = !self.composer.read(cx).text().trim().is_empty();
        let mentions = self
            .mention
            .as_ref()
            .filter(|mention| !mention.results.is_empty())
            .map(|mention| {
                let mut list = div()
                    .id("companion-mentions")
                    .debug_selector(|| "companion-mentions".into())
                    .occlude()
                    .absolute()
                    .left(px(10.0))
                    .right(px(10.0))
                    .bottom(relative(1.0))
                    .mb(px(4.0))
                    .flex()
                    .flex_col()
                    .p(px(4.0))
                    .rounded(px(Metrics::RADIUS))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface_raised)
                    .shadow_lg()
                    .text_size(px(12.0));
                for (index, path) in mention.results.iter().enumerate() {
                    let highlighted = index == mention.highlighted;
                    list = list.child(
                        div()
                            .id(("companion-mention", index))
                            .debug_selector(move || format!("companion-mention-{index}"))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .h(px(26.0))
                            .px(px(8.0))
                            .rounded(px(5.0))
                            .bg(if highlighted {
                                theme.muted
                            } else {
                                theme.muted.opacity(0.0)
                            })
                            .cursor_pointer()
                            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                if let Some(mention) = this.mention.as_mut()
                                    && mention.highlighted != index
                                {
                                    mention.highlighted = index;
                                    cx.notify();
                                }
                            }))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(mention) = this.mention.as_mut() {
                                    mention.highlighted = index;
                                }
                                this.accept_mention(cx);
                                this.composer.read(cx).focus(window);
                            }))
                            .child(icon("icons/file-text.svg", theme.muted_foreground, 12.0))
                            .child(div().truncate().child(file_name(path))),
                    );
                }
                deferred(list).with_priority(3)
            });
        let send = div()
            .id("companion-send")
            .debug_selector(|| "companion-send".into())
            .tab_index(0)
            .focusable()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(28.0))
            .rounded(px(7.0))
            .border_1()
            .map(|button| {
                if streaming {
                    button
                        .bg(theme.surface_well)
                        .border_color(theme.border)
                        .text_color(theme.foreground)
                } else if has_text {
                    button
                        .bg(theme.foreground)
                        .border_color(theme.foreground.opacity(0.0))
                } else {
                    button
                        .bg(theme.surface_well)
                        .border_color(theme.border.opacity(0.0))
                }
            })
            .cursor_pointer()
            .hover(|style| style.opacity(0.88))
            .active(|style| style.opacity(0.75))
            .focus(move |style| style.border_color(theme.primary))
            .on_click(cx.listener(move |this, _, window, cx| {
                if this.conversation.streaming {
                    this.stop(cx);
                    this.composer.read(cx).focus(window);
                } else {
                    this.submit(window, cx);
                }
            }))
            .child(if streaming {
                icon("icons/stop.svg", theme.foreground, 14.0)
            } else {
                icon(
                    "icons/arrow-up.svg",
                    if has_text {
                        theme.background
                    } else {
                        theme.muted_foreground
                    },
                    15.0,
                )
            });
        div()
            .relative()
            .flex_none()
            .px(px(10.0))
            .pt(px(8.0))
            .pb(px(10.0))
            .children(mentions)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .p(px(10.0))
                    .pb(px(8.0))
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(if focused {
                        theme.primary.opacity(0.45)
                    } else {
                        theme.border
                    })
                    .bg(match theme.color_scheme {
                        ColorScheme::Dark => theme.muted,
                        ColorScheme::Light => theme.surface_raised,
                    })
                    .when(theme.color_scheme == ColorScheme::Light, |field| {
                        field.shadow_sm()
                    })
                    .capture_action(cx.listener(|this, _: &ComposerMoveUp, _, cx| {
                        if this.move_mention(-1, cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .capture_action(cx.listener(|this, _: &ComposerMoveDown, _, cx| {
                        if this.move_mention(1, cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .capture_action(cx.listener(|this, _: &ComposerSend, _, cx| {
                        if this.mention.as_ref().is_some_and(|m| !m.results.is_empty()) {
                            this.accept_mention(cx);
                            cx.stop_propagation();
                        }
                    }))
                    .capture_action(cx.listener(|this, _: &FieldCancel, _, cx| {
                        if this.mention.take().is_some() {
                            cx.notify();
                            cx.stop_propagation();
                        }
                    }))
                    .child(self.composer.clone())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(theme.muted_foreground.opacity(0.85))
                                    .child(if streaming {
                                        "Answering…"
                                    } else {
                                        "⏎ send · ⇧⏎ newline · @ add a file"
                                    }),
                            )
                            .child(send),
                    ),
            )
            .into_any_element()
    }

    fn render_resize_handle(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let entity = cx.entity();
        let dragging = self.resizing.is_some();
        div()
            .id("companion-resize")
            .debug_selector(|| "companion-resize".into())
            .track_focus(&self.resize_focus)
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(-RESIZE_HANDLE / 2.0))
            .w(px(RESIZE_HANDLE))
            .cursor(CursorStyle::ResizeLeftRight)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(RESIZE_HANDLE / 2.0 - 1.0))
                    .w(px(2.0))
                    .when(dragging, |line| line.bg(theme.primary.opacity(0.6)))
                    .hover(move |line| line.bg(theme.border)),
            )
            .focus(move |style| style.bg(theme.primary.opacity(0.25)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.resizing = Some(f32::from(event.position.x));
                    this.window_width = f32::from(window.viewport_size().width);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let step = match event.keystroke.key.as_str() {
                    "left" => RESIZE_STEP,
                    "right" => -RESIZE_STEP,
                    _ => return,
                };
                this.window_width = f32::from(window.viewport_size().width);
                this.set_width(this.settings.panel_width + step, cx);
                this.store.save(&this.settings);
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        window.on_mouse_event({
                            let entity = entity.clone();
                            move |event: &MouseMoveEvent, _, window, cx| {
                                if entity.read(cx).resizing.is_none() {
                                    return;
                                }
                                let window_width = f32::from(window.viewport_size().width);
                                let width = window_width - f32::from(event.position.x);
                                entity.update(cx, |this, cx| {
                                    this.window_width = window_width;
                                    this.set_width(width, cx);
                                });
                            }
                        });
                        window.on_mouse_event({
                            let entity = entity.clone();
                            move |event: &MouseUpEvent, _, _, cx| {
                                if event.button == MouseButton::Left
                                    && entity.read(cx).resizing.is_some()
                                {
                                    entity.update(cx, |this, cx| {
                                        this.finish_resize();
                                        cx.notify();
                                    });
                                }
                            }
                        });
                    },
                )
                .size_full(),
            )
            .into_any_element()
    }
}

fn agent_phrase(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Custom => "your custom agent",
        other => other.label(),
    }
}

fn disclosure_row(id: (&'static str, usize), theme: Theme, expanded: bool) -> Stateful<Div> {
    div()
        .id(id)
        .tab_index(0)
        .focusable()
        .flex()
        .items_center()
        .gap(px(6.0))
        .h(px(24.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(theme.primary.opacity(0.0))
        .text_size(px(12.0))
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .hover(move |style| style.text_color(theme.foreground))
        .focus(move |style| style.border_color(theme.primary))
        .child(icon(
            if expanded {
                "icons/chevron-down.svg"
            } else {
                "icons/chevron-right.svg"
            },
            theme.muted_foreground,
            12.0,
        ))
}

fn tool_io(label: &'static str, value: Option<&str>, theme: Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .px(px(10.0))
        .py(px(8.0))
        .border_t_1()
        .border_color(theme.border_subtle)
        .child(
            div()
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.muted_foreground)
                .child(label.to_uppercase()),
        )
        .child(
            div()
                .font_family(Metrics::FONT_MONO)
                .text_size(px(11.0))
                .text_color(theme.foreground)
                .child(
                    value
                        .map(|value| value.lines().take(40).collect::<Vec<_>>().join("\n"))
                        .unwrap_or_else(|| "—".into()),
                ),
        )
}

impl Render for CompanionPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        self.window_width = f32::from(window.viewport_size().width);
        let setup = self.conversation.providers_loaded && !self.has_available_provider();
        if setup && self.composer.read(cx).is_focused(window) {
            // The composer is not shown during setup; keep keyboard focus inside the panel.
            self.focus_handle.focus(window);
        }
        let entity = cx.entity();
        let body: AnyElement = if setup {
            self.render_setup(cx)
        } else if self.conversation.messages.is_empty() {
            self.render_empty(cx)
        } else {
            list(self.list_state.clone(), move |index, _, cx| {
                entity.update(cx, |this, cx| this.render_message(index, cx))
            })
            .flex_1()
            .min_h_0()
            .w_full()
            .into_any_element()
        };
        let error = self
            .conversation
            .error
            .clone()
            .filter(|error| !(setup && error.kind == ErrorKind::NoProvider))
            .map(|error| self.render_error(&error, cx));
        div()
            .id("companion-panel")
            .debug_selector(|| "companion-panel".into())
            .key_context("CompanionPanel")
            .track_focus(&self.focus_handle)
            .relative()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(self.display_width))
            .h_full()
            .min_h_0()
            .border_l_1()
            .border_color(theme.border_subtle)
            .bg(theme.sidebar)
            .font_family(Metrics::FONT_SANS)
            .text_size(px(13.0))
            .text_color(theme.foreground)
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| {
                if this.menu.take().is_some() || this.mention.take().is_some() {
                    this.composer.read(cx).focus(window);
                    cx.notify();
                } else if this.context_open {
                    this.context_open = false;
                    cx.notify();
                } else {
                    cx.emit(CompanionEvent::FocusReader);
                }
            }))
            .on_action(
                cx.listener(|this, _: &NewCompanionChat, window, cx| this.new_chat(window, cx)),
            )
            .child(self.render_header(cx))
            .child(body)
            .children(error)
            .when(!setup, |panel| {
                panel
                    .child(self.render_context_bar(cx))
                    .child(self.render_composer(window, cx))
            })
            .child(self.render_resize_handle(cx))
            .children(self.render_menu(cx))
    }
}

// ---------------------------------------------------------------------------------------------
// Hooks into the window. `MdowApp` keeps one `CompanionHost`; everything else lives here.

#[derive(Default)]
pub struct CompanionHost {
    panel: Option<Entity<CompanionPanel>>,
    open: bool,
    prefs: CompanionPrefs,
    search: Option<SearchPath>,
    _events: Option<Subscription>,
}

impl CompanionHost {
    pub fn is_open(&self) -> bool {
        self.open && self.prefs.enabled
    }

    pub fn panel(&self) -> Option<&Entity<CompanionPanel>> {
        self.panel.as_ref()
    }

    /// Tests point detection at a fake `PATH` instead of the login shell's.
    pub fn set_search_path(&mut self, search: SearchPath) {
        self.search = Some(search);
    }

    /// The width the open panel takes from the window this frame.
    pub fn reserved_width(&self, window_width: f32, cx: &App) -> f32 {
        match (&self.panel, self.is_open()) {
            (Some(panel), true) => {
                settings::clamp_panel_width(panel.read(cx).width(), window_width)
            }
            _ => 0.0,
        }
    }

    /// Syncs the panel with the window and returns it when open. Call once per frame.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        prefs: CompanionPrefs,
        theme: Theme,
        context: DocumentContext,
        window_width: f32,
        app_focus: &FocusHandle,
        window: &mut Window,
        cx: &mut Context<MdowApp>,
    ) -> Option<AnyElement> {
        self.prefs = prefs;
        if !prefs.enabled
            && let Some(panel) = self.panel.as_ref()
        {
            if panel.read(cx).focus_handle.contains_focused(window, cx) {
                app_focus.focus(window);
            }
            // Disabling the companion retires its agent process with the panel.
            self.panel = None;
            self._events = None;
            self.open = false;
        }
        let panel = self.panel.clone()?;
        if !self.is_open() {
            return None;
        }
        let width = self.reserved_width(window_width, cx);
        panel.update(cx, |panel, cx| {
            panel.sync(prefs, theme, context, cx);
            panel.display_width = width;
        });
        Some(panel.into_any_element())
    }

    /// The tab-bar toggle, beside Find and the command palette. It stays out of the Tab order
    /// (⌘I, the palette and View > AI Companion reach it from the keyboard) so the toolbar's
    /// focus sequence is unchanged.
    pub fn toggle_button(&self, theme: Theme, cx: &Context<MdowApp>) -> Option<AnyElement> {
        if !self.prefs.enabled {
            return None;
        }
        let open = self.is_open();
        let color = if open {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        Some(
            div()
                .id("toggle-companion")
                .debug_selector(|| "toggle-companion".into())
                .group("toggle-companion")
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(28.0))
                .rounded(px(6.0))
                .when(open, |button| button.bg(theme.muted))
                .cursor_pointer()
                .hover(move |style| style.bg(theme.muted))
                .active(|style| style.opacity(0.8))
                .on_click(cx.listener(|this, _, window, cx| this.toggle_companion(window, cx)))
                .child(
                    icon("icons/message-square.svg", color, Metrics::ICON_SIZE)
                        .group_hover("toggle-companion", move |style| {
                            style.text_color(theme.foreground)
                        }),
                )
                .into_any_element(),
        )
    }

    pub(crate) fn set_prefs(&mut self, prefs: CompanionPrefs) {
        self.prefs = prefs;
    }
}

impl MdowApp {
    fn companion_context(&self) -> DocumentContext {
        DocumentContext {
            active_path: self.model.tabs.active().map(|tab| tab.path().to_owned()),
            workspace_root: self
                .model
                .workspace
                .as_ref()
                .map(|tree| tree.root.path.clone()),
        }
    }

    fn ensure_companion_panel(
        &mut self,
        prefs: CompanionPrefs,
        state_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<CompanionPanel> {
        if let Some(panel) = self.companion.panel.clone() {
            return panel;
        }
        let search = self.companion.search.clone();
        let theme = Theme::resolve(crate::prefs::ThemeMode::System, window.appearance());
        let store = CompanionStore::beside(&state_path);
        let panel = cx.new(|cx| CompanionPanel::new(prefs, store, search, theme, window, cx));
        let events = cx.subscribe_in(&panel, window, Self::on_companion_event);
        self.companion.panel = Some(panel.clone());
        self.companion._events = Some(events);
        panel
    }

    /// ⌘I, the tab-bar button, the palette and View > Companion all land here.
    pub(crate) fn toggle_companion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prefs = self.companion_prefs();
        if !prefs.enabled {
            return;
        }
        self.companion.set_prefs(prefs);
        if self.companion.is_open() {
            self.companion.open = false;
            self.focus_handle(cx).focus(window);
            cx.notify();
            return;
        }
        let panel = self.ensure_companion_panel(prefs, self.companion_state_path(), window, cx);
        self.companion.open = true;
        let context = self.companion_context();
        panel.update(cx, |panel, cx| {
            panel.context = context;
            panel.opened(window, cx);
        });
        cx.notify();
    }

    /// Settings > Companion > Choose…
    pub(crate) fn choose_companion_executable(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prefs = self.companion_prefs();
        let panel = self.ensure_companion_panel(prefs, self.companion_state_path(), window, cx);
        panel.update(cx, |panel, cx| panel.choose_custom_executable(cx));
    }

    fn on_companion_event(
        &mut self,
        _: &Entity<CompanionPanel>,
        event: &CompanionEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            CompanionEvent::FocusReader => self.focus_handle(cx).focus(window),
            CompanionEvent::Close => {
                self.companion.open = false;
                self.focus_handle(cx).focus(window);
                cx.notify();
            }
            CompanionEvent::OpenPath(path) => self.open_path(path, cx),
            CompanionEvent::OpenSettings => {
                self.click_toggle_overlay(OverlayKind::Settings, window, cx)
            }
            CompanionEvent::Pref(edit) => self.companion_pref(*edit, cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        companion::{ToggleCompanion, composer::test_bindings, state::MessageStatus},
        document::parse_document,
    };
    use gpui::{Modifiers, Pixels, Point, TestAppContext, VisualTestContext, WindowHandle, point};

    #[test]
    fn mentions_match_the_trailing_at_query() {
        assert_eq!(trailing_mention("compare with @arch"), Some("arch"));
        assert_eq!(trailing_mention("@"), Some(""));
        assert_eq!(trailing_mention("email me@x.com please"), None);
        assert_eq!(trailing_mention("no mention"), None);
        assert!(
            mention_score("arc", "architecture.md").unwrap()
                < mention_score("arc", "search-cache.md").unwrap()
        );
        assert_eq!(mention_score("zzz", "architecture.md"), None);
    }

    #[test]
    fn token_and_byte_labels_match_electron() {
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1_250), "1.2k");
        assert_eq!(format_tokens(12_400), "12k");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2_048), "2.0 KB");
    }

    /// A PATH with the fake ACP agent installed as `opencode`.
    fn fake_agent_bin() -> tempfile::TempDir {
        let bin = tempfile::tempdir().unwrap();
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fake-acp-agent.sh");
        let target = bin.path().join("opencode");
        std::fs::copy(script, &target).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        bin
    }

    fn companion_window(
        cx: &mut TestAppContext,
        bin: Option<&std::path::Path>,
    ) -> (WindowHandle<MdowApp>, tempfile::TempDir) {
        cx.update(|cx| cx.bind_keys(test_bindings()));
        let docs = tempfile::tempdir().unwrap();
        let path = docs.path().join("overview.md");
        let source = "# Overview\n\nLaunch is on October 14.\n";
        std::fs::write(&path, source).unwrap();
        let search = SearchPath::new(bin.map(|bin| bin.to_owned()));
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let mut app = MdowApp::new(window, cx);
                    app.model
                        .tabs
                        .open(parse_document(path.clone(), source.into()));
                    app.open_error = None;
                    app.companion.set_search_path(search);
                    app
                })
            })
            .unwrap()
        });
        (window, docs)
    }

    fn redraw(visual: &mut VisualTestContext) {
        visual.update(|window, cx| window.draw(cx).clear());
    }

    fn panel(
        window: &WindowHandle<MdowApp>,
        visual: &mut VisualTestContext,
    ) -> Entity<CompanionPanel> {
        window
            .update(visual, |app, _, _| app.companion.panel().cloned())
            .unwrap()
            .expect("companion panel exists")
    }

    /// Waits in real time for the service thread and the agent subprocess.
    fn wait_until(
        visual: &mut VisualTestContext,
        what: &str,
        mut done: impl FnMut(&mut VisualTestContext) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            visual.run_until_parked();
            if done(visual) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn click(visual: &mut VisualTestContext, selector: &'static str) {
        redraw(visual);
        let center = visual
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} should be painted"))
            .center();
        visual.simulate_mouse_move(center, None, Modifiers::none());
        visual.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
    }

    fn conversation(
        window: &WindowHandle<MdowApp>,
        visual: &mut VisualTestContext,
    ) -> Conversation {
        let panel = panel(window, visual);
        panel.read_with(visual, |panel, _| panel.conversation().clone())
    }

    fn ready(window: &WindowHandle<MdowApp>, visual: &mut VisualTestContext) {
        wait_until(visual, "the agent session", |visual| {
            conversation(window, visual).connection == ConnectionStatus::Ready
        });
    }

    fn type_and_send(visual: &mut VisualTestContext, text: &str) {
        visual.simulate_input(text);
        visual.simulate_keystrokes("enter");
    }

    fn app_focused(window: &WindowHandle<MdowApp>, visual: &mut VisualTestContext) -> bool {
        let focus = window
            .update(visual, |app, _, cx| app.focus_handle(cx))
            .unwrap();
        visual.update(|window, _| focus.is_focused(window))
    }

    #[gpui::test]
    fn the_toggle_opens_the_panel_beside_the_reader(cx: &mut TestAppContext) {
        let (window, _docs) = companion_window(cx, None);
        let mut visual = VisualTestContext::from_window(*window, cx);
        redraw(&mut visual);
        assert!(visual.debug_bounds("companion-panel").is_none());
        let main_before = visual.debug_bounds("main-column").unwrap();

        click(&mut visual, "toggle-companion");
        redraw(&mut visual);
        let panel_bounds = visual
            .debug_bounds("companion-panel")
            .expect("panel is open");
        let main_after = visual.debug_bounds("main-column").unwrap();
        assert_eq!(panel_bounds.size.width, px(settings::DEFAULT_PANEL_WIDTH));
        assert_eq!(main_after.right(), panel_bounds.left());
        assert!(main_after.size.width < main_before.size.width);
        let panel_focus =
            panel(&window, &mut visual).read_with(&visual, |panel, cx| panel.focus_handle(cx));
        assert!(visual.update(|window, cx| panel_focus.contains_focused(window, cx)));

        // No agent on this PATH: the setup screen explains what to install.
        wait_until(&mut visual, "provider detection", |visual| {
            conversation(&window, visual).providers_loaded
        });
        redraw(&mut visual);
        assert!(visual.debug_bounds("companion-setup").is_some());

        visual.dispatch_action(ToggleCompanion);
        redraw(&mut visual);
        // gpui keeps stale debug bounds across frames, so closing is checked by layout.
        assert!(
            !window
                .update(&mut visual, |app, _, _| app.companion.is_open())
                .unwrap()
        );
        assert_eq!(
            visual.debug_bounds("main-column").unwrap().size.width,
            main_before.size.width
        );
        assert!(app_focused(&window, &mut visual));
    }

    #[gpui::test]
    fn escape_in_the_composer_hands_focus_back_to_the_reader(cx: &mut TestAppContext) {
        let (window, _docs) = companion_window(cx, None);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleCompanion);
        redraw(&mut visual);
        assert!(!app_focused(&window, &mut visual));
        visual.simulate_keystrokes("escape");
        assert!(app_focused(&window, &mut visual));
        redraw(&mut visual);
        assert!(
            visual.debug_bounds("companion-panel").is_some(),
            "Escape keeps the panel open"
        );
    }

    #[gpui::test]
    fn dragging_the_divider_resizes_within_limits(cx: &mut TestAppContext) {
        let (window, _docs) = companion_window(cx, None);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleCompanion);
        redraw(&mut visual);
        let drag = |visual: &mut VisualTestContext, dx: f32| {
            let handle = visual.debug_bounds("companion-resize").unwrap().center();
            let to: Point<Pixels> = point(handle.x + px(dx), handle.y);
            visual.simulate_mouse_move(handle, None, Modifiers::none());
            visual.simulate_mouse_down(handle, MouseButton::Left, Modifiers::none());
            visual.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
            visual.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
            redraw(visual);
            visual.debug_bounds("companion-panel").unwrap().size.width
        };
        let wider = f32::from(drag(&mut visual, -80.0));
        assert!(
            (wider - (settings::DEFAULT_PANEL_WIDTH + 80.0)).abs() <= 1.0,
            "{wider}"
        );
        assert_eq!(drag(&mut visual, 400.0), px(settings::MIN_PANEL_WIDTH));
    }

    #[gpui::test]
    fn sends_to_a_fake_acp_agent_and_renders_the_streamed_answer(cx: &mut TestAppContext) {
        let bin = fake_agent_bin();
        let (window, _docs) = companion_window(cx, Some(bin.path()));
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleCompanion);
        ready(&window, &mut visual);
        let models = conversation(&window, &mut visual).models;
        assert_eq!(models.current_name(), Some("Claude Sonnet 4.5"));

        type_and_send(&mut visual, "Summarize this document");
        wait_until(&mut visual, "the streamed answer", |visual| {
            let conversation = conversation(&window, visual);
            conversation.messages.len() == 2
                && conversation.messages[1].status == MessageStatus::Complete
        });
        let conversation = conversation(&window, &mut visual);
        let answer = &conversation.messages[1];
        assert!(answer.text().contains("## Summary"));
        assert!(
            !answer.text().contains("src:"),
            "citations are stripped from the text"
        );
        assert_eq!(answer.citations.len(), 1);
        assert_eq!(answer.citations[0].label, "overview.md");
        assert!(
            answer
                .parts
                .iter()
                .any(|part| matches!(part, Part::Tool(tool) if tool.state == ToolState::Completed))
        );
        let trace = conversation.context_trace.expect("context trace");
        assert_eq!(trace.focused_count, 1);
        redraw(&mut visual);
        assert!(visual.debug_bounds("companion-message-1").is_some());
        assert!(visual.debug_bounds("companion-context-chip").is_some());
        assert!(visual.debug_bounds("companion-source-0").is_some());
        let composer =
            panel(&window, &mut visual).read_with(&visual, |panel, _| panel.composer().clone());
        assert_eq!(
            composer.read_with(&visual, |composer, _| composer.text().to_owned()),
            ""
        );
    }

    #[gpui::test]
    fn stop_cancels_a_running_answer(cx: &mut TestAppContext) {
        let bin = fake_agent_bin();
        let (window, _docs) = companion_window(cx, Some(bin.path()));
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleCompanion);
        ready(&window, &mut visual);
        type_and_send(&mut visual, "SLOW answer please");
        wait_until(&mut visual, "the first chunk", |visual| {
            conversation(&window, visual)
                .messages
                .get(1)
                .is_some_and(|message| message.text().contains("Working on it"))
        });
        assert!(conversation(&window, &mut visual).streaming);
        click(&mut visual, "companion-send");
        visual.run_until_parked();
        let stopped = conversation(&window, &mut visual);
        assert!(!stopped.streaming);
        assert_eq!(stopped.messages[1].status, MessageStatus::Cancelled);
        // The agent restarts in the background so the next question just works.
        ready(&window, &mut visual);
        type_and_send(&mut visual, "Summarize this document");
        wait_until(&mut visual, "the next answer", |visual| {
            conversation(&window, visual)
                .messages
                .get(3)
                .is_some_and(|message| message.status == MessageStatus::Complete)
        });
    }

    #[gpui::test]
    fn permission_requests_are_answered_inline(cx: &mut TestAppContext) {
        let bin = fake_agent_bin();
        let (window, _docs) = companion_window(cx, Some(bin.path()));
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleCompanion);
        ready(&window, &mut visual);
        type_and_send(&mut visual, "PERMISSION: tidy the notes");
        wait_until(&mut visual, "the permission request", |visual| {
            conversation(&window, visual).pending_permission().is_some()
        });
        redraw(&mut visual);
        assert!(visual.debug_bounds("companion-permission-deny").is_some());
        click(&mut visual, "companion-permission-allow");
        wait_until(&mut visual, "the answer after approval", |visual| {
            conversation(&window, visual)
                .messages
                .get(1)
                .is_some_and(|message| message.status == MessageStatus::Complete)
        });
        let answer = conversation(&window, &mut visual).messages[1].clone();
        assert!(answer.text().contains("Permission granted"));
        assert!(answer.parts.iter().any(|part| matches!(
            part,
            Part::Permission(permission) if permission.answer == Some(PermissionAnswer::Allowed)
        )));
    }

    #[gpui::test]
    fn disabling_the_companion_hides_it_and_stops_the_agent(cx: &mut TestAppContext) {
        let bin = fake_agent_bin();
        let (window, _docs) = companion_window(cx, Some(bin.path()));
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.dispatch_action(ToggleCompanion);
        ready(&window, &mut visual);
        window
            .update(&mut visual, |app, _, cx| {
                app.companion_pref(PrefEdit::CompanionEnabled(false), cx)
            })
            .unwrap();
        redraw(&mut visual);
        let full_width = visual.debug_bounds("main-column").unwrap().size.width;
        assert!(
            window
                .update(&mut visual, |app, _, _| app.companion.panel().is_none())
                .unwrap()
        );
        visual.dispatch_action(ToggleCompanion);
        redraw(&mut visual);
        assert!(
            !window
                .update(&mut visual, |app, _, _| app.companion.is_open())
                .unwrap()
        );
        assert_eq!(
            visual.debug_bounds("main-column").unwrap().size.width,
            full_width
        );
    }
}
