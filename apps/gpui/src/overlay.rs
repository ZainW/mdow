use crate::actions::Dismiss;
use crate::document::DocumentBlock;
use crate::prefs::{
    CodeFont, ColumnWidth, ContentFont, InterfaceScale, PrefEdit, Prefs, ThemeMode,
};
use crate::session::Recents;
use crate::syntax::PreparedDocument;
use crate::theme::{Metrics, Theme};
use crate::ui::field::{Field, FieldEvent};
use crate::ui::primitives::compact_icon_button;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight,
    IntoElement, Render, SharedString, Subscription, Window, div, prelude::*, px,
};
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

    pub fn refresh_settings(&self, prefs: &Prefs, cx: &mut App) {
        if let Some(OverlayView::Settings(view)) = self.open.as_ref().map(|open| &open.view) {
            view.update(cx, |panel, cx| panel.refresh(*prefs, cx));
        }
    }

    pub fn render_layer(&self, theme: Theme) -> Option<AnyElement> {
        let open = self.open.as_ref()?;
        Some(match &open.view {
            OverlayView::Find(view) => find_layer(view.clone()),
            OverlayView::Palette(view) => modal_layer(view.clone(), theme),
            OverlayView::Settings(view) => modal_layer(view.clone(), theme),
            OverlayView::Shortcuts(view) => modal_layer(view.clone(), theme),
        })
    }
}

fn find_layer(view: Entity<FindOverlay>) -> AnyElement {
    div()
        .absolute()
        .top(px(48.0))
        .right(px(16.0))
        .w(px(360.0))
        .child(view)
        .into_any_element()
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
        let haystack = text.painted_plain_text().to_lowercase();
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
    _query_events: Subscription,
}

impl EventEmitter<FindEvent> for FindOverlay {}

