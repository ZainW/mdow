use crate::actions::Dismiss;
use crate::document::DocumentBlock;
use crate::prefs::{
    CodeFont, ContentFont, InterfaceScale, LineWidth, PrefEdit, Prefs, READER_FONT_SIZE,
    READER_LINE_HEIGHT, ThemeMode,
};
use crate::session::Recents;
use crate::sparkle::UpdateUi;
use crate::syntax::PreparedDocument;
use crate::theme::{ColorScheme, Metrics, Theme, active_ui_scale};
use crate::ui::field::{Field, FieldEvent};
use crate::ui::primitives::{
    ListRowStyle, compact_icon_button, icon, key_hint, list_row, tabular_sans,
};
use gpui::{
    AnyElement, App, ClickEvent, Context, Div, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, Hsla, IntoElement, Render, SharedString, Stateful, Subscription, Window, div,
    prelude::*, px,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

gpui::actions!(overlay, [SelectNext, SelectPrev]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Find,
    Palette,
    Settings,
    Shortcuts,
}

pub struct OpenOverlay {
    view: OverlayView,
    _events: Subscription,
}

enum OverlayView {
    Find(Entity<FindOverlay>),
    Palette(Entity<PaletteOverlay>),
    Settings(Entity<SettingsPanel>),
    Shortcuts(Entity<ShortcutsCard>),
}

impl OpenOverlay {
    pub fn find(view: Entity<FindOverlay>, events: Subscription) -> Self {
        Self {
            view: OverlayView::Find(view),
            _events: events,
        }
    }

    pub fn palette(view: Entity<PaletteOverlay>, events: Subscription) -> Self {
        Self {
            view: OverlayView::Palette(view),
            _events: events,
        }
    }

    pub fn settings(view: Entity<SettingsPanel>, events: Subscription) -> Self {
        Self {
            view: OverlayView::Settings(view),
            _events: events,
        }
    }

    pub fn shortcuts(view: Entity<ShortcutsCard>, events: Subscription) -> Self {
        Self {
            view: OverlayView::Shortcuts(view),
            _events: events,
        }
    }

    fn kind(&self) -> OverlayKind {
        match self.view {
            OverlayView::Find(_) => OverlayKind::Find,
            OverlayView::Palette(_) => OverlayKind::Palette,
            OverlayView::Settings(_) => OverlayKind::Settings,
            OverlayView::Shortcuts(_) => OverlayKind::Shortcuts,
        }
    }
}

#[derive(Default)]
pub struct OverlayHost {
    open: Option<OpenOverlay>,
    return_focus: Option<FocusHandle>,
}

impl OverlayHost {
    pub fn kind(&self) -> Option<OverlayKind> {
        self.open.as_ref().map(OpenOverlay::kind)
    }

    pub fn open(&mut self, overlay: OpenOverlay, return_focus: FocusHandle) {
        if self.return_focus.is_none() {
            self.return_focus = Some(return_focus);
        }
        self.open = Some(overlay);
    }

    pub fn close(&mut self, window: Option<&mut Window>) -> bool {
        let closed = self.open.take().is_some();
        if closed && let (Some(window), Some(focus)) = (window, self.return_focus.take()) {
            focus.focus(window);
        }
        closed
    }

    pub fn find(&self) -> Option<&Entity<FindOverlay>> {
        match self.open.as_ref().map(|open| &open.view) {
            Some(OverlayView::Find(view)) => Some(view),
            _ => None,
        }
    }

    pub fn retarget_find(&self, document: Option<Arc<PreparedDocument>>, cx: &mut App) {
        if let Some(view) = self.find() {
            view.update(cx, |find, cx| find.retarget(document, cx));
        }
    }

    pub fn refresh_settings(&self, prefs: &Prefs, update: UpdateUi, cx: &mut App) {
        if let Some(OverlayView::Settings(view)) = self.open.as_ref().map(|open| &open.view) {
            view.update(cx, |panel, cx| panel.refresh(*prefs, update, cx));
        }
    }

    pub fn render_layer(&self, theme: Theme) -> Option<AnyElement> {
        let open = self.open.as_ref()?;
        Some(match &open.view {
            OverlayView::Find(view) => find_layer(view.clone(), theme),
            OverlayView::Palette(view) => modal_layer(view.clone(), theme),
            OverlayView::Settings(view) => modal_layer(view.clone(), theme),
            OverlayView::Shortcuts(view) => modal_layer(view.clone(), theme),
        })
    }
}

fn find_layer(view: Entity<FindOverlay>, theme: Theme) -> AnyElement {
    div()
        .absolute()
        // Just below the breadcrumb so the bar never covers its full-width toggle.
        .top(px(theme.ui.titlebar_height
            + theme.ui.breadcrumb_height
            + 10.0))
        .right(px(16.0))
        .w(px(340.0))
        .child(view)
        .into_any_element()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum OverlayEdge {
    LightShadow,
    DarkKeyline { keyline: Hsla },
}

impl OverlayEdge {
    fn for_theme(theme: Theme) -> Self {
        match theme.color_scheme {
            ColorScheme::Light => Self::LightShadow,
            ColorScheme::Dark => Self::DarkKeyline {
                keyline: theme.border_subtle,
            },
        }
    }
}

pub fn overlay_surface(width: f32, theme: Theme) -> Div {
    let surface = div()
        .w(px(theme.ui.space(width)))
        .rounded(px(Metrics::RADIUS))
        .bg(theme.surface_raised);
    match OverlayEdge::for_theme(theme) {
        OverlayEdge::LightShadow => surface.shadow_lg(),
        OverlayEdge::DarkKeyline { keyline } => surface.border_1().border_color(keyline),
    }
}

fn paint_overlay_edge(element: Div, theme: Theme) -> Div {
    match OverlayEdge::for_theme(theme) {
        OverlayEdge::LightShadow => element.shadow_lg(),
        OverlayEdge::DarkKeyline { keyline } => element.border_1().border_color(keyline),
    }
}

fn modal_layer(child: impl IntoElement, theme: Theme) -> AnyElement {
    div()
        .id("overlay-backdrop")
        .debug_selector(|| "overlay-backdrop".into())
        .absolute()
        .inset_0()
        .flex()
        .items_start()
        .justify_center()
        .pt(px(72.0))
        .bg(theme.background.opacity(0.46))
        .occlude()
        .on_click(|_, window, cx| window.dispatch_action(Box::new(Dismiss), cx))
        .child(
            div()
                .id("overlay-card")
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(child),
        )
        .into_any_element()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindHit {
    pub block: usize,
    pub range_start: usize,
    pub range_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FindMatches {
    hits: Vec<FindHit>,
    cursor: Option<usize>,
}

impl FindMatches {
    fn from_hits(hits: Vec<FindHit>, prefer: Option<FindHit>) -> Self {
        let cursor = if hits.is_empty() {
            None
        } else {
            Some(
                prefer
                    .and_then(|wanted| {
                        hits.iter().position(|hit| {
                            hit.block > wanted.block
                                || (hit.block == wanted.block
                                    && hit.range_start >= wanted.range_start)
                        })
                    })
                    .unwrap_or(0),
            )
        };
        Self { hits, cursor }
    }

    pub fn hits(&self) -> &[FindHit] {
        &self.hits
    }

    pub fn active(&self) -> Option<FindHit> {
        self.cursor.and_then(|index| self.hits.get(index).copied())
    }

    pub fn position(&self) -> Option<(usize, usize)> {
        Some((self.cursor? + 1, self.hits.len()))
    }
}

pub fn find_in_blocks(blocks: &[DocumentBlock], query: &str) -> Vec<FindHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    for (block, text) in blocks.iter().enumerate() {
        let haystack = text.find_text().to_lowercase();
        let mut start = 0;
        while let Some(offset) = haystack[start..].find(&needle) {
            let range_start = start + offset;
            let range_end = range_start + needle.len();
            hits.push(FindHit {
                block,
                range_start,
                range_end,
            });
            start = range_start + needle.len().max(1);
        }
    }
    hits
}

#[derive(Debug, Clone, PartialEq)]
pub enum FindEvent {
    ActiveHit(FindHit),
    Dismissed,
}

pub struct FindOverlay {
    query: Entity<Field>,
    document: Option<Arc<PreparedDocument>>,
    matches: FindMatches,
    theme_mode: ThemeMode,
    _query_events: Subscription,
}

impl EventEmitter<FindEvent> for FindOverlay {}

impl FindOverlay {
    pub fn new(
        document: Option<Arc<PreparedDocument>>,
        theme_mode: ThemeMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| Field::new("Find in document", window, cx));
        let query_events = cx.subscribe(&query, |this, field, event, cx| match event {
            FieldEvent::Edited => {
                let text = field.read(cx).text().to_owned();
                this.recompute(&text, cx);
            }
            FieldEvent::Submitted { backward } => this.advance(*backward, cx),
            FieldEvent::Cancelled => cx.emit(FindEvent::Dismissed),
        });
        Self {
            query,
            document,
            matches: FindMatches::default(),
            theme_mode,
            _query_events: query_events,
        }
    }

    /// The document find searches (the focused split pane's, when split).
    pub fn document_path(&self) -> Option<&Path> {
        self.document
            .as_ref()
            .map(|document| document.path.as_path())
    }

    pub fn retarget(&mut self, document: Option<Arc<PreparedDocument>>, cx: &mut Context<Self>) {
        self.document = document;
        let query = self.query.read(cx).text().to_owned();
        let prefer = self.matches.active();
        self.matches = FindMatches::from_hits(self.search(&query), prefer);
        if let Some(hit) = self.matches.active() {
            cx.emit(FindEvent::ActiveHit(hit));
        }
        cx.notify();
    }

    pub fn advance(&mut self, backward: bool, cx: &mut Context<Self>) {
        let Some(len) = (!self.matches.hits.is_empty()).then_some(self.matches.hits.len()) else {
            return;
        };
        let current = self.matches.cursor.unwrap_or(0);
        let next = if backward {
            current.checked_sub(1).unwrap_or(len - 1)
        } else {
            (current + 1) % len
        };
        self.matches.cursor = Some(next);
        if let Some(hit) = self.matches.active() {
            cx.emit(FindEvent::ActiveHit(hit));
        }
        cx.notify();
    }

    pub fn matches(&self) -> &FindMatches {
        &self.matches
    }

    #[cfg(test)]
    pub(crate) fn query_text(&self, cx: &App) -> String {
        self.query.read(cx).text().to_owned()
    }

    fn recompute(&mut self, query: &str, cx: &mut Context<Self>) {
        let prefer = self.matches.active();
        self.matches = FindMatches::from_hits(self.search(query), prefer);
        if let Some(hit) = self.matches.active() {
            cx.emit(FindEvent::ActiveHit(hit));
        }
        cx.notify();
    }

    fn search(&self, query: &str) -> Vec<FindHit> {
        self.document
            .as_ref()
            .map(|document| find_in_blocks(&document.blocks, query))
            .unwrap_or_default()
    }
}

/// "2 / 13" while there are matches, "No results" for a query with none (Electron's copy), and
/// nothing while the query is empty.
fn find_count_label(query_is_empty: bool, position: Option<(usize, usize)>) -> String {
    match position {
        Some((index, total)) => format!("{index} / {total}"),
        None if query_is_empty => String::new(),
        None => "No results".into(),
    }
}

impl Render for FindOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme =
            Theme::resolve(self.theme_mode, window.appearance()).scaled(active_ui_scale(cx));
        let count = find_count_label(
            self.query.read(cx).text().is_empty(),
            self.matches.position(),
        );
        let has_matches = !self.matches.hits.is_empty();
        let step_color = if has_matches {
            theme.foreground
        } else {
            theme.muted_foreground.opacity(0.5)
        };
        paint_overlay_edge(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .pl(px(10.0))
                .pr(px(6.0))
                .h(px(38.0))
                .rounded(px(Metrics::RADIUS))
                .bg(theme.surface_raised),
            theme,
        )
        .font_family(Metrics::FONT_SANS)
        .child(icon("icons/search.svg", theme.muted_foreground, 14.0))
        .child(
            div()
                .flex_grow()
                .min_w_0()
                .text_size(px(13.0))
                .text_color(theme.foreground)
                .child(self.query.clone()),
        )
        .child(
            div()
                .flex_none()
                .pr(px(4.0))
                .font(tabular_sans(FontWeight::NORMAL))
                .text_size(px(11.5))
                .text_color(theme.muted_foreground)
                .child(count),
        )
        .child(
            div()
                .flex_none()
                .w(px(1.0))
                .h(px(18.0))
                .bg(theme.border_subtle),
        )
        .child(find_step_button(
            "find-prev",
            "icons/chevron-up.svg",
            step_color,
            theme,
            cx.listener(|this, _, _, cx| this.advance(true, cx)),
        ))
        .child(find_step_button(
            "find-next",
            "icons/chevron-down.svg",
            step_color,
            theme,
            cx.listener(|this, _, _, cx| this.advance(false, cx)),
        ))
        .child(compact_icon_button(
            "find-close",
            "icons/x.svg",
            24.0,
            13.0,
            theme,
            cx.listener(|_, _, _, cx| cx.emit(FindEvent::Dismissed)),
        ))
    }
}

