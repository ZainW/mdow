//! Candidate C4 — the reader engine.
//!
//! # Module map
//!
//! ```text
//! src/
//!   app.rs              MdowApp: tabs, workspace, drop, menus. Owns no reader state.
//!   actions.rs          + Find, FindNext, FindPrev, Dismiss, Copy, SelectAll, Palette,
//!                         Settings, Shortcuts, Zoom{In,Out,Reset}, CycleSidebarMode
//!   chrome/
//!     mod.rs            Chrome: one modal *or* none, plus a non-modal find layer
//!     tab_bar.rs        (unchanged shape)
//!     sidebar.rs        recents / folder / outline
//!     palette.rs        command palette over a static command table + fuzzy scorer
//!     settings.rs       preferences dialog
//!     text_field.rs     minimal one-line EntityInputHandler (GPUI 0.2.2 has no Input)
//!   settings.rs         Preferences, Session, Store (atomic, debounced, idempotent)
//!   theme.rs            Theme, Metrics, ShellLayout  (unchanged palette; + ThemePreference)
//!   reader/
//!     mod.rs            Reader entity + ReaderEvent. The only public surface.
//!     doc.rs            Source -> BlockTree -> Atoms. Incremental splice. OWNS text.
//!     markdown.rs       pulldown-cmark -> BlockTree
//!     html.rs           html5ever -> BlockTree   (same Atom pipeline, no webview)
//!     highlight.rs      syntect -> StyleRun token classes  (colours resolved at paint)
//!     select.rs         Cursor / Span algebra. Pure. OWNS selection semantics.
//!     find.rs           Query -> Hits over atoms. Pure. OWNS match ordering.
//!     frame.rs          FrameLog: painted geometry. OWNS hit-testing.
//!     style.rs          ReaderStyle + block spacing (the measured visual contract)
//!     view.rs           Atoms -> GPUI elements, wash painting, input installation
//! ```
//!
//! # What is deleted
//!
//! `document.rs` (`DocumentBlock`, `InlineSpan`, `InlineContainer::Flatten`),
//! `syntax.rs` (`PreparedDocument`, `HighlightedCode` on the public surface),
//! and all of `ui/reader.rs` except the spacing table, which moves to `reader/style.rs`
//! intact because 132 tests pin it. From `MdowApp`: `copied_code`, `hovered_link`,
//! `focused_link`, `reader_scrollbar_drag`, `reader_scroll_handles`,
//! `reader_link_focus_handles`, and the fifteen methods that maintain them.
//!
//! # The one idea
//!
//! A document compiles to a flat, document-ordered vector of **atoms**: shaped text runs
//! with stable identity. Selection, find, copy, outline, links, and reveal are all queries
//! over that one vector, in one `(atom, byte range)` coordinate space. Nothing is
//! synchronised; everything is derived.

#![allow(unused, clippy::needless_pass_by_value)]