impl FindOverlay {
    pub fn new(
        document: Option<Arc<PreparedDocument>>,
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
            _query_events: query_events,
        }
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

impl Render for FindOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());
        let count = match self.matches.position() {
            None if self.query.read(cx).text().is_empty() => String::new(),
            None => "0 of 0".into(),
            Some((index, total)) => format!("{index} of {total}"),
        };
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .h(px(36.0))
            .rounded(px(Metrics::RADIUS))
            .border_1()
            .border_color(theme.border)
            .bg(theme.card)
            .shadow_sm()
            .child(
                div()
                    .flex_grow()
                    .min_w_0()
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(12.0))
                    .text_color(theme.foreground)
                    .child(self.query.clone()),
            )
            .child(
                div()
                    .w(px(64.0))
                    .flex_none()
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .child(count),
            )
            .child(compact_icon_button(
                "find-prev",
                "icons/chevron-up.svg",
                24.0,
                12.0,
                theme,
                cx.listener(|this, _, _, cx| this.advance(true, cx)),
            ))
            .child(compact_icon_button(
                "find-next",
                "icons/chevron-down.svg",
                24.0,
                12.0,
                theme,
                cx.listener(|this, _, _, cx| this.advance(false, cx)),
            ))
            .child(compact_icon_button(
                "find-close",
                "icons/x.svg",
                24.0,
                12.0,
                theme,
                cx.listener(|_, _, _, cx| cx.emit(FindEvent::Dismissed)),
            ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    OpenFile,
    OpenFolder,
    CloseTab,
    ToggleSidebar,
    SidebarRecents,
    SidebarFolder,
    SidebarOutline,
    ToggleWideMode,
    ColumnStandard,
    ColumnComfortable,
    ColumnWide,
    ThemeSystem,
    ThemeLight,
    ThemeDark,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    FindInDocument,
    OpenSettings,
    OpenShortcuts,
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
            id: CommandId::ColumnStandard,
            title: "Reading Width: Standard",
            keys: None,
        },
        CommandSpec {
            id: CommandId::ColumnComfortable,
            title: "Reading Width: Comfortable",
            keys: None,
        },
        CommandSpec {
            id: CommandId::ColumnWide,
            title: "Reading Width: Wide",
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
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteItem {
    Command(&'static CommandSpec),
    Recent(PathBuf),
}

pub fn palette_items(query: &str, recents: &Recents) -> Vec<PaletteItem> {
    let mut items = Vec::new();
    if query.is_empty() {
        items.extend(
            recents
                .iter()
                .map(|path| PaletteItem::Recent(path.to_owned())),
        );
        items.extend(command_catalog().iter().map(PaletteItem::Command));
        return items;
    }
    let mut scored = Vec::new();
    for path in recents.iter() {
        if let Some(score) = subsequence_score(query, &path_label(path)) {
            scored.push((score, PaletteItem::Recent(path.to_owned())));
        }
    }
    for spec in command_catalog() {
        if let Some(score) = subsequence_score(query, spec.title) {
            scored.push((score, PaletteItem::Command(spec)));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, item)| item).collect()
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_owned()
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
    _query_events: Subscription,
}

impl EventEmitter<PaletteEvent> for PaletteOverlay {}

impl PaletteOverlay {
    pub fn new(recents: Recents, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = palette_items("", &recents);
        let query = cx.new(|cx| Field::new("Search commands and recent files…", window, cx));
        let query_events = cx.subscribe(&query, {
            let recents = recents.clone();
            move |this, field, event, cx| match event {
                FieldEvent::Edited => {
                    let text = field.read(cx).text().to_owned();
                    this.items = palette_items(&text, &recents);
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
            _query_events: query_events,
        }
    }

    fn invoke(&mut self, cx: &mut Context<Self>) {
        let Some(item) = self.items.get(self.selected) else {
            return;
        };
        let action = match item {
            PaletteItem::Command(spec) => PaletteAction::Run(spec.id),
            PaletteItem::Recent(path) => PaletteAction::Open(path.clone()),
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
        let theme = Theme::for_appearance(window.appearance());
        let selected = self.selected;
        let mut list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .max_h(px(320.0))
            .overflow_y_scroll();
        for (index, item) in self.items.iter().enumerate() {
            let label = match item {
                PaletteItem::Command(spec) => spec.title.to_owned(),
                PaletteItem::Recent(path) => path_label(path),
            };
            let hint = match item {
                PaletteItem::Command(spec) => spec.keys.unwrap_or("").to_owned(),
                PaletteItem::Recent(_) => "Recent".to_owned(),
            };
            list = list.child(
                div()
                    .id(("palette-item", index))
                    .debug_selector(move || format!("palette-item-{index}"))
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(28.0))
                    .px(px(10.0))
                    .rounded(px(6.0))
                    .bg(if index == selected {
                        theme.sidebar_accent
                    } else {
                        theme.sidebar_accent.opacity(0.0)
                    })
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(12.0))
                    .text_color(theme.foreground)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = index;
                        this.invoke(cx);
                    }))
                    .child(label)
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .text_size(px(11.0))
                            .child(hint),
                    ),
            );
        }
        div()
            .key_context("Palette")
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_prev))
            .w(px(480.0))
            .rounded(px(Metrics::RADIUS))
            .border_1()
            .border_color(theme.border)
            .bg(theme.card)
            .shadow_lg()
            .p(px(10.0))
            .child(
                div()
                    .mb(px(8.0))
                    .font_family(Metrics::FONT_SANS)
                    .text_size(px(13.0))
                    .child(self.query.clone()),
            )
            .child(list)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsEvent {
    Edited(PrefEdit),
    Dismissed,
}

pub struct SettingsPanel {
    prefs: Prefs,
    focus_handle: FocusHandle,
}

impl EventEmitter<SettingsEvent> for SettingsPanel {}

impl SettingsPanel {
    pub fn new(prefs: Prefs, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        Self {
            prefs,
            focus_handle,
        }
    }

    pub fn refresh(&mut self, prefs: Prefs, cx: &mut Context<Self>) {
        self.prefs = prefs;
        cx.notify();
    }
}

impl Focusable for SettingsPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::resolve(self.prefs.theme_mode, window.appearance());
        let prefs = self.prefs;
        div()
            .track_focus(&self.focus_handle)
            .w(px(420.0))
            .rounded(px(Metrics::RADIUS))
            .border_1()
            .border_color(theme.border)
            .bg(theme.card)
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .font_family(Metrics::FONT_SANS)
            .text_size(px(12.0))
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
                        12.0,
                        theme,
                        cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Dismissed)),
                    )),
            )
            .child(seg_row(
                "Theme",
                [
                    (
                        "System",
                        prefs.theme_mode == ThemeMode::System,
                        PrefEdit::Theme(ThemeMode::System),
                    ),
                    (
                        "Light",
                        prefs.theme_mode == ThemeMode::Light,
                        PrefEdit::Theme(ThemeMode::Light),
                    ),
                    (
                        "Dark",
                        prefs.theme_mode == ThemeMode::Dark,
                        PrefEdit::Theme(ThemeMode::Dark),
                    ),
                ],
                theme,
                cx,
            ))
            .child(seg_row(
                "Content font",
                [
                    (
                        "Inter",
                        prefs.content_font == ContentFont::Inter,
                        PrefEdit::ContentFont(ContentFont::Inter),
                    ),
                    (
                        "Charter",
                        prefs.content_font == ContentFont::Charter,
                        PrefEdit::ContentFont(ContentFont::Charter),
                    ),
                    (
                        "System",
                        prefs.content_font == ContentFont::SystemSans,
                        PrefEdit::ContentFont(ContentFont::SystemSans),
                    ),
                    (
                        "Georgia",
                        prefs.content_font == ContentFont::Georgia,
                        PrefEdit::ContentFont(ContentFont::Georgia),
                    ),
                ],
                theme,
                cx,
            ))
            .child(seg_row(
                "Code font",
                [
                    (
                        "Geist",
                        prefs.code_font == CodeFont::GeistMono,
                        PrefEdit::CodeFont(CodeFont::GeistMono),
                    ),
                    (
                        "System",
                        prefs.code_font == CodeFont::SystemMono,
                        PrefEdit::CodeFont(CodeFont::SystemMono),
                    ),
                    (
                        "SF Mono",
                        prefs.code_font == CodeFont::SfMono,
                        PrefEdit::CodeFont(CodeFont::SfMono),
                    ),
                    (
                        "JetBrains",
                        prefs.code_font == CodeFont::JetBrainsMono,
                        PrefEdit::CodeFont(CodeFont::JetBrainsMono),
                    ),
                ],
                theme,
                cx,
            ))
            .child(seg_row(
                "Interface scale",
                [
                    (
                        "Compact",
                        prefs.interface_scale == InterfaceScale::Compact,
                        PrefEdit::InterfaceScale(InterfaceScale::Compact),
                    ),
                    (
                        "Comfortable",
                        prefs.interface_scale == InterfaceScale::Comfortable,
                        PrefEdit::InterfaceScale(InterfaceScale::Comfortable),
                    ),
                    (
                        "Large",
                        prefs.interface_scale == InterfaceScale::Large,
                        PrefEdit::InterfaceScale(InterfaceScale::Large),
                    ),
                ],
                theme,
                cx,
            ))
            .child(seg_row(
                "Reading width",
                [
                    (
                        "Standard",
                        prefs.reader_width.column() == ColumnWidth::Standard,
                        PrefEdit::Column(ColumnWidth::Standard),
                    ),
                    (
                        "Comfortable",
                        prefs.reader_width.column() == ColumnWidth::Comfortable,
                        PrefEdit::Column(ColumnWidth::Comfortable),
                    ),
                    (
                        "Wide",
                        prefs.reader_width.column() == ColumnWidth::Wide,
                        PrefEdit::Column(ColumnWidth::Wide),
                    ),
                ],
                theme,
                cx,
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child("Zoom")
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(chip("-", false, PrefEdit::ZoomOut, theme, cx))
                            .child(
                                div()
                                    .min_w(px(40.0))
                                    .text_center()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{}%", prefs.zoom.percent())),
                            )
                            .child(chip("+", false, PrefEdit::ZoomIn, theme, cx))
                            .child(chip("Reset", false, PrefEdit::ZoomReset, theme, cx)),
                    ),
            )
            .child(chip(
                "Reset all settings",
                false,
                PrefEdit::ResetAll,
                theme,
                cx,
            ))
    }
}

