//! Candidate c1 — exclusive Overlay state machine, typed Prefs/Session, local Field
//! entity. Sketch only: every body is `unimplemented!()`, tricky logic is `// TODO`
//! pseudocode. Target module map (all under `apps/gpui/src/`):
//!
//!   prefs.rs      — pure preference domain types (this file: `mod prefs`)
//!   session.rs    — session snapshot types (`mod session`)
//!   persist.rs    — the only disk/string boundary (`mod persist`)
//!   ui/field.rs   — one-line input entity (`mod field`)
//!   overlay.rs    — overlay slot + the four overlay entities (`mod overlay`)
//!   theme.rs      — Theme::resolve + ShellLayout signature change (`mod theme_changes`)
//!   app.rs        — MdowApp diff (`mod app_changes`)
//!   actions.rs    — new actions (`mod actions_changes`)
//!   document.rs / ui/reader.rs — GFM/HTML/mermaid + highlight plumbing (`mod document_changes`)
//!
//! Existing types referenced but unchanged: AppModel, TabSet, PreparedDocument,
//! ParsedDocument, WorkspaceTree, Theme's palette constructors, FileWatcher.

#![allow(dead_code, unused_variables)]

use gpui::{
    App, AnyElement, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, Pixels, Point, Render, SharedString, Subscription, UTF16Selection, Window,
};
use std::{ops::Range, path::{Path, PathBuf}, sync::Arc};

// Stand-ins for existing crate types so this sketch reads standalone.
use crate::app::AppModel;
use crate::document::{DocumentBlock, Heading};
use crate::syntax::PreparedDocument;
use crate::theme::{ColorScheme, Theme};

// ---------------------------------------------------------------------------------
// prefs.rs — pure domain types. No IO, no strings, no gpui except Pixels.
// ---------------------------------------------------------------------------------
pub mod prefs {
    use super::*;