/// Previous/next read as dimmed when there is nothing to step through.
fn find_step_button(
    id: &'static str,
    icon_path: &'static str,
    color: Hsla,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .tab_index(0)
        .focusable()
        .flex()
        .items_center()
        .justify_center()
        .size(px(24.0))
        .flex_none()
        .rounded(px(5.0))
        .cursor_pointer()
        .hover(move |style| style.bg(theme.muted))
        .active(|style| style.opacity(0.8))
        .focus(move |style| style.border_1().border_color(theme.primary))
        .on_click(on_click)
        .child(icon(icon_path, color, 14.0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    OpenFile,
    OpenFolder,
    CloseTab,
    NextTab,
    PreviousTab,
    ToggleSidebar,
    SidebarRecents,
    SidebarFolder,
    SidebarOutline,
    ToggleWideMode,
    ToggleSplitView,
    LineWidth(LineWidth),
    ThemeSystem,
    ThemeLight,
    ThemeDark,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    FindInDocument,
    OpenSettings,
    OpenShortcuts,
    CheckForUpdates,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandSpec {
    pub id: CommandId,
    pub title: &'static str,
    pub keys: Option<&'static str>,
}

pub fn command_catalog() -> &'static [CommandSpec] {
    &[
        CommandSpec {
            id: CommandId::OpenFile,
            title: "Open File",
            keys: Some("⌘O"),
        },
        CommandSpec {
            id: CommandId::OpenFolder,
            title: "Open Folder",
            keys: Some("⇧⌘O"),
        },
        CommandSpec {
            id: CommandId::CloseTab,
            title: "Close Tab",
            keys: Some("⌘W"),
        },
        CommandSpec {
            id: CommandId::NextTab,
            title: "Next Tab",
            keys: Some("⌥⌘→"),
        },
        CommandSpec {
            id: CommandId::PreviousTab,
            title: "Previous Tab",
            keys: Some("⌥⌘←"),
        },
        CommandSpec {
            id: CommandId::ToggleSidebar,
            title: "Toggle Sidebar",
            keys: Some("⌘B"),
        },
        CommandSpec {
            id: CommandId::SidebarRecents,
            title: "Sidebar: Recents",
            keys: Some("⌃1"),
        },
        CommandSpec {
            id: CommandId::SidebarFolder,
            title: "Sidebar: Folder",
            keys: Some("⌃2"),
        },
        CommandSpec {
            id: CommandId::SidebarOutline,
            title: "Sidebar: Outline",
            keys: Some("⌃3"),
        },
        CommandSpec {
            id: CommandId::ToggleWideMode,
            title: "Toggle Wide Mode",
            keys: Some("⇧⌘W"),
        },
        CommandSpec {
            id: CommandId::ToggleSplitView,
            title: "Toggle Split View",
            keys: Some("⌘\\"),
        },
        CommandSpec {
            id: CommandId::LineWidth(LineWidth::Narrow),
            title: "Line Width: Narrow",
            keys: None,
        },
        CommandSpec {
            id: CommandId::LineWidth(LineWidth::Medium),
            title: "Line Width: Medium",
            keys: None,
        },
        CommandSpec {
            id: CommandId::LineWidth(LineWidth::Wide),
            title: "Line Width: Wide",
            keys: None,
        },
        CommandSpec {
            id: CommandId::LineWidth(LineWidth::Full),
            title: "Line Width: Full",
            keys: None,
        },
        CommandSpec {
            id: CommandId::ThemeSystem,
            title: "Theme: System",
            keys: None,
        },
        CommandSpec {
            id: CommandId::ThemeLight,
            title: "Theme: Light",
            keys: None,
        },
        CommandSpec {
            id: CommandId::ThemeDark,
            title: "Theme: Dark",
            keys: None,
        },
        CommandSpec {
            id: CommandId::ZoomIn,
            title: "Zoom In",
            keys: Some("⌘="),
        },
        CommandSpec {
            id: CommandId::ZoomOut,
            title: "Zoom Out",
            keys: Some("⌘-"),
        },
        CommandSpec {
            id: CommandId::ZoomReset,
            title: "Zoom Reset",
            keys: Some("⌘0"),
        },
        CommandSpec {
            id: CommandId::FindInDocument,
            title: "Find in Document",
            keys: Some("⌘F"),
        },
        CommandSpec {
            id: CommandId::OpenSettings,
            title: "Settings",
            keys: Some("⌘,"),
        },
        CommandSpec {
            id: CommandId::OpenShortcuts,
            title: "Keyboard Shortcuts",
            keys: Some("⌘/"),
        },
        #[cfg(target_os = "macos")]
        CommandSpec {
            id: CommandId::CheckForUpdates,
            title: "Check for Updates",
            keys: None,
        },
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteItem {
    Command(&'static CommandSpec),
    File { path: PathBuf, recent: bool },
}

pub fn palette_items(
    query: &str,
    recents: &Recents,
    workspace_files: &[PathBuf],
) -> Vec<PaletteItem> {
    let workspace_set = workspace_files.iter().cloned().collect::<HashSet<_>>();
    let files = workspace_files
        .iter()
        .cloned()
        .map(|path| PaletteItem::File {
            path,
            recent: false,
        })
        .chain(recents.iter().filter_map(|path| {
            if workspace_set.contains(path) {
                None
            } else {
                Some(PaletteItem::File {
                    path: path.to_owned(),
                    recent: true,
                })
            }
        }))
        .collect::<Vec<_>>();

    if query.is_empty() {
        return command_catalog()
            .iter()
            .map(PaletteItem::Command)
            .chain(files)
            .collect();
    }

    let mut scored = Vec::new();
    for spec in command_catalog() {
        if let Some(score) = subsequence_score(query, spec.title) {
            scored.push((score, PaletteItem::Command(spec)));
        }
    }
    for item in files {
        let PaletteItem::File { path, recent } = item else {
            continue;
        };
        if let Some(score) = subsequence_score(query, &path_label(&path)) {
            scored.push((score, PaletteItem::File { path, recent }));
        }
    }
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    scored.into_iter().map(|(_, item)| item).collect()
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_owned()
}

fn path_hint(path: &Path, recent: bool) -> String {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .map(|name| name.to_owned())
        .unwrap_or_else(|| {
            if recent {
                "Recent".into()
            } else {
                String::new()
            }
        })
}

fn subsequence_score(query: &str, candidate: &str) -> Option<u32> {
    let query = query.to_lowercase();
    let candidate_lower = candidate.to_lowercase();
    let mut score = 0u32;
    let mut from = 0usize;
    let mut run = 0u32;
    for needle in query.chars() {
        let rest = &candidate_lower[from..];
        let pos = rest.find(needle)?;
        if pos == 0 {
            run += 1;
            score += 8 + run;
        } else {
            run = 0;
            score += 1;
        }
        if from == 0 && pos == 0 {
            score += 12;
        }
        from += pos + needle.len_utf8();
    }
    Some(score)
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteAction {
    Run(CommandId),
    Open(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteEvent {
    Invoked(PaletteAction),
    Dismissed,
}

pub struct PaletteOverlay {
    query: Entity<Field>,
    items: Vec<PaletteItem>,
    selected: usize,
    theme_mode: ThemeMode,
    _query_events: Subscription,
}

impl EventEmitter<PaletteEvent> for PaletteOverlay {}

impl PaletteOverlay {
    pub fn new(
        recents: Recents,
        workspace_files: Vec<PathBuf>,
        theme_mode: ThemeMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let items = palette_items("", &recents, &workspace_files);
        let query = cx.new(|cx| Field::search("Search files and commands…", window, cx));
        let query_events = cx.subscribe(&query, {
            let recents = recents.clone();
            let workspace_files = workspace_files.clone();
            move |this, field, event, cx| match event {
                FieldEvent::Edited => {
                    let text = field.read(cx).text().to_owned();
                    this.items = palette_items(&text, &recents, &workspace_files);
                    if this.selected >= this.items.len() {
                        this.selected = this.items.len().saturating_sub(1);
                    }
                    cx.notify();
                }
                FieldEvent::Submitted { .. } => this.invoke(cx),
                FieldEvent::Cancelled => cx.emit(PaletteEvent::Dismissed),
            }
        });
        Self {
            query,
            items,
            selected: 0,
            theme_mode,
            _query_events: query_events,
        }
    }

    fn invoke(&mut self, cx: &mut Context<Self>) {
        let Some(item) = self.items.get(self.selected) else {
            return;
        };
        let action = match item {
            PaletteItem::Command(spec) => PaletteAction::Run(spec.id),
            PaletteItem::File { path, .. } => PaletteAction::Open(path.clone()),
        };
        cx.emit(PaletteEvent::Invoked(action));
    }

    fn select_next(&mut self, _: &SelectNext, _: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.items.len();
        cx.notify();
    }

    fn select_prev(&mut self, _: &SelectPrev, _: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        self.selected = self.selected.checked_sub(1).unwrap_or(self.items.len() - 1);
        cx.notify();
    }
}

impl Render for PaletteOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme =
            Theme::resolve(self.theme_mode, window.appearance()).scaled(active_ui_scale(cx));
        self.query.update(cx, |field, _| field.apply_theme(theme));
        let selected = self.selected;
        let commands = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, PaletteItem::Command(_)))
            .collect::<Vec<_>>();
        let files = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, PaletteItem::File { .. }))
            .collect::<Vec<_>>();
        let mut list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .max_h(px(400.0))
            .overflow_y_scroll();
        if self.items.is_empty() {
            list = list.child(
                div()
                    .px(px(theme.ui.space(10.0)))
                    .py(px(theme.ui.space(16.0)))
                    .text_center()
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(theme.ui.text(12.0)))
                    .text_color(theme.muted_foreground)
                    .child("No matching files or commands"),
            );
        } else {
            if !commands.is_empty() {
                list = list.child(palette_heading("Actions", theme));
                for (index, item) in commands {
                    list = list.child(palette_row(index, item, index == selected, theme, cx));
                }
            }
            if !files.is_empty() {
                list = list.child(palette_heading("Files", theme));
                for (index, item) in files {
                    list = list.child(palette_row(index, item, index == selected, theme, cx));
                }
            }
        }
        overlay_surface(480.0, theme)
            .key_context("Palette")
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_prev))
            .child(
                div()
                    .flex()
                    .items_center()
                    .px(px(theme.ui.space(12.0)))
                    .h(px(theme.ui.space(44.0)))
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_well)
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(theme.ui.text(13.0)))
                    .child(self.query.clone()),
            )
            .child(list)
            .child(
                div()
                    .px(px(theme.ui.space(12.0)))
                    .py(px(theme.ui.space(8.0)))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(theme.ui.space(10.0)))
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(theme.ui.text(10.0)))
                    .text_color(theme.muted_foreground)
                    .child(key_hint("↵", theme))
                    .child("Open")
                    .child(key_hint("esc", theme))
                    .child("Dismiss"),
            )
    }
}