fn settings_heading(title: &'static str, theme: Theme) -> impl IntoElement {
    div()
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(14.0))
        .text_color(theme.foreground)
        .child(title)
}

fn seg_row<const N: usize>(
    label: &'static str,
    options: [(&'static str, bool, PrefEdit); N],
    theme: Theme,
    cx: &mut Context<SettingsPanel>,
) -> impl IntoElement {
    let mut row = div().flex().gap(px(4.0));
    for (title, selected, edit) in options {
        row = row.child(chip(title, selected, edit, theme, cx));
    }
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(div().text_color(theme.muted_foreground).child(label))
        .child(row)
}

fn chip(
    label: &'static str,
    selected: bool,
    edit: PrefEdit,
    theme: Theme,
    cx: &mut Context<SettingsPanel>,
) -> impl IntoElement {
    let chip_id = format!("{label}-{edit:?}");
    div()
        .id(SharedString::from(chip_id.clone()))
        .debug_selector(move || chip_id)
        .px(px(8.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .rounded(px(6.0))
        .border_1()
        .border_color(if selected {
            theme.primary
        } else {
            theme.border
        })
        .bg(if selected {
            theme.primary.opacity(0.12)
        } else {
            theme.card
        })
        .cursor_pointer()
        .on_click(cx.listener(move |_, _, _, cx| cx.emit(SettingsEvent::Edited(edit))))
        .child(label)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShortcutsEvent {
    Dismissed,
}

pub struct ShortcutsCard {
    focus_handle: FocusHandle,
}

impl EventEmitter<ShortcutsEvent> for ShortcutsCard {}

impl ShortcutsCard {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        Self { focus_handle }
    }
}

impl Focusable for ShortcutsCard {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ShortcutsCard {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::for_appearance(window.appearance());
        let mut list = div().flex().flex_col().gap(px(6.0));
        for spec in command_catalog() {
            if let Some(keys) = spec.keys {
                list = list.child(
                    div()
                        .flex()
                        .justify_between()
                        .h(px(24.0))
                        .font_family(Metrics::FONT_SANS)
                        .text_size(px(12.0))
                        .text_color(theme.foreground)
                        .child(spec.title)
                        .child(div().text_color(theme.muted_foreground).child(keys)),
                );
            }
        }
        div()
            .track_focus(&self.focus_handle)
            .w(px(420.0))
            .rounded(px(Metrics::RADIUS))
            .border_1()
            .border_color(theme.border)
            .bg(theme.card)
            .p(px(16.0))
            .child(settings_heading("Keyboard shortcuts", theme))
            .child(div().mt(px(10.0)).child(list))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::InlineSpan;

    #[test]
    fn find_uses_painted_spaces_for_soft_breaks() {
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
    fn palette_empty_query_lists_recents_then_commands() {
        let recents = Recents::from_paths(vec![PathBuf::from("/notes/a.md")]);
        let items = palette_items("", &recents);
        assert!(matches!(items.first(), Some(PaletteItem::Recent(_))));
        assert!(items.iter().any(|item| matches!(
            item,
            PaletteItem::Command(spec) if spec.id == CommandId::OpenFile
        )));
    }

    #[test]
    fn palette_filters_by_subsequence() {
        let items = palette_items("thm drk", &Recents::default());
        assert!(items.iter().any(|item| matches!(
            item,
            PaletteItem::Command(spec) if spec.id == CommandId::ThemeDark
        )));
    }
}