    /// Electron's `theme: string` becomes a closed set. Unknown wire values parse to
    /// `System` at the boundary (persist.rs); past that point a stringly theme cannot
    /// exist. Per encode-lessons-in-structure.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ThemeMode {
        #[default]
        System,
        Light,
        Dark,
    }

    impl ThemeMode {
        /// Pure resolution: preference + OS appearance -> concrete scheme.
        /// `Theme::resolve` (theme.rs) is the only caller.
        pub fn scheme(self, appearance: gpui::WindowAppearance) -> ColorScheme {
            unimplemented!() // System -> follow appearance; Light/Dark -> fixed
        }
    }

    /// The three column widths Electron's settings offer.
    /// standard 48rem = 768 px, comfortable 56rem = 896 px, wide 68rem = 1088 px
    /// (matches READING_WIDTHS in apps/desktop MarkdownView.tsx at 16px root).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ColumnWidth {
        #[default]
        Standard,
        Comfortable,
        Wide,
    }

    impl ColumnWidth {
        pub fn max_width(self) -> Pixels {
            unimplemented!()
        }
    }

    /// The single source of truth for reader width. Electron stores `wideMode: bool`
    /// AND `readingWidth: string` and lets them drift; here the pair is one value.
    /// `Full` remembers the column it toggled away from, so cmd-shift-W is an
    /// involution: Comfortable -> Full{Comfortable} -> Comfortable. The "unsynced
    /// wideMode/readingWidth" state is unrepresentable by construction.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ReaderWidth {
        Column(ColumnWidth),
        Full { returns_to: ColumnWidth },
    }

    impl Default for ReaderWidth {
        fn default() -> Self {
            ReaderWidth::Column(ColumnWidth::Standard)
        }
    }

    impl ReaderWidth {
        /// cmd-shift-W. Involution; toggling twice is the identity.
        pub fn toggled_full(self) -> Self {
            unimplemented!()
        }

        /// The settings 3-way select. While `Full`, edits `returns_to` (same visible
        /// behavior as Electron, where changing readingWidth under wideMode has no
        /// visible effect until wide is off — but here it cannot be *lost*).
        pub fn with_column(self, column: ColumnWidth) -> Self {
            unimplemented!()
        }

        pub fn column(self) -> ColumnWidth {
            unimplemented!()
        }

        /// `None` means full-bleed (ShellLayout falls back to READER_INSET margins,
        /// the existing wide_mode branch).
        pub fn max_width(self) -> Option<Pixels> {
            unimplemented!()
        }

        pub fn is_full(self) -> bool {
            unimplemented!()
        }
    }

    /// Chrome density. Electron maps these to CSS custom properties
    /// (data-ui-scale in index.css); GPUI maps them to concrete tokens.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum InterfaceScale {
        #[default]
        Compact,
        Comfortable,
        Large,
    }

    /// Resolved chrome tokens; replaces reaching into Metrics for the scaled values.
    /// compact: control 12 / button 28 (today's Metrics); comfortable: 13 / 32;
    /// large: 14 / 36 — from apps/desktop index.css data-ui-scale rules.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ScaleTokens {
        pub control_font: f32,
        pub control_xs_font: f32,
        pub button_height: f32,
        pub button_xs_height: f32,
    }

    impl InterfaceScale {
        pub fn tokens(self) -> ScaleTokens {
            unimplemented!()
        }
    }

    /// Percent zoom, invariant 60..=200 snapped to steps of 10 (Electron's clamp in
    /// settings-slice.ts). The constructor is the only entry, so an out-of-range or
    /// non-multiple value cannot exist; parse garbage becomes the nearest legal zoom.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ZoomLevel(u16);

    impl Default for ZoomLevel {
        fn default() -> Self {
            unimplemented!() // 100
        }
    }

    impl ZoomLevel {
        pub const MIN: u16 = 60;
        pub const MAX: u16 = 200;
        pub const STEP: u16 = 10;

        /// Clamp + snap. Total: every f64 maps to a legal zoom (parse boundary).
        pub fn from_percent(raw: f64) -> Self {
            unimplemented!()
        }

        pub fn percent(self) -> u16 {
            unimplemented!()
        }

        /// Multiplier applied to the reader's 15.5px/1.65 base type only. Chrome is
        /// governed by InterfaceScale, matching Electron's split.
        pub fn factor(self) -> f32 {
            unimplemented!()
        }

        /// Saturating at MAX / MIN respectively — idempotent at the rails.
        pub fn zoomed_in(self) -> Self {
            unimplemented!()
        }

        pub fn zoomed_out(self) -> Self {
            unimplemented!()
        }
    }

    /// Content fonts Electron offers (typography.ts). GPUI bundles Inter; the rest
    /// resolve to installed system families with the same fallbacks.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ContentFont {
        #[default]
        Inter,
        Charter,
        SystemSans,
        Georgia,
    }

    impl ContentFont {
        pub fn family(self) -> &'static str {
            unimplemented!()
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum CodeFont {
        #[default]
        GeistMono,
        SystemMono,
        SfMono,
        JetBrainsMono,
    }

    impl CodeFont {
        pub fn family(self) -> &'static str {
            unimplemented!()
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum SidebarMode {
        #[default]
        Recents,
        Folder,
        Outline,
    }

    /// The whole preference surface. Every field is a closed type with a Default, so
    /// `Prefs::default()` is a complete, legal state and partial parses stay legal.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct Prefs {
        pub theme_mode: ThemeMode,
        pub content_font: ContentFont,
        pub code_font: CodeFont,
        pub interface_scale: InterfaceScale,
        pub reader_width: ReaderWidth,
        pub zoom: ZoomLevel,
        pub sidebar_mode: SidebarMode,
    }

    /// Every way a preference can change, from any surface (settings panel, palette,
    /// menu, keybinding). One vocabulary means one funnel in MdowApp and one
    /// persistence point in StoredPrefs.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PrefEdit {
        Theme(ThemeMode),
        ContentFont(ContentFont),
        CodeFont(CodeFont),
        InterfaceScale(InterfaceScale),
        Column(ColumnWidth),
        /// cmd-shift-W / palette "Toggle wide mode".
        ToggleFull,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        Sidebar(SidebarMode),
        /// Settings panel "Reset to defaults" (keeps session untouched).
        ResetAll,
    }

    impl Prefs {
        /// Returns true when the edit changed anything, so callers can skip
        /// persistence and notify. Applying the same edit twice is a no-op the
        /// second time (idempotent), except the deliberate involutions
        /// (ToggleFull) which are documented as such.
        pub fn apply(&mut self, edit: PrefEdit) -> bool {
            unimplemented!()
        }

        /// Everything render_document needs, in one struct, so the reader signature
        /// doesn't grow a parameter per preference.
        pub fn reader_style(&self) -> ReaderStyle {
            unimplemented!()
        }
    }

    /// Derived view of Prefs for the reader. Content-only: chrome reads ScaleTokens.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ReaderStyle {
        pub content_family: &'static str,
        pub code_family: &'static str,
        /// 15.5 * zoom.factor()
        pub font_size: f32,
        /// Fixed 1.65 — zoom scales size, not leading ratio (matches Electron).
        pub line_height: f32,
        pub max_width: Option<Pixels>,
    }
}

// ---------------------------------------------------------------------------------
// session.rs — what was open. Captured from live state, never synced field-by-field.
// ---------------------------------------------------------------------------------
pub mod session {
    use super::prefs::*;
    use super::*;