fn palette_heading(title: &'static str, theme: Theme) -> impl IntoElement {
    div()
        .px(px(theme.ui.space(10.0)))
        .pt(px(theme.ui.space(8.0)))
        .pb(px(theme.ui.space(4.0)))
        .font_family(Metrics::FONT_SANS)
        .text_size(px(theme.ui.text(10.0)))
        .text_color(theme.muted_foreground)
        .child(title)
}

fn palette_row(
    index: usize,
    item: &PaletteItem,
    selected: bool,
    theme: Theme,
    cx: &mut Context<PaletteOverlay>,
) -> impl IntoElement {
    let (icon_path, label, shortcut, file_hint) = match item {
        PaletteItem::Command(spec) => ("icons/command.svg", spec.title.to_owned(), spec.keys, None),
        PaletteItem::File { path, recent } => (
            "icons/file.svg",
            path_label(path),
            None,
            Some(path_hint(path, *recent)),
        ),
    };
    let mut row = list_row(
        ("palette-item", index),
        ListRowStyle {
            selected,
            indent: 0.0,
        },
        theme,
    )
    .gap(px(theme.ui.space(8.0)))
    .on_click(cx.listener(move |this, _, _, cx| {
        this.selected = index;
        this.invoke(cx);
    }))
    .child(icon(icon_path, theme.muted_foreground, 14.0))
    .child(div().min_w_0().flex_grow().truncate().child(label));
    if let Some(keys) = shortcut {
        row = row.child(key_hint(keys, theme));
    } else if let Some(hint) = file_hint {
        row = row.child(
            div()
                .flex_none()
                .text_color(theme.muted_foreground)
                .text_size(px(theme.ui.text(11.0)))
                .child(hint),
        );
    }
    row
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsEvent {
    Edited(PrefEdit),
    CheckForUpdates,
    DownloadUpdate,
    ViewReleases,
    InstallUpdate,
    Dismissed,
}

pub struct SettingsPanel {
    prefs: Prefs,
    update: UpdateUi,
    focus_handle: FocusHandle,
}

impl EventEmitter<SettingsEvent> for SettingsPanel {}

impl SettingsPanel {
    pub fn new(
        prefs: Prefs,
        update: UpdateUi,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        Self {
            prefs,
            update,
            focus_handle,
        }
    }

    pub fn refresh(&mut self, prefs: Prefs, update: UpdateUi, cx: &mut Context<Self>) {
        self.prefs = prefs;
        self.update = update;
        cx.notify();
    }
}

impl Focusable for SettingsPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Fixed label column so every control in Settings starts on the same x.
const SETTINGS_LABEL_WIDTH: f32 = 120.0;
const SETTINGS_WIDTH: f32 = 520.0;

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::resolve(self.prefs.theme_mode, window.appearance())
            .scaled(self.prefs.interface_scale);
        let prefs = self.prefs;
        let max_height = (f32::from(window.viewport_size().height) - 96.0).max(240.0);

        let appearance = settings_group("Appearance", theme)
            .child(settings_row(
                "Theme",
                None,
                segmented(
                    [
                        (ThemeMode::System, "System", "icons/monitor.svg"),
                        (ThemeMode::Light, "Light", "icons/sun.svg"),
                        (ThemeMode::Dark, "Dark", "icons/moon.svg"),
                    ]
                    .map(|(mode, label, icon_path)| Segment {
                        label,
                        selected: prefs.theme_mode == mode,
                        edit: PrefEdit::Theme(mode),
                        font: None,
                        icon: Some(icon_path),
                    }),
                    theme,
                    cx,
                ),
                theme,
            ))
            .child(settings_row(
                "Interface size",
                None,
                segmented(
                    [
                        (InterfaceScale::Compact, "Compact"),
                        (InterfaceScale::Comfortable, "Comfortable"),
                        (InterfaceScale::Large, "Large"),
                    ]
                    .map(|(scale, label)| {
                        Segment::text(
                            label,
                            prefs.interface_scale == scale,
                            PrefEdit::InterfaceScale(scale),
                        )
                    }),
                    theme,
                    cx,
                ),
                theme,
            ));

        let reading = settings_group("Reading", theme)
            .child(settings_row(
                "Text font",
                None,
                segmented(
                    [
                        (ContentFont::Inter, "Inter"),
                        (ContentFont::Charter, "Charter"),
                        (ContentFont::SystemSans, "System"),
                        (ContentFont::Georgia, "Georgia"),
                    ]
                    .map(|(font, label)| Segment {
                        font: Some(font.family()),
                        ..Segment::text(
                            label,
                            prefs.content_font == font,
                            PrefEdit::ContentFont(font),
                        )
                    }),
                    theme,
                    cx,
                ),
                theme,
            ))
            .child(settings_row(
                "Code font",
                None,
                segmented(
                    [
                        (CodeFont::GeistMono, "Geist"),
                        (CodeFont::SfMono, "SF Mono"),
                        (CodeFont::JetBrainsMono, "JetBrains"),
                        (CodeFont::SystemMono, "System"),
                    ]
                    .map(|(font, label)| Segment {
                        font: Some(font.family()),
                        ..Segment::text(label, prefs.code_font == font, PrefEdit::CodeFont(font))
                    }),
                    theme,
                    cx,
                ),
                theme,
            ))
            .child(settings_row(
                "Line width",
                None,
                segmented(
                    LineWidth::ALL.map(|width| {
                        Segment::text(
                            width.label(),
                            prefs.reader_width.line_width() == width,
                            PrefEdit::LineWidth(width),
                        )
                    }),
                    theme,
                    cx,
                ),
                theme,
            ))
            .child(settings_row(
                "Text size",
                Some("⌘+ / ⌘−"),
                zoom_stepper(prefs.zoom.percent(), theme, cx),
                theme,
            ));

        overlay_surface(SETTINGS_WIDTH, theme)
            .id("settings-panel")
            .track_focus(&self.focus_handle)
            .max_h(px(max_height))
            .overflow_y_scroll()
            .pt(px(theme.ui.space(18.0)))
            .px(px(theme.ui.space(20.0)))
            .pb(px(theme.ui.space(16.0)))
            .flex()
            .flex_col()
            .font_family(Metrics::FONT_SANS)
            .text_size(px(theme.ui.text(13.0)))
            .text_color(theme.foreground)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(settings_heading("Settings", theme))
                    .child(compact_icon_button(
                        "settings-close",
                        "icons/x.svg",
                        24.0,
                        13.0,
                        theme,
                        cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Dismissed)),
                    )),
            )
            .child(settings_preview(prefs, theme))
            .child(appearance)
            .child(reading)
            .when(cfg!(target_os = "macos"), |panel| {
                panel.child(settings_updates(&self.update, prefs.auto_update, theme, cx))
            })
            .child(settings_footer(theme, cx))
    }
}