use gpui::{
    AnyElement, App, Bounds, ClipboardItem, Context, Entity, EventEmitter, FocusHandle, Focusable,
    Hsla, IntoElement, Pixels, Point, Render, ScrollHandle, SharedString, TextLayout, TextRun,
    Window,
};
use std::{
    cell::RefCell,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

// ═══════════════════════════════════════════════════════════════════════════
// reader/doc.rs — the compiled document
// ═══════════════════════════════════════════════════════════════════════════

pub mod doc {
    use super::*;

    /// Stable identity for one compiled block.
    ///
    /// Survives an incremental reparse for every block whose source bytes are unchanged.
    /// This is what lets scroll offset, selection anchors, and find matches outlive a
    /// file save. Never an ordinal: inserting a block above must not renumber the ones
    /// below.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct BlockId(u64);

    /// Stable identity for one shaped text atom, scoped to its block.
    ///
    /// `slot` is the atom's position *within* its block (0 for a paragraph, the cell
    /// ordinal for a table). Reparsing a block reassigns slots consistently, so a table
    /// cell keeps its identity when a neighbouring cell's text changes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct AtomId {
        pub block: BlockId,
        pub slot: u16,
    }

    /// Position in *this* compilation's document order.
    ///
    /// Deliberately a different type from [`AtomId`]. Ordering comparisons and range
    /// arithmetic are only valid on indices; persistence across a reparse is only valid
    /// on ids. Mixing them is the bug this pair exists to make unrepresentable.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct AtomIndex(pub u32);

    /// Position in the depth-first block vector.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct BlockIndex(pub u32);

    /// One shaped text element: a flat string plus the semantic runs that tile it.
    ///
    /// # Invariants
    ///
    /// - `runs` tile `text` exactly. GPUI's `StyledText::with_runs` panics otherwise, so
    ///   this is enforced by construction: an `Atom` can only be built through
    ///   [`AtomBuilder`], which appends text and style together.
    /// - Runs carry *semantics*, never colour. Colour is resolved against the palette at
    ///   paint time, so a theme switch cannot invalidate a single byte of compiled work.
    /// - `text` is what the user sees. Find and copy read the same string the glyphs came
    ///   from, which is how the native reader matches Electron's DOM-walk semantics
    ///   without a second traversal.
    #[derive(Debug, Clone)]
    pub struct Atom {
        pub id: AtomId,
        pub text: SharedString,
        pub runs: Vec<StyleRun>,
        pub links: Vec<LinkRange>,
        /// Inline-code spans, painted as rounded washes under the glyphs.
        pub code_ranges: Vec<Range<usize>>,
        pub role: AtomRole,
        /// First atom of a top-level block: copy inserts a blank line before it.
        pub starts_block: bool,
    }

    /// Semantic inline style. No colours, no font handles.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct StyleRun {
        pub len: usize,
        pub emphasis: bool,
        pub strong: bool,
        pub code: bool,
        /// A real GFM strikethrough, not today's `InlineContainer::Flatten` loss.
        pub strike: bool,
        pub link: bool,
        /// Superscript footnote markers and math, which need their own metrics.
        pub script: Option<Script>,
        /// Set only inside a code block; drives token colour at paint.
        pub token: Option<TokenClass>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Script {
        Super,
        Sub,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TokenClass {
        Comment,
        Keyword,
        String,
        Number,
        Function,
        Type,
        Constant,
    }

    /// How an atom participates in reader behaviour.
    ///
    /// Generated ornament (list bullets, ordered markers, table rules) is not an atom at
    /// all, so it can never be selected, copied, or matched. That is the invariant, not a
    /// filter applied at three call sites.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AtomRole {
        Prose,
        Code,
        /// Image alt text and mermaid captions: selectable, but skipped by outline.
        Caption,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LinkRange {
        pub range: Range<usize>,
        pub id: LinkId,
        /// Classified at compile time against the document's own path, so the view never
        /// re-resolves a path and the app never sees a raw href.
        pub target: LinkTarget,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct LinkId {
        pub atom: AtomId,
        pub ordinal: u16,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum LinkTarget {
        Document(PathBuf),
        Web(String),
        File(PathBuf),
        /// In-document anchor; resolved to a heading at compile time.
        Anchor(HeadingId),
        Inert,
    }

    /// A block is a leaf that owns atoms, or a container that owns children. Never both.
    ///
    /// Encoding it this way is what keeps a subtree's atoms contiguous, which is what
    /// makes the reparse splice a single `Vec::splice` instead of a scatter.
    #[derive(Debug, Clone)]
    pub enum BlockBody {
        Atoms(Range<AtomIndex>),
        /// Depth-first descendant span. Direct children are walked by hopping each
        /// child's own `descendants.end`; grandchildren interleave, so this is a span,
        /// not a child list.
        Children(Range<BlockIndex>),
    }

    #[derive(Debug, Clone)]
    pub struct Block {
        pub id: BlockId,
        pub kind: BlockKind,
        pub body: BlockBody,
        /// Byte span in the document source. The reparse diff snaps to these.
        pub source: Range<usize>,
        pub depth: u16,
    }

    /// Layout-relevant block shape. Text lives in atoms; this is everything else.
    #[derive(Debug, Clone, PartialEq)]
    pub enum BlockKind {
        Heading { level: u8, id: HeadingId },
        Paragraph,
        CodeBlock { language: Option<SharedString> },
        /// A native card. No JS runtime; the diagram source renders as code.
        Mermaid,
        BlockQuote { alert: Option<AlertKind> },
        List { ordered_start: Option<u64> },
        ListItem { marker: Marker },
        Table { columns: u16, align: Vec<Align> },
        Image { source: Option<PathBuf>, alt: SharedString },
        ThematicBreak,
        /// First-class GFM footnotes, collected into one section at the document foot.
        FootnoteDefinition { label: SharedString },
        /// Inert `<script>`-free HTML that the HTML compiler could not map to a block.
        RawText,
    }

    /// GFM alerts (`> [!NOTE]`), which Electron already renders and GPUI currently drops.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AlertKind {
        Note,
        Tip,
        Important,
        Warning,
        Caution,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Marker {
        Bullet,
        Ordered(u64),
        Task { checked: bool },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum Align {
        #[default]
        Left,
        Center,
        Right,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct HeadingId(u64);

    #[derive(Debug, Clone)]
    pub struct Outline {
        pub id: HeadingId,
        pub level: u8,
        pub text: SharedString,
        /// Where the sidebar's jump lands, and what the scroll-spy compares against.
        pub block: BlockIndex,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DocKind {
        Markdown,
        /// `.html` / `.htm`, compiled to the same atoms. No WKWebView, and selection,
        /// find, copy, and outline come for free.
        Html,
    }

    /// The compiled document. Immutable between reloads; every reader capability is a
    /// query over it.
    #[derive(Debug)]
    pub struct Doc {
        pub path: Arc<Path>,
        pub kind: DocKind,
        pub title: SharedString,
        source: Arc<str>,
        /// Depth-first order. A subtree is a contiguous span.
        blocks: Vec<Block>,
        /// Document order. A block's atoms are a contiguous range.
        atoms: Vec<Atom>,
        outline: Vec<Outline>,
        next_id: u64,
    }

    /// What a reparse invalidated, so the reader can repair anchors it holds.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Reparse {
        /// Atoms replaced wholesale. Anchors whose `AtomId` no longer resolves were in
        /// here and must be dropped; everything else is still valid.
        pub replaced: Range<AtomIndex>,
        /// True when nothing changed, so the caller can skip repaint entirely.
        pub unchanged: bool,
    }

    impl Doc {
        /// Compile from scratch. Blocking; the caller decides whether to background it.
        pub fn compile(path: Arc<Path>, source: Arc<str>, kind: DocKind) -> Self {
            unimplemented!()
        }

        /// Replace the source, reusing every block whose bytes are unchanged.
        ///
        /// The diff runs over the *source*, snapped to block boundaries recorded in
        /// `Block::source`, taking the longest common prefix and suffix. Only the middle
        /// is reparsed and spliced. Ids of surviving blocks are preserved by definition,
        /// so their atoms keep their `AtomId`.
        ///
        /// Idempotent: reloading byte-identical source returns `unchanged: true` and
        /// mutates nothing, so a watcher that fires twice costs one string compare.
        pub fn reparse(&mut self, source: Arc<str>) -> Reparse {
            // TODO
            // 1. prefix = common prefix of self.source / source, snapped down to the
            //    start of the block containing it.
            // 2. suffix = common suffix, snapped up to the end of its block. Clamp so
            //    prefix <= suffix.
            // 3. Reparse source[prefix..suffix] with the block id allocator continuing
            //    from self.next_id, so new blocks never collide with kept ones.
            // 4. Splice blocks and atoms. Shift the trailing blocks' `source` spans by
            //    the byte delta; their ids and atoms are untouched.
            // 5. Rebuild `outline` (cheap: one pass over headings) and `title`.
            unimplemented!()
        }

        pub fn source(&self) -> &Arc<str> {
            unimplemented!()
        }

        pub fn atoms(&self) -> &[Atom] {
            unimplemented!()
        }

        pub fn atom(&self, index: AtomIndex) -> Option<&Atom> {
            unimplemented!()
        }

        /// Resolve a persisted identity into this compilation's order. `None` means the
        /// block was replaced by the last reparse.
        pub fn index_of(&self, id: AtomId) -> Option<AtomIndex> {
            // Linear over blocks, called at most twice per reparse (the two selection
            // endpoints) and once per find repair. A map here would be a map maintained
            // for a cold path.
            unimplemented!()
        }

        pub fn blocks(&self) -> &[Block] {
            unimplemented!()
        }

        /// Direct children of a container, walked by hopping descendant spans.
        pub fn children(&self, block: BlockIndex) -> impl Iterator<Item = BlockIndex> + '_ {
            std::iter::from_fn(|| unimplemented!())
        }

        /// Contiguous atom span for a whole subtree. Used by the splice and by
        /// block-level reveal.
        pub fn subtree_atoms(&self, block: BlockIndex) -> Range<AtomIndex> {
            unimplemented!()
        }

        /// Index of the top-level block containing an atom.
        ///
        /// The reader column's direct children are top-level blocks, so this is the
        /// argument to `ScrollHandle::scroll_to_item` when revealing an atom that is not
        /// currently painted and therefore has no geometry.
        pub fn top_level_of(&self, atom: AtomIndex) -> usize {
            unimplemented!()
        }

        pub fn outline(&self) -> &[Outline] {
            unimplemented!()
        }

        pub fn heading_block(&self, heading: HeadingId) -> Option<BlockIndex> {
            unimplemented!()
        }

        /// Clamp a byte offset to a char boundary within an atom. The only way to make a
        /// [`super::select::Cursor`].
        pub fn cursor(&self, atom: AtomIndex, offset: usize) -> Option<super::select::Cursor> {
            unimplemented!()
        }
    }

    /// Builds an [`Atom`] so that its text and runs cannot diverge.
    ///
    /// Every push appends text *and* the run that covers it, so the tiling invariant that
    /// GPUI enforces with a panic is instead enforced by the type. Adjacent runs with
    /// equal style merge, and adjacent link ranges with the same source node merge, so a
    /// `[**a** `b`](url)` yields one clickable range rather than three.
    #[derive(Debug, Default)]
    pub struct AtomBuilder {
        text: String,
        runs: Vec<StyleRun>,
        links: Vec<LinkRange>,
        code_ranges: Vec<Range<usize>>,
    }

    impl AtomBuilder {
        pub fn push(&mut self, text: &str, style: StyleRun) -> &mut Self {
            unimplemented!()
        }

        /// Open a link scope. Nested pushes join one [`LinkRange`] with one [`LinkId`],
        /// which is how two adjacent links to the same target stay distinguishable.
        pub fn link(&mut self, target: LinkTarget, body: impl FnOnce(&mut Self)) -> &mut Self {
            unimplemented!()
        }

        pub fn finish(self, id: AtomId, role: AtomRole, starts_block: bool) -> Atom {
            unimplemented!()
        }
    }

    // ── Compilers ──────────────────────────────────────────────────────────
    //
    // Each front end produces the same tree; everything downstream is shared. Adding a
    // format is one function, not a parallel renderer.

    pub mod markdown {
        use super::*;

        /// `pulldown-cmark` with GFM, tables, tasklists, strikethrough, footnotes, and
        /// math, over one source span. `first_id` continues the document's id allocator
        /// so a spliced range never collides with kept blocks.
        pub fn compile(
            source: &str,
            span: Range<usize>,
            document_path: &Path,
            first_id: u64,
        ) -> (Vec<Block>, Vec<Atom>, u64) {
            unimplemented!()
        }

        /// Byte offsets at which appending or editing cannot change what precedes them:
        /// blank lines outside fenced code, HTML blocks, and indented blocks. The
        /// reparse diff snaps to these.
        pub fn stable_boundaries(source: &str) -> Vec<usize> {
            unimplemented!()
        }
    }

    pub mod html {
        use super::*;

        /// Parse HTML into the same block tree. Scripts, styles, iframes, and event
        /// attributes are dropped at parse time — the sandbox is the absence of an
        /// executor, not a policy applied later.
        pub fn compile(source: &str, span: Range<usize>, first_id: u64) -> (Vec<Block>, Vec<Atom>, u64) {
            unimplemented!()
        }
    }

    pub mod highlight {
        use super::*;

        /// Tokenise into class runs that tile `code` exactly, including newlines.
        ///
        /// Returns classes, never colours: highlighting must not change a run's length or
        /// font, so the shaped width of a code block is identical with and without it.
        pub fn tokenize(language: Option<&str>, code: &str) -> Vec<StyleRun> {
            unimplemented!()
        }

        /// Electron's alias table (`rs` -> `rust`, `zsh` -> `bash`, ...). Unchanged.
        pub fn normalize_language(info: &str) -> String {
            unimplemented!()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// reader/select.rs — coordinate algebra. Pure, no GPUI.
// ═══════════════════════════════════════════════════════════════════════════

pub mod select {
    use super::doc::{AtomId, AtomIndex, Doc};
    use super::*;

    /// A char-boundary-clamped position. Only constructible via [`Doc::cursor`], so an
    /// offset that splits a grapheme cannot enter the system.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Cursor {
        pub atom: AtomId,
        pub offset: usize,
    }

    /// Anchor and head, in the order the user dragged them.
    ///
    /// Held by the reader entity as plain data. GPUI serialises entity updates, so there
    /// is exactly one writer and no lock; the geometry the input handlers read lives in a
    /// separate, paint-written structure ([`super::frame::FrameLog`]) that is merged with
    /// this one at a single point: [`Selection::resolve`].
    #[derive(Debug, Clone, Default)]
    pub struct Selection {
        anchor: Option<Cursor>,
        head: Option<Cursor>,
    }

    /// A selection projected onto this compilation's atom order, normalised so
    /// `start <= end`. Computed once per frame and consumed by the wash painter and by
    /// copy, so there is no cached span list to invalidate.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Resolved {
        pub start: (AtomIndex, usize),
        pub end: (AtomIndex, usize),
    }

    /// One atom's slice of a selection.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Span {
        pub atom: AtomIndex,
        pub range: Range<usize>,
        /// Copy inserts a blank line before this span. Never set on the first span.
        pub block_break: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Granularity {
        Character,
        /// Double click.
        Word,
        /// Triple click.
        Line,
    }

    impl Selection {
        pub fn is_empty(&self) -> bool {
            unimplemented!()
        }

        pub fn begin(&mut self, at: Cursor, granularity: Granularity, doc: &Doc) {
            unimplemented!()
        }

        /// Extend to a new head. Returns whether anything moved, so a drag that stays
        /// inside one glyph does not repaint.
        pub fn drag_to(&mut self, head: Cursor) -> bool {
            unimplemented!()
        }

        pub fn select_all(&mut self, doc: &Doc) {
            unimplemented!()
        }

        pub fn clear(&mut self) -> bool {
            unimplemented!()
        }

        /// Drop endpoints whose atoms did not survive a reparse.
        pub fn repair(&mut self, doc: &Doc) {
            unimplemented!()
        }

        /// Project onto document order. `None` when empty or when either endpoint is not
        /// in the current compilation.
        pub fn resolve(&self, doc: &Doc) -> Option<Resolved> {
            unimplemented!()
        }
    }

    impl Resolved {
        /// The slice of `index` this selection covers, given that atom's text length.
        /// The whole wash painter is this function plus a rect walk.
        pub fn range_for(&self, index: AtomIndex, len: usize) -> Option<Range<usize>> {
            unimplemented!()
        }

        pub fn spans(&self, doc: &Doc) -> Vec<Span> {
            unimplemented!()
        }
    }

    /// Join spans into the text the user expects on the clipboard: single newlines
    /// between atoms of one block, a blank line at each `block_break`.
    pub fn copy_text(spans: &[Span], doc: &Doc) -> String {
        unimplemented!()
    }

    pub fn word_range(text: &str, offset: usize) -> Range<usize> {
        unimplemented!()
    }

    pub fn line_range(text: &str, offset: usize) -> Range<usize> {
        unimplemented!()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// reader/find.rs — find-in-document. Pure, no GPUI.
// ═══════════════════════════════════════════════════════════════════════════

pub mod find {
    use super::doc::{AtomIndex, Doc};
    use super::*;

    /// One match, in the same coordinate space as a selection span. That identity is the
    /// point: the wash painter that draws selection draws matches, unchanged.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Hit {
        pub atom: AtomIndex,
        pub range: Range<usize>,
    }

    /// The result of a query. Hits are in document order because atoms are, so `next` is
    /// `index + 1` and needs no sort.
    #[derive(Debug, Clone, Default)]
    pub struct Matches {
        pub query: SharedString,
        pub hits: Vec<Hit>,
        /// Index into `hits`. `None` only when `hits` is empty.
        pub current: Option<usize>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MatchStatus {
        pub index: usize,
        pub total: usize,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Step {
        Next,
        Previous,
    }

    /// Case-insensitive substring over every atom's visible text, matching Electron's
    /// tree-walk semantics without a tree walk.
    pub fn search(doc: &Doc, query: &str) -> Vec<Hit> {
        unimplemented!()
    }

    impl Matches {
        /// Recompute against a new document, keeping the current match if the text under
        /// it survived. This is why a save under the user's cursor does not throw them
        /// back to match 1 of 43.
        pub fn recompute(&mut self, doc: &Doc) {
            unimplemented!()
        }

        pub fn step(&mut self, direction: Step) -> Option<Hit> {
            unimplemented!()
        }

        pub fn status(&self) -> MatchStatus {
            unimplemented!()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// reader/frame.rs — painted geometry
// ═══════════════════════════════════════════════════════════════════════════

pub mod frame {
    use super::doc::AtomIndex;
    use super::*;

    /// The geometry of the atoms painted this frame, in paint order — which is document
    /// order, because the renderer walks atoms in order.
    ///
    /// This is the one piece of shared mutable state in the design, and it exists because
    /// GPUI 0.2.2 fills a `TextLayout` during prepaint and offers no cross-element
    /// selection. It is deliberately not a general container:
    ///
    /// - **Paint writes**, through [`FrameLog::begin`] and [`FrameLog::record`], and
    ///   never reads.
    /// - **Input reads**, through [`FrameLog::hit`] and [`FrameLog::bounds_of`], and
    ///   never writes.
    ///
    /// No borrow escapes, so the two phases cannot interleave a mutation with a read.
    #[derive(Clone, Default)]
    pub struct FrameLog(Rc<RefCell<Vec<Painted>>>);

    struct Painted {
        index: AtomIndex,
        len: usize,
        layout: TextLayout,
    }

    impl FrameLog {
        /// Clear, from a zero-size canvas painted before any atom.
        pub fn begin(&self) {
            unimplemented!()
        }

        pub fn record(&self, index: AtomIndex, len: usize, layout: TextLayout) {
            unimplemented!()
        }

        /// The atom and byte offset under a point. Falls back to the vertically nearest
        /// painted atom, so a drag through a gutter, a list marker, or the space between
        /// blocks clamps to the sensible neighbour instead of stalling.
        pub fn hit(&self, position: Point<Pixels>) -> Option<(AtomIndex, usize)> {
            unimplemented!()
        }

        /// `None` for an atom that is scrolled out of view; reveal falls back to
        /// `Doc::top_level_of` plus `ScrollHandle::scroll_to_item`.
        pub fn bounds_of(&self, index: AtomIndex) -> Option<Bounds<Pixels>> {
            unimplemented!()
        }
    }

    /// One rect per visual row a byte range covers, in window coordinates, read out of
    /// the text's own layout.
    ///
    /// Walks the shaped wrap boundaries rather than `position_for_index`, because a
    /// soft-wrap boundary has two caret affinities and the generic API resolves it to the
    /// preceding row — which drops the first glyph of every continuation row.
    ///
    /// `pad_x` overhangs (inline code washes); a selection or match wash passes zero so
    /// adjacent rows tile seamlessly.
    pub fn range_rects(
        layout: &TextLayout,
        range: &Range<usize>,
        pad_x: f32,
        inset_y: f32,
    ) -> Vec<Bounds<Pixels>> {
        unimplemented!()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// reader/style.rs — the measured visual contract
// ═══════════════════════════════════════════════════════════════════════════

pub mod style {
    use super::doc::{Block, BlockKind, StyleRun, TokenClass};
    use super::*;

    /// Everything a render pass needs, derived from `Preferences` + `Theme`. Never
    /// stored beside them; recomputed and pushed, so there is one source of truth.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ReaderStyle {
        pub content_font: SharedString,
        pub code_font: SharedString,
        /// 15.5 at 100% zoom, scaled 60..=200.
        pub base_size: f32,
        /// 1.65. Pinned by the visual contract.
        pub line_height: f32,
        /// 768 / 896 / 1088, or unbounded in wide mode.
        pub column_width: Option<f32>,
        pub palette: Palette,
    }

    /// Reader colours, including the two the current build has no concept of.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Palette {
        pub foreground: Hsla,
        pub muted_foreground: Hsla,
        pub background: Hsla,
        pub muted: Hsla,
        pub border: Hsla,
        pub link: Hsla,
        pub code_wash: Hsla,
        pub selection: Hsla,
        pub match_wash: Hsla,
        pub active_match_wash: Hsla,
        pub dark: bool,
    }

    /// A cheap identity for the whole style. Bumped on any change; the run cache compares
    /// this integer rather than the struct.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct StyleEpoch(u64);

    impl ReaderStyle {
        pub fn resolve(prefs: &crate::settings::Preferences, theme: crate::theme::Theme) -> Self {
            unimplemented!()
        }

        pub fn epoch(&self) -> StyleEpoch {
            unimplemented!()
        }

        pub fn token_color(&self, class: TokenClass) -> Hsla {
            unimplemented!()
        }
    }

    /// Semantic runs to GPUI runs. The only place colour enters the pipeline.
    pub fn text_runs(runs: &[StyleRun], style: &ReaderStyle, base: BlockStyle) -> Vec<TextRun> {
        unimplemented!()
    }

    /// Per-frame memo of resolved runs, keyed by atom and invalidated wholesale when the
    /// epoch changes.
    ///
    /// Only the cheap colour mapping is invalidated by a theme switch; parsing,
    /// tokenising, and flattening are in `Doc` and survive it. That is a deliberate
    /// improvement over baking colours into cached runs.
    #[derive(Default)]
    pub struct RunCache {
        epoch: StyleEpoch,
        runs: std::collections::HashMap<doc::AtomId, Rc<[TextRun]>>,
    }

    impl RunCache {
        pub fn get_or_build(
            &mut self,
            style: &ReaderStyle,
            atom: &doc::Atom,
            base: BlockStyle,
        ) -> Rc<[TextRun]> {
            unimplemented!()
        }

        /// Drop entries for atoms replaced by a reparse.
        pub fn evict(&mut self, replaced: Range<doc::AtomIndex>, doc: &doc::Doc) {
            unimplemented!()
        }
    }

    // ── The pinned numbers ─────────────────────────────────────────────────
    //
    // These move verbatim from `ui/reader.rs`. 132 tests assert them; the reader engine
    // is a rewrite of the machinery underneath, not of the typography above it.

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct BlockStyle {
        pub font_size: f32,
        pub font_weight: u16,
        pub line_height: f32,
        pub letter_spacing_em: f32,
        pub margin_top_em: f32,
        pub margin_bottom_em: f32,
        pub muted: bool,
        pub uppercase: bool,
        pub radius: f32,
        pub padding: [f32; 2],
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct BlockSpacing {
        pub before: f32,
        pub after: f32,
    }

    impl BlockStyle {
        pub fn for_kind(kind: &BlockKind, style: &ReaderStyle) -> Self {
            unimplemented!()
        }

        pub fn heading(level: u8, style: &ReaderStyle) -> Self {
            unimplemented!()
        }
    }

    /// Adjacent-margin collapse and list grouping, unchanged in behaviour. Now takes
    /// blocks by kind rather than by the old `DocumentBlock` enum.
    pub fn block_sequence_spacing(blocks: &[Block], style: &ReaderStyle) -> Vec<BlockSpacing> {
        unimplemented!()
    }

    /// A plain bullet is suppressed when its group contains a task item.
    pub fn list_marker_is_visible(blocks: &[Block], index: usize) -> bool {
        unimplemented!()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// reader/view.rs — atoms to elements
// ═══════════════════════════════════════════════════════════════════════════

pub mod view {
    use super::doc::{Atom, AtomIndex, BlockIndex, Doc};
    use super::find::Hit;
    use super::frame::FrameLog;
    use super::select::Resolved;
    use super::style::{BlockStyle, ReaderStyle, RunCache};
    use super::*;

    /// Every wash an atom needs, in one value.
    ///
    /// Selection and find highlights are the same kind of thing — a byte range on an atom
    /// — so they share one painter and one geometry walk. Adding find cost the renderer a
    /// field, not a subsystem.
    #[derive(Clone, Copy)]
    pub struct Washes<'a> {
        pub selection: Option<Resolved>,
        pub matches: &'a [Hit],
        /// Index into `matches`, painted in the stronger colour.
        pub active: Option<usize>,
    }

    impl Washes<'_> {
        /// Every highlight range on one atom, with its colour. Binary search into
        /// `matches`, since hits are in document order.
        pub fn ranges_for(&self, index: AtomIndex, len: usize) -> Vec<(Range<usize>, Hsla)> {
            unimplemented!()
        }
    }

    /// Everything the block walk carries. One struct instead of eleven positional
    /// arguments and four `#[allow(clippy::too_many_arguments)]`.
    pub struct Paint<'a> {
        pub doc: &'a Doc,
        pub style: &'a ReaderStyle,
        pub runs: &'a mut RunCache,
        pub washes: Washes<'a>,
        pub frame: &'a FrameLog,
        pub hovered_link: Option<doc::LinkId>,
        pub focused_link: Option<doc::LinkId>,
        pub copied_code: Option<doc::BlockId>,
    }

    /// The scrolling column: reader inset, reading width, every top-level block, and the
    /// custom scrollbar. The reader column's direct children are top-level blocks, which
    /// is the contract `ScrollHandle::scroll_to_item` relies on.
    pub fn document(paint: &mut Paint<'_>, scroll: &ScrollHandle, cx: &mut Context<super::Reader>) -> AnyElement {
        unimplemented!()
    }

    fn block(paint: &mut Paint<'_>, index: BlockIndex, cx: &mut Context<super::Reader>) -> AnyElement {
        unimplemented!()
    }

    /// One selectable, decorated text element.
    ///
    /// The wash canvas is an earlier sibling than the text, so GPUI paints it underneath
    /// the glyphs while the text's prepaint has already filled the shared `TextLayout`.
    /// That ordering is what lets a pure-paint pass read real glyph geometry without a
    /// second layout pass — and it is why code washes, selection, and match highlights
    /// can never change a block's measured height.
    fn atom(
        paint: &mut Paint<'_>,
        index: AtomIndex,
        base: BlockStyle,
        cx: &mut Context<super::Reader>,
    ) -> AnyElement {
        unimplemented!()
    }

    /// Mouse handling for selection, installed once per frame at the reader root rather
    /// than once per atom: the frame log already holds every atom's geometry, so three
    /// closures replace three-per-paragraph and a mouse move costs one scan instead of
    /// one dispatch per visible block.
    pub fn install_selection_input(
        window: &mut Window,
        reader: gpui::WeakEntity<super::Reader>,
        frame: FrameLog,
    ) {
        unimplemented!()
    }

    /// The overlay scrollbar. Geometry unchanged from the current build; it now hangs off
    /// the reader's own scroll handle instead of a map in `MdowApp`.
    pub fn scrollbar(scroll: &ScrollHandle, style: &ReaderStyle, cx: &mut Context<super::Reader>) -> Option<AnyElement> {
        unimplemented!()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// reader/mod.rs — the public surface
// ═══════════════════════════════════════════════════════════════════════════

use doc::{DocKind, HeadingId, LinkId, LinkTarget, Outline};
use find::{MatchStatus, Matches, Step};
use frame::FrameLog;
use select::Selection;
use style::{ReaderStyle, RunCache};

/// One open document.
///
/// Owns the compiled text, its own scroll position, its selection, its find session, and
/// its transient interaction state. Ten methods and a `Render` impl hide parsing,
/// incremental reload, syntax highlighting, text shaping, glyph geometry, hit-testing,
/// selection algebra, clipboard formatting, match ordering, and scroll reveal.
pub struct Reader {
    doc: doc::Doc,
    style: ReaderStyle,
    runs: RunCache,
    selection: Selection,
    matches: Matches,
    /// Debounce for recomputing matches while the user types, matching Electron's 120ms.
    query_debounce: Option<gpui::Task<()>>,
    scroll: ScrollHandle,
    frame: FrameLog,
    hovered_link: Option<LinkId>,
    focused_link: Option<LinkId>,
    link_focus: std::collections::HashMap<LinkId, FocusHandle>,
    copied_code: Option<(doc::BlockId, Instant)>,
    scrollbar_drag: Option<f32>,
    focus: FocusHandle,
}

/// What the reader tells the app. Everything else it handles itself.
#[derive(Debug, Clone)]
pub enum ReaderEvent {
    /// Already classified. The app decides whether that means a new tab or `open::that`.
    ActivateLink(LinkTarget),
    /// The outline sidebar and the breadcrumb should redraw.
    OutlineChanged,
    /// The find bar's `N of M` should redraw.
    MatchesChanged,
}

#[derive(Debug, Clone, Copy)]
pub enum RevealTarget {
    Heading(HeadingId),
    CurrentMatch,
    Top,
}

impl EventEmitter<ReaderEvent> for Reader {}

impl Reader {
    pub fn open(path: Arc<Path>, source: Arc<str>, kind: DocKind, cx: &mut Context<Self>) -> Self {
        unimplemented!()
    }

    /// Splice in new source. Preserves scroll, selection, and the current match wherever
    /// their blocks survived. Byte-identical source is a no-op.
    pub fn reload(&mut self, source: Arc<str>, cx: &mut Context<Self>) {
        // TODO
        // let reparse = self.doc.reparse(source);
        // if reparse.unchanged { return }
        // self.runs.evict(reparse.replaced, &self.doc);
        // self.selection.repair(&self.doc);
        // self.matches.recompute(&self.doc);
        // emit OutlineChanged + MatchesChanged; cx.notify()
        unimplemented!()
    }

    /// Idempotent. Only bumps the run cache when the resolved style actually differs, so
    /// the app can push this unconditionally every frame.
    pub fn set_style(&mut self, style: ReaderStyle, cx: &mut Context<Self>) {
        unimplemented!()
    }

    pub fn path(&self) -> &Path {
        unimplemented!()
    }

    pub fn title(&self) -> &str {
        unimplemented!()
    }

    /// The sidebar's outline list and the scroll-spy's input. Not a pass-through: entries
    /// carry the block index that [`Reader::reveal`] consumes.
    pub fn outline(&self) -> &[Outline] {
        unimplemented!()
    }

    /// The heading currently at the top of the viewport, for outline highlighting.
    pub fn active_heading(&self) -> Option<HeadingId> {
        unimplemented!()
    }

    pub fn reveal(&mut self, target: RevealTarget, cx: &mut Context<Self>) {
        unimplemented!()
    }

    /// `None` clears the session. Debounced; emits `MatchesChanged` when hits settle.
    pub fn set_query(&mut self, query: Option<SharedString>, cx: &mut Context<Self>) {
        unimplemented!()
    }

    pub fn step_match(&mut self, direction: Step, cx: &mut Context<Self>) {
        unimplemented!()
    }

    /// `None` while there is no query. `Some` with `total == 0` means no results, which
    /// is a different thing the find bar renders differently.
    pub fn match_status(&self) -> Option<MatchStatus> {
        unimplemented!()
    }

    /// The clipboard text for the current selection, with block breaks preserved.
    pub fn selection_text(&self) -> Option<String> {
        unimplemented!()
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        unimplemented!()
    }

    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        unimplemented!()
    }

    /// `home` / `end` / `pageup` / `pagedown`, clamped to the scroll extent. Returns
    /// whether the key was consumed.
    pub fn scroll_by_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        unimplemented!()
    }
}

impl Focusable for Reader {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        unimplemented!()
    }
}

impl Render for Reader {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // TODO
        // - reconcile link focus handles against doc links (drop dead, create new)
        // - resolve selection once, build Washes, build Paint
        // - view::install_selection_input(window, cx.weak_entity(), self.frame.clone())
        // - view::document(&mut paint, &self.scroll, cx)
        gpui::div()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// chrome.rs — product surface. Typed, thin, not load-bearing.
// ═══════════════════════════════════════════════════════════════════════════

pub mod chrome {
    use super::*;

    /// Overlay state, shaped so the illegal combinations cannot be written.
    ///
    /// Electron keeps four independent booleans, which permits settings over a palette
    /// over a shortcuts sheet. Two fields instead of four, because find genuinely *is* a
    /// different layer: it is non-modal, it coexists with a readable document, and it
    /// keeps the reader's selection alive. Collapsing it into the modal enum would encode
    /// a lie.
    #[derive(Default)]
    pub struct Chrome {
        modal: Option<Modal>,
        find: Option<FindBar>,
        pub sidebar: Sidebar,
    }

    pub enum Modal {
        CommandPalette(palette::CommandPalette),
        Settings(settings_dialog::SettingsDialog),
        Shortcuts,
    }

    pub struct FindBar {
        pub field: Entity<text_field::TextField>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Sidebar {
        pub open: bool,
        pub mode: SidebarMode,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SidebarMode {
        Recents,
        Folder,
        Outline,
    }

    impl Chrome {
        /// Opening a modal closes any other. Exclusivity lives here, once.
        pub fn open_modal(&mut self, modal: Modal, window: &mut Window) {
            unimplemented!()
        }

        pub fn open_find(&mut self, seed: Option<String>, window: &mut Window, cx: &mut App) {
            unimplemented!()
        }

        /// Close the topmost layer: modal first, then find. Returns whether anything
        /// closed, so Escape can fall through to the reader when nothing did.
        pub fn dismiss(&mut self) -> bool {
            unimplemented!()
        }

        pub fn find_query(&self, cx: &App) -> Option<SharedString> {
            unimplemented!()
        }
    }

    /// One-line text input over `EntityInputHandler`, following GPUI's `examples/input.rs`.
    /// Shared by the find bar and the palette so there is one place that knows how IME,
    /// selection, and caret painting work in a Mdow input.
    pub mod text_field {
        use super::*;

        pub struct TextField {
            content: SharedString,
            selected: Range<usize>,
            focus: FocusHandle,
        }

        pub enum TextFieldEvent {
            Changed(SharedString),
            Submitted(SharedString),
            Cancelled,
        }

        impl TextField {
            pub fn new(placeholder: &str, cx: &mut Context<Self>) -> Self {
                unimplemented!()
            }

            pub fn text(&self) -> &SharedString {
                unimplemented!()
            }

            pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
                unimplemented!()
            }
        }
    }

    pub mod palette {
        use super::*;

        /// A command is a name, an optional shortcut, and an action. The palette does not
        /// know what any of them do.
        pub struct Command {
            pub name: &'static str,
            pub keystroke: Option<&'static str>,
            pub action: Box<dyn gpui::Action>,
        }

        pub struct CommandPalette {
            field: Entity<text_field::TextField>,
            /// Commands plus open tabs plus workspace files, scored together.
            candidates: Vec<Candidate>,
            selected: usize,
        }

        pub enum Candidate {
            Command(&'static str),
            Tab(PathBuf),
            File(PathBuf),
        }

        /// Subsequence match with a contiguity and word-boundary bonus. Returns `None`
        /// for a non-match so the filter is a `filter_map`.
        pub fn score(query: &str, candidate: &str) -> Option<u32> {
            unimplemented!()
        }
    }

    pub mod settings_dialog {
        use super::*;

        pub struct SettingsDialog {
            focus: FocusHandle,
        }

        pub enum SettingsEvent {
            Changed(crate::settings::Preferences),
            ResetToDefaults,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// settings.rs — preferences and session
// ═══════════════════════════════════════════════════════════════════════════

pub mod settings {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ThemePreference {
        #[default]
        System,
        Light,
        Dark,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum InterfaceScale {
        #[default]
        Compact,
        Comfortable,
        Large,
    }

    /// Replaces the current `wide_mode: bool`. Electron's three widths are 48rem / 56rem /
    /// 68rem; wide mode is a separate override that ignores the cap.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ReadingWidth {
        #[default]
        Standard,
        Comfortable,
        Wide,
    }

    impl ReadingWidth {
        pub const fn pixels(self) -> f32 {
            match self {
                Self::Standard => 768.0,
                Self::Comfortable => 896.0,
                Self::Wide => 1088.0,
            }
        }
    }

    /// Clamped 60..=200 in steps of 10 by construction, so no caller can produce an
    /// out-of-range zoom and no consumer needs to re-check.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ZoomLevel(u8);

    impl ZoomLevel {
        pub const DEFAULT: Self = Self(100);

        pub fn stepped(self, delta: i8) -> Self {
            unimplemented!()
        }

        pub fn scale(self) -> f32 {
            unimplemented!()
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Preferences {
        pub theme: ThemePreference,
        pub content_font: SharedString,
        pub code_font: SharedString,
        pub interface_scale: InterfaceScale,
        pub reading_width: ReadingWidth,
        pub wide_mode: bool,
        pub zoom: ZoomLevel,
    }

    /// Everything restored on launch. One value, one file, one write.
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub struct Session {
        pub tabs: Vec<PathBuf>,
        pub active: Option<PathBuf>,
        pub workspace: Option<PathBuf>,
        pub recents: Vec<PathBuf>,
        pub sidebar: crate::chrome::SidebarMode,
        pub sidebar_open: bool,
        pub preferences: Preferences,
    }

    pub struct Store {
        path: PathBuf,
        last_written: Option<Session>,
        pending: Option<gpui::Task<()>>,
    }

    impl Store {
        /// A missing, unreadable, or malformed file yields defaults. Never an error the
        /// user has to see: losing a session is not worth a dialog.
        pub fn load(directory: &Path) -> (Self, Session) {
            unimplemented!()
        }

        /// Debounced and idempotent: equal to `last_written` writes nothing, and the
        /// write itself is temp-file-plus-rename, so a crash mid-save leaves the previous
        /// session intact rather than a truncated file.
        pub fn save(&mut self, session: &Session, cx: &mut App) {
            unimplemented!()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// theme.rs — additions only
// ═══════════════════════════════════════════════════════════════════════════

pub mod theme {
    use super::*;

    /// Unchanged palette; `for_appearance` gains a preference override.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Theme {
        /* ... existing fields, plus: */
        pub selection: Hsla,
        pub match_wash: Hsla,
        pub active_match_wash: Hsla,
    }

    impl Theme {
        /// `System` follows the window; `Light` and `Dark` pin it.
        pub fn resolve(
            preference: crate::settings::ThemePreference,
            appearance: gpui::WindowAppearance,
        ) -> Self {
            unimplemented!()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// app.rs — what is left of MdowApp
// ═══════════════════════════════════════════════════════════════════════════

pub mod app {
    use super::*;

    /// One tab, one reader.
    ///
    /// Per-tab scroll, selection, and find state are per-reader by construction. Two tabs
    /// cannot fight over them, so there is no shared reader state to reconcile and no
    /// `clear_reader_transient_state` to remember to call.
    pub struct Tab {
        pub reader: Entity<super::Reader>,
        pub reload_error: Option<String>,
        _subscription: gpui::Subscription,
    }

    pub struct MdowApp {
        tabs: TabSet,
        workspace: Option<crate::workspace::WorkspaceTree>,
        workspace_error: Option<UserFacingError>,
        open_error: Option<UserFacingError>,
        chrome: crate::chrome::Chrome,
        prefs: crate::settings::Preferences,
        session: crate::settings::Store,
        drop_state: DropState,
        watcher: crate::watcher::FileWatcher,
        focus: FocusHandle,
    }

    pub struct TabSet {
        tabs: Vec<Tab>,
        active: Option<PathBuf>,
    }

    impl MdowApp {
        pub fn restore(session: crate::settings::Session, window: &mut Window, cx: &mut Context<Self>) -> Self {
            unimplemented!()
        }

        /// Opens markdown or HTML; a folder becomes the workspace. Reuses the existing
        /// tab for a path already open, so opening twice is idempotent.
        pub fn open_path(&mut self, path: &Path, cx: &mut Context<Self>) {
            unimplemented!()
        }

        /// The single link policy, reached from `ReaderEvent::ActivateLink`.
        fn follow(&mut self, target: doc::LinkTarget, cx: &mut Context<Self>) {
            unimplemented!()
        }

        fn active_reader(&self) -> Option<&Entity<super::Reader>> {
            unimplemented!()
        }
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct DropState {
        active: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UserFacingError {
        pub title: String,
        pub body: String,
        pub path: PathBuf,
    }
}

// Placeholders for modules this sketch does not reshape.
pub mod watcher {
    pub struct FileWatcher;
}
pub mod workspace {
    pub struct WorkspaceTree;
}