    /// Saved window placement (Electron windowBounds parity).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct SavedWindowBounds {
        pub x: f32,
        pub y: f32,
        pub width: f32,
        pub height: f32,
    }

    /// Open tabs with the active one structurally required to be a member — a
    /// zipper. Electron's `sessionTabs` + `sessionActiveTabPath` pair can name an
    /// active path that isn't in the tab list; this cannot. `None` on the Session
    /// field covers the empty case.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SessionTabs {
        pub before: Vec<PathBuf>,
        pub active: PathBuf,
        pub after: Vec<PathBuf>,
    }

    impl SessionTabs {
        pub fn iter(&self) -> impl Iterator<Item = &Path> {
            // before, active, after — restore order == visual order
            unimplemented!();
            #[allow(unreachable_code)]
            std::iter::empty()
        }
    }

    /// Most-recent-first, deduped by canonical path, capped. Cheap to clone (palette
    /// takes a snapshot at open).
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub struct Recents(Vec<PathBuf>);

    impl Recents {
        /// Electron's MAX_RECENTS in main/store.ts.
        pub const CAP: usize = 20;

        /// Move-to-front + dedupe + cap. Returns true if the list changed. Calling
        /// twice with the same path is a no-op the second time.
        pub fn note(&mut self, path: &Path) -> bool {
            unimplemented!()
        }

        pub fn iter(&self) -> impl Iterator<Item = &Path> {
            unimplemented!();
            #[allow(unreachable_code)]
            std::iter::empty()
        }

        pub fn is_empty(&self) -> bool {
            unimplemented!()
        }

        /// From wire paths at the parse boundary; drops duplicates, enforces CAP.
        /// Missing files are kept (they may be on an unmounted volume) and pruned
        /// lazily on render, matching Electron's pruneRecentsList.
        pub fn from_paths(paths: Vec<PathBuf>) -> Self {
            unimplemented!()
        }
    }

    /// Snapshot of restorable state. Produced two ways only: `StateStore::load`
    /// (from disk) and `Session::capture` (from live state). There is no
    /// field-by-field mutation API, so it cannot drift from the model — derive,
    /// don't sync.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct Session {
        pub tabs: Option<SessionTabs>,
        pub last_folder: Option<PathBuf>,
        pub recents: Recents,
        pub window: Option<SavedWindowBounds>,
    }

    impl Session {
        /// Read the live model. `window` is the shell's last observed bounds
        /// (MdowApp records it during render, where &Window is available).
        pub fn capture(model: &AppModel, window: Option<SavedWindowBounds>) -> Self {
            // TODO: tabs from model.tabs.paths() split around model.tabs.active();
            //       last_folder from model.workspace root; recents from model.recents.
            unimplemented!()
        }
    }
}

// ---------------------------------------------------------------------------------
// persist.rs — the only module that touches disk or wire strings.
// ---------------------------------------------------------------------------------
pub mod persist {
    use super::prefs::*;
    use super::session::*;
    use super::*;

    /// What launch gets back. Never an error: a viewer must open even if its state
    /// file is corrupt. Per boundary-discipline: parse here, trust types after.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct Restored {
        pub prefs: Prefs,
        pub session: Session,
    }

    /// Owns the state file path. All serde types are private to this module — wire
    /// keys (camelCase, Electron-compatible: theme, contentFont, codeFont,
    /// interfaceScale, readingWidth, wideMode, zoomLevel, sidebarMode, recents,
    /// lastFolder, sessionTabs, sessionActiveTabPath, windowBounds) never appear on
    /// the public surface.
    pub struct StateStore {
        path: PathBuf,
    }

    impl StateStore {
        /// ~/Library/Application Support/Mdow Native/state.json
        pub fn open_default() -> Self {
            unimplemented!()
        }

        /// For tests.
        pub fn open_at(path: PathBuf) -> Self {
            unimplemented!()
        }

        /// Total function. Field-tolerant: each unknown/missing/ill-typed field
        /// falls back to its Default independently; one bad field never discards
        /// the rest. Migration happens here and only here:
        ///   (wideMode: true,  readingWidth: w) -> ReaderWidth::Full { returns_to: w }
        ///   (wideMode: false, readingWidth: w) -> ReaderWidth::Column(w)
        ///   sessionActiveTabPath not in sessionTabs -> active falls back to last tab
        pub fn load(&self) -> Restored {
            unimplemented!()
        }

        /// Whole-file atomic write: serialize to temp file in the same directory,
        /// fsync, rename over. A crash mid-save leaves the previous state intact
        /// (make-operations-idempotent: replaying a save is harmless).
        /// Errors are logged, not surfaced — losing one save must not break reading.
        fn save(&self, prefs: &Prefs, session: &Session) {
            unimplemented!()
        }
    }

    /// Write-through preference cell: the only owner of both the live `Prefs` and
    /// the `StateStore`, so a preference mutation that skips persistence is
    /// unrepresentable. MdowApp holds exactly one of these.
    pub struct StoredPrefs {
        prefs: Prefs,
        store: StateStore,
    }

    impl StoredPrefs {
        pub fn restore(prefs: Prefs, store: StateStore) -> Self {
            unimplemented!()
        }

        pub fn get(&self) -> &Prefs {
            unimplemented!()
        }

        /// Applies the edit; if anything changed, saves prefs *and* the provided
        /// session snapshot in one atomic write. Returns whether a change happened.
        pub fn apply(&mut self, edit: PrefEdit, session: &Session) -> bool {
            unimplemented!()
        }

        /// Session-only mutations (tab open/close/switch, folder open, recents)
        /// reuse the same atomic write path. Called from MdowApp's
        /// active_document_changed funnel and on quit.
        pub fn save_session(&self, session: &Session) {
            unimplemented!()
        }
    }

    // -- private wire types ---------------------------------------------------------
    // #[derive(serde::Serialize, serde::Deserialize, Default)]
    // struct WireState {
    //     theme: Option<String>,
    //     contentFont: Option<String>,
    //     codeFont: Option<String>,
    //     interfaceScale: Option<String>,
    //     readingWidth: Option<String>,
    //     wideMode: Option<bool>,
    //     zoomLevel: Option<f64>,
    //     sidebarMode: Option<String>,
    //     recents: Option<Vec<String>>,
    //     lastFolder: Option<String>,
    //     sessionTabs: Option<Vec<WireTab>>,      // { path: String } — Electron shape
    //     sessionActiveTabPath: Option<String>,
    //     windowBounds: Option<WireBounds>,
    // }
    // fn decode(wire: WireState) -> Restored     — pure, total
    // fn encode(prefs: &Prefs, session: &Session) -> WireState — pure
    // decode(encode(x)) == x  — round-trip property test lives here.
}