fn settings_preview(prefs: Prefs, theme: Theme) -> impl IntoElement {
    div()
        .mt(px(theme.ui.space(14.0)))
        .px(px(theme.ui.space(16.0)))
        .py(px(theme.ui.space(14.0)))
        .rounded(px(8.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.background)
        .font_family(prefs.content_font.family())
        .text_color(theme.foreground)
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(theme.ui.text(19.0)))
                .line_height(px(19.0 * 1.3))
                .child("A quiet place to read"),
        )
        .child(
            div()
                .mt(px(theme.ui.space(4.0)))
                .text_size(px(READER_FONT_SIZE))
                .line_height(px(READER_FONT_SIZE * READER_LINE_HEIGHT))
                .child("Mdow re-renders the moment you save."),
        )
        .child(
            div()
                .mt(px(theme.ui.space(6.0)))
                .font_family(prefs.code_font.family())
                .text_size(px(theme.ui.text(13.0)))
                .text_color(theme.muted_foreground)
                .child("let width = 68;"),
        )
}

fn settings_heading(title: &'static str, theme: Theme) -> impl IntoElement {
    div()
        .font_weight(FontWeight::SEMIBOLD)
        .text_size(px(theme.ui.text(15.0)))
        .text_color(theme.foreground)
        .child(title)
}

fn settings_group(title: &'static str, theme: Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .mt(px(theme.ui.space(14.0)))
                .mb(px(theme.ui.space(4.0)))
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(theme.ui.text(11.0)))
                .text_color(theme.muted_foreground)
                .child(title.to_uppercase()),
        )
        .child(
            div()
                .h(px(theme.ui.space(1.0)))
                .mb(px(theme.ui.space(6.0)))
                .bg(theme.border_subtle),
        )
}

fn settings_row(
    label: impl Into<SharedString>,
    hint: Option<&'static str>,
    control: impl IntoElement,
    theme: Theme,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(theme.ui.space(16.0)))
        .min_h(px(36.0))
        .child(
            div()
                .w(px(SETTINGS_LABEL_WIDTH))
                .flex_none()
                .flex()
                .flex_col()
                .child(label.into())
                .when_some(hint, |label, hint| {
                    label.child(
                        div()
                            .mt(px(theme.ui.space(1.0)))
                            .text_size(px(theme.ui.text(11.5)))
                            .text_color(theme.muted_foreground)
                            .child(hint),
                    )
                }),
        )
        .child(div().flex_1().min_w_0().child(control))
}

#[derive(Clone, Copy)]
struct Segment {
    label: &'static str,
    selected: bool,
    edit: PrefEdit,
    /// Font options preview in their own typeface.
    font: Option<&'static str>,
    icon: Option<&'static str>,
}