// ---------------------------------------------------------------------------------
// ui/field.rs — one-line text input entity, from gpui 0.2.2 examples/input.rs.
// Deep module: callers get new/text/set_text/events; UTF-16 offsets, IME marked
// text, selection painting, and mouse selection are all hidden behind it.
// ---------------------------------------------------------------------------------
pub mod field {
    use super::*;

    /// Field-scoped actions, bound under key context "Field" so they never shadow
    /// reader/global bindings.
    /// gpui::actions!(field, [Backspace, Delete, MoveLeft, MoveRight, SelectLeft,
    ///     SelectRight, SelectAll, Home, End, Paste, Copy, Cut, Submit,
    ///     SubmitBackward, Cancel]);
    pub struct FieldActionsPlaceholder;

    /// What owners subscribe to. Everything else about text editing is private.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum FieldEvent {
        /// Content changed (typing, paste, cut, IME commit).
        Edited,
        /// Enter. `backward` is shift-enter (find uses it for "previous match").
        Submitted { backward: bool },
        /// Escape pressed while the field was focused.
        Cancelled,
    }

    pub struct Field {
        content: SharedString,
        placeholder: SharedString,
        /// Byte offsets into `content`; UTF-16 conversion happens only inside the
        /// EntityInputHandler impl.
        selected_range: Range<usize>,
        selection_reversed: bool,
        marked_range: Option<Range<usize>>,
        /// Last laid-out line + bounds, needed by bounds_for_range /
        /// character_index_for_point. Written by FieldElement during paint.
        last_layout: Option<gpui::ShapedLine>,
        last_bounds: Option<Bounds<Pixels>>,
        is_selecting: bool,
        focus_handle: FocusHandle,
    }

    impl EventEmitter<FieldEvent> for Field {}

    impl Field {
        pub fn new(placeholder: impl Into<SharedString>, window: &mut Window, cx: &mut Context<Self>) -> Self {
            unimplemented!()
        }

        pub fn text(&self) -> &str {
            unimplemented!()
        }

        /// Replaces content, selects all (so typing overwrites), emits Edited if
        /// changed. Idempotent for equal text.
        pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
            unimplemented!()
        }

        pub fn focus(&self, window: &mut Window) {
            unimplemented!()
        }

        // Private: on_backspace/on_delete/movement/selection/clipboard action
        // handlers, offset_from_utf16/offset_to_utf16, mouse down/drag selection.
        // All lifted from examples/input.rs and kept single-line (newlines in
        // pasted text are stripped).
    }

    impl Focusable for Field {
        fn focus_handle(&self, cx: &App) -> FocusHandle {
            unimplemented!()
        }
    }

    impl gpui::EntityInputHandler for Field {
        fn text_for_range(&mut self, range_utf16: Range<usize>, adjusted_range: &mut Option<Range<usize>>, window: &mut Window, cx: &mut Context<Self>) -> Option<String> {
            unimplemented!()
        }
        fn selected_text_range(&mut self, ignore_disabled_input: bool, window: &mut Window, cx: &mut Context<Self>) -> Option<UTF16Selection> {
            unimplemented!()
        }
        fn marked_text_range(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<Range<usize>> {
            unimplemented!()
        }
        fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
            unimplemented!()
        }
        fn replace_text_in_range(&mut self, range_utf16: Option<Range<usize>>, text: &str, window: &mut Window, cx: &mut Context<Self>) {
            unimplemented!()
        }
        fn replace_and_mark_text_in_range(&mut self, range_utf16: Option<Range<usize>>, new_text: &str, new_selected_range: Option<Range<usize>>, window: &mut Window, cx: &mut Context<Self>) {
            unimplemented!()
        }
        fn bounds_for_range(&mut self, range_utf16: Range<usize>, element_bounds: Bounds<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> Option<Bounds<Pixels>> {
            unimplemented!()
        }
        fn character_index_for_point(&mut self, point: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> Option<usize> {
            unimplemented!()
        }
    }

    impl Render for Field {
        /// Paints via a private custom Element (cursor, selection rect, marked-text
        /// underline, placeholder in muted_foreground). 28px tall, Emil rules: no
        /// layout shift on focus, focus ring via theme.primary ring not border swap.
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            unimplemented!();
            #[allow(unreachable_code)]
            gpui::div()
        }
    }

    // struct FieldElement { field: Entity<Field> }  — private custom Element
    // impl Element for FieldElement { request_layout / prepaint / paint }
}