impl Segment {
    fn text(label: &'static str, selected: bool, edit: PrefEdit) -> Self {
        Self {
            label,
            selected,
            edit,
            font: None,
            icon: None,
        }
    }
}

/// Equal-width segments so selected states line up from row to row.
fn segmented<const N: usize>(
    segments: [Segment; N],
    theme: Theme,
    cx: &mut Context<SettingsPanel>,
) -> impl IntoElement {
    let mut track = div()
        .flex()
        .h(px(theme.ui.space(30.0)))
        .p(px(theme.ui.space(2.0)))
        .gap(px(theme.ui.space(2.0)))
        .rounded(px(7.0))
        .bg(theme.surface_well);
    for segment in segments {
        track = track.child(segment_button(segment, theme, cx));
    }
    track
}

fn segment_button(
    segment: Segment,
    theme: Theme,
    cx: &mut Context<SettingsPanel>,
) -> impl IntoElement {
    let Segment {
        label,
        selected,
        edit,
        font,
        icon: icon_path,
    } = segment;
    let id = format!("{label}-{edit:?}");
    let text_color = if selected {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    div()
        .id(SharedString::from(id.clone()))
        .debug_selector(move || id)
        .tab_index(0)
        .focusable()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(theme.ui.space(6.0)))
        .rounded(px(5.0))
        .border_1()
        .text_size(px(theme.ui.text(12.5)))
        .text_color(text_color)
        .when_some(font, |segment, family| segment.font_family(family))
        .map(|segment| {
            if selected {
                paint_selected_segment(segment, theme)
            } else {
                segment
                    .border_color(theme.border.opacity(0.0))
                    .cursor_pointer()
                    .hover(move |style| style.text_color(theme.foreground))
            }
        })
        .focus(move |style| style.border_color(theme.primary))
        .on_click(cx.listener(move |_, _, _, cx| cx.emit(SettingsEvent::Edited(edit))))
        .when_some(icon_path, |segment, path| {
            segment.child(icon(path, text_color, 13.0))
        })
        .child(div().truncate().child(label))
}

fn paint_selected_segment(segment: Stateful<Div>, theme: Theme) -> Stateful<Div> {
    match theme.color_scheme {
        ColorScheme::Light => segment
            .bg(theme.surface_raised)
            .border_color(theme.border_subtle)
            .shadow_sm(),
        ColorScheme::Dark => segment.bg(theme.surface_raised).border_color(theme.border),
    }
}

fn zoom_stepper(percent: u16, theme: Theme, cx: &mut Context<SettingsPanel>) -> impl IntoElement {
    let emit = |edit| cx.listener(move |_, _, _, cx| cx.emit(SettingsEvent::Edited(edit)));
    div()
        .flex()
        .items_center()
        // Hug the − 100% + controls instead of stretching across the control column.
        .w(px(2.0 + 26.0 + 52.0 + 26.0 + 2.0))
        .h(px(theme.ui.space(30.0)))
        .p(px(theme.ui.space(2.0)))
        .rounded(px(7.0))
        .bg(theme.surface_well)
        .child(compact_icon_button(
            "settings-zoom-out",
            "icons/minus.svg",
            26.0,
            13.0,
            theme,
            emit(PrefEdit::ZoomOut),
        ))
        .child(
            div()
                .w(px(52.0))
                .text_center()
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(theme.ui.text(12.5)))
                .child(format!("{percent}%")),
        )
        .child(compact_icon_button(
            "settings-zoom-in",
            "icons/plus.svg",
            26.0,
            13.0,
            theme,
            emit(PrefEdit::ZoomIn),
        ))
}

/// Settings status line for the updater: the same states as the banner, phrased as a status
/// rather than an announcement.
fn update_status(update: &UpdateUi, auto_update: bool) -> String {
    match update {
        UpdateUi::Idle | UpdateUi::Checking { manual: false } if auto_update => {
            "Checks automatically".into()
        }
        UpdateUi::Idle | UpdateUi::Checking { manual: false } => "Automatic checks are off".into(),
        UpdateUi::Checking { manual: true } => "Checking…".into(),
        UpdateUi::Available { version } => format!("{version} is available"),
        UpdateUi::Downloading { percent, .. } => format!("Downloading… {percent}%"),
        UpdateUi::Ready { .. } => "Ready. Restart to apply.".into(),
        UpdateUi::UpToDate { .. } => "Up to date".into(),
        UpdateUi::Failed { .. } => "Couldn't check. Try again later.".into(),
        UpdateUi::Unavailable { .. } => "No automatic updates in this build".into(),
    }
}

fn settings_updates(
    update: &UpdateUi,
    auto_update: bool,
    theme: Theme,
    cx: &mut Context<SettingsPanel>,
) -> impl IntoElement {
    let version = crate::sparkle::app_version()
        .map(|version| format!("Mdow {version}"))
        .unwrap_or_else(|| "Mdow Native".into());
    let (label, id, event) = if update.can_install() {
        (
            "Restart",
            "settings-install-update",
            SettingsEvent::InstallUpdate,
        )
    } else if update.can_download() {
        (
            "Download",
            "settings-download-update",
            SettingsEvent::DownloadUpdate,
        )
    } else if update.needs_manual_download() {
        (
            "Open Releases",
            "settings-check-updates",
            SettingsEvent::ViewReleases,
        )
    } else {
        (
            "Check now",
            "settings-check-updates",
            SettingsEvent::CheckForUpdates,
        )
    };
    settings_group("Updates", theme)
        .child(settings_row(
            version,
            None,
            div()
                .flex()
                .items_center()
                .gap(px(theme.ui.space(10.0)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(theme.ui.text(12.0)))
                        .text_color(theme.muted_foreground)
                        .child(update_status(update, auto_update)),
                )
                .child(settings_button(label, id, theme, event, cx)),
            theme,
        ))
        .child(settings_row(
            "Auto-check",
            None,
            switch(
                "settings-auto-update",
                auto_update,
                theme,
                cx.listener(move |_, _, _, cx| {
                    cx.emit(SettingsEvent::Edited(PrefEdit::AutoUpdate(!auto_update)))
                }),
            ),
            theme,
        ))
}

fn switch(
    id: &'static str,
    on: bool,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (track, knob_x) = if on {
        (theme.primary, 16.0)
    } else {
        (theme.border, 2.0)
    };
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .tab_index(0)
        .focusable()
        .relative()
        .flex_none()
        .w(px(34.0))
        .h(px(theme.ui.space(20.0)))
        .rounded_full()
        .border_1()
        .border_color(track)
        .bg(track)
        .cursor_pointer()
        .focus(move |style| style.border_color(theme.foreground.opacity(0.5)))
        .on_click(on_click)
        .child(
            div()
                .absolute()
                .top(px(1.0))
                .left(px(knob_x - 1.0))
                .size(px(theme.ui.space(16.0)))
                .rounded_full()
                .bg(gpui::white())
                .shadow_sm(),
        )
}