// ---------------------------------------------------------------------------------
// overlay.rs — the exclusive slot and the four overlay entities.
// ---------------------------------------------------------------------------------
pub mod overlay {
    use super::field::{Field, FieldEvent};
    use super::prefs::*;
    use super::session::Recents;
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OverlayKind {
        Find,
        Palette,
        Settings,
        Shortcuts,
    }

    /// One open overlay: the view plus the subscription that routes its events to
    /// MdowApp. The subscription lives and dies with the slot, so events from a
    /// closed overlay are unrepresentable (no stale-callback bugs).
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
            unimplemented!()
        }
        pub fn palette(view: Entity<PaletteOverlay>, events: Subscription) -> Self {
            unimplemented!()
        }
        pub fn settings(view: Entity<SettingsPanel>, events: Subscription) -> Self {
            unimplemented!()
        }
        pub fn shortcuts(view: Entity<ShortcutsCard>, events: Subscription) -> Self {
            unimplemented!()
        }
    }

    /// The state machine. `open: Option<OpenOverlay>` IS the exclusivity invariant:
    /// Electron's four independent booleans (16 states, 11 illegal) collapse to
    /// five representable states, all legal. MdowApp owns exactly one host.
    #[derive(Default)]
    pub struct OverlayHost {
        open: Option<OpenOverlay>,
        /// Where focus returns on close (captured from window at open).
        return_focus: Option<FocusHandle>,
    }

    impl OverlayHost {
        pub fn kind(&self) -> Option<OverlayKind> {
            unimplemented!()
        }

        /// Replaces whatever is open (opening palette over find closes find),
        /// captures current focus for restore, focuses the new overlay's field.
        pub fn open(&mut self, overlay: OpenOverlay, window: &mut Window) {
            unimplemented!()
        }

        /// Drops the overlay (its Field entity, subscriptions, and any transient
        /// query/match state die with it) and restores focus. Returns false when
        /// nothing was open — callers use that to let Escape fall through to
        /// e.g. dismissing the reload-error banner. Idempotent.
        pub fn close(&mut self, window: &mut Window) -> bool {
            unimplemented!()
        }

        /// Render hook for find-highlight derivation in MdowApp::render.
        pub fn find(&self) -> Option<&Entity<FindOverlay>> {
            unimplemented!()
        }

        /// Funnel from MdowApp::active_document_changed — retargets an open find
        /// overlay at the new document. No-op for other overlays.
        pub fn retarget_find(&self, document: Option<Arc<PreparedDocument>>, cx: &mut App) {
            unimplemented!()
        }

        /// Push fresh prefs into an open settings panel after a PrefEdit applies
        /// (panel displays state; it never owns it).
        pub fn refresh_settings(&self, prefs: &Prefs, cx: &mut App) {
            unimplemented!()
        }

        /// The overlay layer for the root render: scrim + centered card for
        /// palette/settings/shortcuts, top-right anchored bar for find. None when
        /// closed. Scrim click emits the overlay's Dismissed.
        pub fn render_layer(&self, theme: Theme, cx: &mut App) -> Option<AnyElement> {
            unimplemented!()
        }
    }

    // -- Find --------------------------------------------------------------------

    /// One match: block index into ParsedDocument::blocks + byte range within that
    /// block's flattened plain text.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FindHit {
        pub block: usize,
        pub range_start: usize,
        pub range_end: usize,
    }

    /// Matches plus cursor. Invariant (private field, constructor-enforced):
    /// `cursor < hits.len()` whenever hits is non-empty, None otherwise.
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub struct FindMatches {
        hits: Vec<FindHit>,
        cursor: Option<usize>,
    }

    impl FindMatches {
        pub fn hits(&self) -> &[FindHit] {
            unimplemented!()
        }
        pub fn active(&self) -> Option<FindHit> {
            unimplemented!()
        }
        /// "3 of 14" for the bar. None when the query is empty.
        pub fn position(&self) -> Option<(usize, usize)> {
            unimplemented!()
        }
    }

    /// Pure and total: the whole find feature's logic in one testable function.
    /// Case-insensitive substring over each block's flattened inline text (the
    /// existing ParsedDocument::plain_text traversal, per block). Empty query ->
    /// empty result.
    pub fn find_in_blocks(blocks: &[DocumentBlock], query: &str) -> Vec<FindHit> {
        unimplemented!()
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum FindEvent {
        /// Cursor moved to a hit (typing recomputed, enter advanced). The app
        /// scrolls the reader; highlights are read back at render time.
        ActiveHit(FindHit),
        Dismissed,
    }

    /// Owns the query field, the target document, and the match state. Subscribes
    /// to its own Field internally — MdowApp never sees FieldEvent for find.
    pub struct FindOverlay {
        query: Entity<Field>,
        document: Option<Arc<PreparedDocument>>,
        matches: FindMatches,
        _query_events: Subscription,
    }

    impl EventEmitter<FindEvent> for FindOverlay {}

    impl FindOverlay {
        pub fn new(document: Option<Arc<PreparedDocument>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
            // TODO: create Field("Find in document"), subscribe:
            //   Edited -> recompute matches via find_in_blocks, cursor to first hit
            //             at-or-after the previous active hit (stable while typing),
            //             emit ActiveHit if any
            //   Submitted { backward } -> advance(backward)
            //   Cancelled -> emit Dismissed
            unimplemented!()
        }

        /// Tab switch / reload while open: recompute against the new document,
        /// cursor resets to first hit. Same query, new haystack.
        pub fn retarget(&mut self, document: Option<Arc<PreparedDocument>>, cx: &mut Context<Self>) {
            unimplemented!()
        }

        /// Enter / shift-enter, wraps around. No-op when there are no hits.
        pub fn advance(&mut self, backward: bool, cx: &mut Context<Self>) {
            unimplemented!()
        }

        pub fn matches(&self) -> &FindMatches {
            unimplemented!()
        }
    }

    impl Render for FindOverlay {
        /// Compact bar: field + "n of m" (tabular nums) + prev/next/close buttons,
        /// 28-36px chrome, no layout shift as counts change (fixed-width count slot).
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            unimplemented!();
            #[allow(unreachable_code)]
            gpui::div()
        }
    }

    // -- Palette -------------------------------------------------------------------

    /// Closed set of reader-loop commands. Adding a command = one variant + one
    /// CommandSpec row + one arm in MdowApp::run_command; the compiler finds the
    /// arms.
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

    pub struct CommandSpec {
        pub id: CommandId,
        pub title: &'static str,
        /// Display hint only ("⌘⇧W"); bindings stay in main.rs.
        pub keys: Option<&'static str>,
    }

    /// Static catalog, source order = display order for the empty query.
    pub fn command_catalog() -> &'static [CommandSpec] {
        unimplemented!()
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum PaletteItem {
        Command(&'static CommandSpec),
        Recent(PathBuf),
    }

    /// Pure filter: subsequence match with contiguous-run and word-start bonuses
    /// (small hand-rolled scorer; no fuzzy-match dependency). Empty query lists
    /// recents first then the full catalog, matching Electron's palette.
    pub fn palette_items(query: &str, recents: &Recents) -> Vec<PaletteItem> {
        unimplemented!()
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

    /// Owns query field + filtered items + selection. Recents are a snapshot taken
    /// at open (the palette is short-lived; staleness is bounded by its lifetime).
    pub struct PaletteOverlay {
        query: Entity<Field>,
        recents: Recents,
        items: Vec<PaletteItem>,
        /// Invariant: `selected < items.len()` when items is non-empty; re-clamped
        /// on every recompute.
        selected: usize,
        _query_events: Subscription,
    }

    impl EventEmitter<PaletteEvent> for PaletteOverlay {}

    impl PaletteOverlay {
        pub fn new(recents: Recents, window: &mut Window, cx: &mut Context<Self>) -> Self {
            // TODO: Field("Search commands and recent files…"); Edited -> re-run
            // palette_items + clamp selection; Submitted -> Invoked(selected item);
            // Cancelled -> Dismissed. Up/Down bound in the palette's key context
            // move `selected`.
            unimplemented!()
        }
    }

    impl Render for PaletteOverlay {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            unimplemented!();
            #[allow(unreachable_code)]
            gpui::div()
        }
    }

    // -- Settings ------------------------------------------------------------------

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum SettingsEvent {
        /// The panel proposes; MdowApp::apply_pref disposes (and persists). The
        /// panel never mutates Prefs directly — display state only.
        Edited(PrefEdit),
        Dismissed,
    }

    /// Rows: theme (3-seg), content font, code font, interface scale (3-seg),
    /// reading width (3-seg over ColumnWidth — Full is the toggle's business),
    /// zoom stepper, reset-all. No Field needed; all controls are discrete.
    pub struct SettingsPanel {
        /// Display copy, refreshed via OverlayHost::refresh_settings after applies.
        prefs: Prefs,
        focus_handle: FocusHandle,
    }

    impl EventEmitter<SettingsEvent> for SettingsPanel {}

    impl SettingsPanel {
        pub fn new(prefs: Prefs, window: &mut Window, cx: &mut Context<Self>) -> Self {
            unimplemented!()
        }

        pub fn refresh(&mut self, prefs: Prefs, cx: &mut Context<Self>) {
            unimplemented!()
        }
    }

    impl Render for SettingsPanel {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            unimplemented!();
            #[allow(unreachable_code)]
            gpui::div()
        }
    }

    // -- Shortcuts -----------------------------------------------------------------

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ShortcutsEvent {
        Dismissed,
    }

    /// Static two-column card generated from command_catalog() keys — single source
    /// for shortcut display, so the card can't disagree with the palette hints.
    pub struct ShortcutsCard {
        focus_handle: FocusHandle,
    }

    impl EventEmitter<ShortcutsEvent> for ShortcutsCard {}

    impl ShortcutsCard {
        pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
            unimplemented!()
        }
    }

    impl Render for ShortcutsCard {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            unimplemented!();
            #[allow(unreachable_code)]
            gpui::div()
        }
    }
}

// ---------------------------------------------------------------------------------
// theme.rs changes
// ---------------------------------------------------------------------------------
pub mod theme_changes {
    use super::prefs::{InterfaceScale, ReaderWidth, ThemeMode};
    use super::*;

    impl Theme {
        /// Replaces for_appearance as the render-time entry point. Pure; recomputed
        /// each frame in MdowApp::render so ThemeMode changes and OS appearance
        /// changes take effect with no stored intermediate (single source of truth).
        pub fn resolve(mode: ThemeMode, appearance: gpui::WindowAppearance) -> Theme {
            unimplemented!()
        }
    }

    /// Changed signature — wide_mode: bool becomes ReaderWidth, and scale arrives
    /// so tab/breadcrumb heights come from ScaleTokens instead of raw Metrics.
    /// Existing width math is otherwise unchanged: Column(w) centers min(w, main),
    /// Full keeps the READER_INSET margins today's wide_mode branch uses.
    pub struct ShellLayoutChange;
    // pub fn for_width(window_width: f32, sidebar_open: bool,
    //                  reader_width: ReaderWidth, scale: InterfaceScale) -> ShellLayout
}

// ---------------------------------------------------------------------------------
// actions.rs changes
// ---------------------------------------------------------------------------------
pub mod actions_changes {
    // actions!(mdow, [
    //     OpenFile, OpenFolder, ToggleSidebar, CloseTab, ToggleWideMode, Quit, // existing
    //     ToggleFind,        // cmd-f
    //     TogglePalette,     // cmd-k (and cmd-shift-p alias)
    //     ToggleSettings,    // cmd-,
    //     ToggleShortcuts,   // cmd-/
    //     Dismiss,           // escape: overlay first, then reload-error banner
    //     FindNext,          // cmd-g   (enter in field also advances)
    //     FindPrevious,      // cmd-shift-g
    //     ZoomIn, ZoomOut, ZoomReset, // cmd-= / cmd-- / cmd-0
    //     SidebarRecents, SidebarFolder, SidebarOutline, // ctrl-1/2/3
    // ]);
    // Field-scoped actions live in ui/field.rs under key context "Field".
    // Menus gain a View menu (sidebar modes, wide mode, zoom, theme) and an Edit
    // menu (Find). All menu items dispatch these actions — menu/keyboard parity
    // comes from sharing one action vocabulary.
}