fn settings_button(
    label: &'static str,
    id: &'static str,
    theme: Theme,
    event: SettingsEvent,
    cx: &mut Context<SettingsPanel>,
) -> impl IntoElement {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .tab_index(0)
        .focusable()
        .flex_none()
        .px(px(theme.ui.space(10.0)))
        .h(px(theme.ui.space(28.0)))
        .flex()
        .items_center()
        .rounded(px(6.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.card)
        .text_size(px(theme.ui.text(12.0)))
        .cursor_pointer()
        .hover(move |style| style.bg(theme.muted))
        .active(|style| style.opacity(0.82))
        .focus(move |style| style.border_color(theme.primary))
        .on_click(cx.listener(move |_, _, _, cx| cx.emit(event)))
        .child(label)
}

/// "Restore defaults" is a quiet footer action, not a peer of the font choices.
fn settings_footer(theme: Theme, cx: &mut Context<SettingsPanel>) -> impl IntoElement {
    div()
        .mt(px(theme.ui.space(14.0)))
        .pt(px(theme.ui.space(12.0)))
        .border_t_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .justify_between()
        .text_size(px(theme.ui.text(12.0)))
        .text_color(theme.muted_foreground)
        .child(
            div()
                .id("settings-restore-defaults")
                .debug_selector(|| "settings-restore-defaults".into())
                .tab_index(0)
                .focusable()
                .flex()
                .items_center()
                .gap(px(theme.ui.space(6.0)))
                .h(px(theme.ui.space(24.0)))
                .px(px(theme.ui.space(6.0)))
                .ml(px(-6.0))
                .rounded(px(5.0))
                .border_1()
                .border_color(theme.border.opacity(0.0))
                .cursor_pointer()
                .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
                .focus(move |style| style.border_color(theme.primary))
                .on_click(
                    cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Edited(PrefEdit::ResetAll))),
                )
                .child(icon(
                    "icons/rotate-ccw.svg",
                    theme.muted_foreground,
                    theme.ui.space(12.0),
                ))
                .child("Restore defaults"),
        )
        .child(
            div()
                .text_size(px(theme.ui.text(11.5)))
                .child("Changes save automatically"),
        )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShortcutsEvent {
    Dismissed,
}

pub struct ShortcutsCard {
    theme_mode: ThemeMode,
    focus_handle: FocusHandle,
}

impl EventEmitter<ShortcutsEvent> for ShortcutsCard {}

impl ShortcutsCard {
    pub fn new(theme_mode: ThemeMode, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        Self {
            theme_mode,
            focus_handle,
        }
    }

    fn theme(&self, window: &Window, cx: &App) -> Theme {
        Theme::resolve(self.theme_mode, window.appearance()).scaled(active_ui_scale(cx))
    }
}

impl Focusable for ShortcutsCard {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ShortcutsCard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme(window, cx);
        let mut list = div().flex().flex_col().gap(px(theme.ui.space(6.0)));
        for spec in command_catalog() {
            if let Some(keys) = spec.keys {
                list = list.child(
                    div()
                        .flex()
                        .justify_between()
                        .h(px(theme.ui.space(24.0)))
                        .font_family(Metrics::FONT_SANS)
                        .text_size(px(theme.ui.text(12.0)))
                        .text_color(theme.foreground)
                        .child(spec.title)
                        .child(div().text_color(theme.muted_foreground).child(keys)),
                );
            }
        }
        overlay_surface(420.0, theme)
            .track_focus(&self.focus_handle)
            .p(px(theme.ui.space(16.0)))
            .child(settings_heading("Keyboard shortcuts", theme))
            .child(div().mt(px(theme.ui.space(10.0))).child(list))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::InlineSpan;
    use crate::theme::ColorScheme;
    use gpui::{TestAppContext, WindowAppearance};

    #[gpui::test]
    fn shortcuts_card_follows_a_forced_theme_preference(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| ShortcutsCard::new(ThemeMode::Dark, window, cx))
            })
            .unwrap()
        });

        window
            .update(cx, |card, window, cx| {
                assert_eq!(window.appearance(), WindowAppearance::Light);
                assert_eq!(card.theme(window, cx).color_scheme, ColorScheme::Dark);
            })
            .unwrap();
    }

    #[test]
    fn find_count_reads_position_no_results_or_nothing() {
        assert_eq!(find_count_label(true, None), "");
        assert_eq!(find_count_label(false, None), "No results");
        assert_eq!(find_count_label(false, Some((2, 13))), "2 / 13");
    }

    #[test]
    fn update_status_reflects_the_automatic_check_preference() {
        assert_eq!(update_status(&UpdateUi::Idle, true), "Checks automatically");
        assert_eq!(
            update_status(&UpdateUi::Idle, false),
            "Automatic checks are off"
        );
        assert_eq!(
            update_status(&UpdateUi::UpToDate { manual: true }, false),
            "Up to date"
        );
    }

    #[test]
    fn find_matches_across_painted_soft_line_breaks() {
        let blocks = vec![DocumentBlock::Paragraph(vec![
            InlineSpan::Text("hello".into()),
            InlineSpan::SoftBreak,
            InlineSpan::Text("world".into()),
        ])];
        let hits = find_in_blocks(&blocks, "hello world");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].range_start, 0);
        assert!(find_in_blocks(&blocks, "hello\nworld").is_empty());
    }

    #[test]
    fn palette_empty_query_lists_commands_then_workspace_then_extra_recents() {
        let recents = Recents::from_paths(vec![
            PathBuf::from("/notes/a.md"),
            PathBuf::from("/vault/readme.md"),
        ]);
        let workspace = vec![
            PathBuf::from("/vault/readme.md"),
            PathBuf::from("/vault/guide.md"),
        ];
        let items = palette_items("", &recents, &workspace);
        assert!(matches!(items.first(), Some(PaletteItem::Command(_))));
        let files = items
            .iter()
            .filter_map(|item| match item {
                PaletteItem::File { path, recent } => Some((path.as_path(), *recent)),
                PaletteItem::Command(_) => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            files,
            vec![
                (Path::new("/vault/readme.md"), false),
                (Path::new("/vault/guide.md"), false),
                (Path::new("/notes/a.md"), true),
            ]
        );
    }

    #[test]
    fn palette_filters_by_subsequence() {
        let items = palette_items("thm drk", &Recents::default(), &[]);
        assert!(items.iter().any(|item| matches!(
            item,
            PaletteItem::Command(spec) if spec.id == CommandId::ThemeDark
        )));
    }

    #[test]
    fn palette_query_matches_workspace_filenames() {
        let items = palette_items(
            "guide",
            &Recents::default(),
            &[PathBuf::from("/vault/guide.md")],
        );
        assert!(items.iter().any(|item| matches!(
            item,
            PaletteItem::File { path, recent: false } if path.ends_with("guide.md")
        )));
    }
}