// ---------------------------------------------------------------------------------
// app.rs changes — MdowApp stays the thin shell: routing, composition, funnels.
// ---------------------------------------------------------------------------------
pub mod app_changes {
    use super::overlay::*;
    use super::persist::*;
    use super::prefs::*;
    use super::session::*;
    use super::*;

    /// Field diff for MdowApp (full struct omitted — unchanged fields elided):
    ///
    ///   - wide_mode: bool                    REMOVED (derived from prefs.reader_width)
    ///   + prefs: StoredPrefs                 write-through preference cell
    ///   + overlays: OverlayHost              the exclusive slot
    ///   + last_window_bounds: Option<SavedWindowBounds>  recorded in render
    ///   ~ theme: Theme                       now derived via Theme::resolve each frame
    ///
    /// AppModel gains one field:
    ///   + recents: Recents                   noted in open_document; domain state,
    ///                                        so Session::capture reads one place
    pub struct MdowAppDiff;

    pub trait MdowAppSketch {
        /// Prefs arrive parsed; the store arrives owned. No IO in the constructor
        /// beyond what new() already does (watcher setup).
        fn new_(prefs: Prefs, store: StateStore, window: &mut Window, cx: &mut Context<Self>) -> Self
        where
            Self: Sized;

        /// Tolerant restore: opens session tabs in order, skipping paths that fail
        /// to load (no launch error banners for moved files — recents keep them),
        /// re-registers watchers, activates the zipper's active tab, reopens
        /// last_folder as workspace. Running it twice dedupes via TabSet.
        fn restore_session(&mut self, session: Session, cx: &mut Context<Self>);

        /// THE preference funnel (usage.md call site 3). Applies via StoredPrefs
        /// (persists on change), refreshes an open settings panel, notifies.
        fn apply_pref(&mut self, edit: PrefEdit, cx: &mut Context<Self>);

        /// THE overlay funnel (usage.md call site 2). Same-kind toggle closes;
        /// different kind replaces. Construction + event subscription happen here
        /// — the only place overlay entities are born.
        fn toggle_overlay(&mut self, kind: OverlayKind, window: &mut Window, cx: &mut Context<Self>);

        /// Escape. Overlay first; if none, dismiss reload-error banner; else no-op.
        fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>);

        /// Palette dispatch: one arm per CommandId, each delegating to an existing
        /// method or apply_pref. No logic of its own (pass-through is the point:
        /// this is routing, not behavior).
        fn run_command(&mut self, id: CommandId, window: &mut Window, cx: &mut Context<Self>);

        /// THE document-change funnel, replacing scattered
        /// clear_reader_transient_state call sites: clears transient reader state,
        /// notes recents, retargets an open find overlay, persists the session.
        /// Called from open_path / open_paths / activate_tab / close_tab / reload.
        fn active_document_changed(&mut self, cx: &mut Context<Self>);

        /// Session::capture(&self.model, self.last_window_bounds) — used by
        /// apply_pref and active_document_changed.
        fn session_snapshot(&self) -> Session;

        /// Shared by find (ActiveHit) and the outline sidebar: scroll the active
        /// reader so `block` is visible.
        /// TODO: reader paint records per-block y-origins into BlockPositions
        /// (same keyed-by-path pattern as reader_scroll_handles); this looks up the
        /// offset and sets the scroll handle, clamped, no animation under
        /// reduced-motion.
        fn scroll_reader_to_block(&mut self, block: usize, cx: &mut Context<Self>);

        /// FindEvent / PaletteEvent / SettingsEvent / ShortcutsEvent handlers —
        /// each a small match routing to the funnels above (see usage.md).
        fn on_find_event(&mut self);
        fn on_palette_event(&mut self);
        fn on_settings_event(&mut self);
        fn on_shortcuts_event(&mut self);
    }

    // Render diff (composition only):
    //   theme      = Theme::resolve(prefs.theme_mode, window.appearance())
    //   layout     = ShellLayout::for_width(w, sidebar_open, prefs.reader_width,
    //                                       prefs.interface_scale)
    //   sidebar    = match prefs.sidebar_mode { Recents | Folder | Outline } (usage.md)
    //   reader     = render_document(doc, prefs.reader_style(), theme, copied,
    //                                links, overlays.find().map(|f| f.read(cx).matches()),
    //                                scroll_handle, cx)
    //   top layer  = .children(overlays.render_layer(theme, cx))
    //   bounds     = self.last_window_bounds = observed window bounds (for capture)
}

// ---------------------------------------------------------------------------------
// document.rs / ui/reader.rs changes — parser-level parity, no new architecture.
// The DocumentBlock pipeline stays; these are additive shapes inside it.
// ---------------------------------------------------------------------------------
pub mod document_changes {
    use super::*;

    /// GFM alert kinds (> [!NOTE] etc.) Electron already styles.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AlertKind {
        Note,
        Tip,
        Important,
        Warning,
        Caution,
    }

    // DocumentBlock gains variants (rendered by the existing reader with new arms):
    //   Alert { kind: AlertKind, children: Vec<DocumentBlock> }
    //   MermaidCard { source: String }        — native card: header "Mermaid" +
    //                                           monospaced source + copy control.
    //                                           No JS runtime, per constraints.
    //   FootnoteSection { notes: Vec<(String, Vec<DocumentBlock>)> }
    // InlineSpan styling gains Strikethrough (replaces InlineContainer::Flatten for
    // pulldown-cmark's Strikethrough events) and FootnoteRef { label } rendered as
    // a superscript link to the section.
    //
    // HTML documents (.html/.htm): parsed to the SAME DocumentBlock vocabulary by a
    // restricted converter — headings/paragraphs/lists/pre/code/blockquote/tables/
    // images; scripts, styles, and unknown elements flatten to inert text. No
    // webview, matching the sandboxed-iframe intent without one:
    pub fn html_to_blocks(source: &str) -> Vec<DocumentBlock> {
        unimplemented!()
    }

    /// render_document signature change (ui/reader.rs): wide_mode: bool is replaced
    /// by ReaderStyle (fonts, zoomed size, max width) and find matches arrive for
    /// highlight painting — accent-tinted runs, active hit in accent-solid.
    pub struct RenderDocumentChange;
    // pub fn render_document(document: Arc<PreparedDocument>, style: ReaderStyle,
    //                        theme: Theme, copied_code: Option<(usize, Instant)>,
    //                        links: &ReaderLinkState, find: Option<&FindMatches>,
    //                        scroll_handle: &ScrollHandle, cx: &mut Context<MdowApp>)
    //                        -> impl IntoElement

    /// Per-path block y-origins recorded during reader paint; read by
    /// scroll_reader_to_block (find + outline). Same lifecycle as
    /// reader_scroll_handles: created on first render, dropped on tab close.
    #[derive(Default)]
    pub struct BlockPositions {
        origins: Vec<Option<f32>>, // index = block index; None until first paint
    }

    impl BlockPositions {
        pub fn record(&mut self, block: usize, y: f32) {
            unimplemented!()
        }
        pub fn origin(&self, block: usize) -> Option<f32> {
            unimplemented!()
        }
    }
}
