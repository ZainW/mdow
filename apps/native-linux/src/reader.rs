//! The document reader: a virtualized `GtkListView` with one row per top-level block, so even
//! huge documents open instantly and scroll smoothly. Floating on top: the find bar (top right)
//! and the zoom indicator (bottom right).

use crate::doc::{Block, CodeChunk, Doc, Heading, gaps, inline_text};
use crate::files::{self, LinkTarget};
use crate::highlight::{self, Span};
use crate::render::{self, Builder, Host, Style};
use adw::prelude::*;
use gtk::{gdk, gio, glib, graphene, pango};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

/// What a reader asks of the window that owns it.
pub trait ReaderHost {
    fn open_document(&self, path: PathBuf, anchor: Option<String>, new_tab: bool);
    fn heading_changed(&self, reader: &Reader, index: Option<usize>);
    fn zoom(&self, step: i32);
}

/// Reading-frame padding.
pub const PAD_X: i32 = 48;

/// The width of a text column holding `chars` characters of `family` at `size` px, side padding
/// included.
pub fn text_column(context: &pango::Context, family: &str, size: f64, chars: f64) -> i32 {
    let mut font = pango::FontDescription::from_string(family);
    font.set_absolute_size(size * pango::SCALE as f64);
    let metrics = context.metrics(Some(&font), None);
    let char_width = metrics.approximate_char_width() as f64 / pango::SCALE as f64;
    (chars * char_width).round() as i32 + 2 * PAD_X
}

/// How wide rows may grow: text stops at a comfortable measure, while code, tables, diagrams,
/// images and formulas, which don't wrap like prose, may use the wider column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    pub text: i32,
    pub wide: i32,
}
const PAD_Y: f64 = 32.0;
/// Where an anchored heading lands.
const HEADING_OFFSET: f64 = 24.0;

/// `GtkListView` keeps up to 200 rows bound around the view, though a screen shows a dozen or
/// two. Building and measuring them all is most of the cost of opening a document or restyling
/// one, so only the rows near the view are built at once: this many above its top row and below
/// it. The rest start as blanks of about the right height, and are built as they near the view
/// or a few a frame after that, whichever comes first.
const EAGER_ABOVE: usize = 4;
const EAGER_BELOW: usize = 24;
/// Rows kept built past the last one in view.
const EAGER_PAST: usize = 8;
/// How long a frame spends building rows that were put off.
const FILL_SLICE: std::time::Duration = std::time::Duration::from_millis(3);

/// `MDOW_EAGER_ROWS=1` builds every bound row at once, as a check on the above.
fn eager_rows() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("MDOW_EAGER_ROWS").is_some())
}

struct BoundRow {
    clamp: adw::Clamp,
    labels: Vec<gtk::Label>,
    /// Whether the row's widgets are built, or it is still a blank.
    built: bool,
}

/// Roughly how many lines of text a block takes at `per_line` characters a line, for the height
/// of a row that isn't built yet.
fn block_lines(block: &Block, per_line: f64) -> f64 {
    let wrapped = |chars: usize| (chars as f64 / per_line).ceil().max(1.0);
    let all = |blocks: &[Block]| blocks.iter().map(|b| block_lines(b, per_line)).sum::<f64>();
    match block {
        Block::Heading { inlines, .. } => wrapped(inline_text(inlines).len()) * 1.4,
        Block::Paragraph(inlines) => wrapped(inline_text(inlines).len()),
        Block::Code(chunk) => chunk.text().lines().count() as f64 * 0.85 + 1.0,
        Block::Quote(blocks) | Block::Alert(_, blocks) => all(blocks) + 1.0,
        Block::List(list) => list.items.iter().map(|item| all(&item.blocks) + 0.2).sum(),
        Block::Table(table) => (table.rows.len() + 1) as f64 * 1.6,
        Block::Footnotes(notes) => notes.iter().map(|n| all(&n.blocks)).sum::<f64>() + 1.0,
        Block::Mermaid(_) | Block::Images(_) => 10.0,
        Block::Math(_) => 3.0,
        Block::Rule => 0.5,
    }
}

struct DocState {
    doc: Doc,
    gaps: Vec<f64>,
    texts: RefCell<Option<Rc<Vec<Vec<String>>>>>,
}

#[derive(Clone, Copy, Debug)]
struct Match {
    row: usize,
    label: usize,
    start: u32,
    end: u32,
}

#[derive(Default)]
struct FindState {
    query: String,
    matches: Vec<Match>,
    current: Option<usize>,
}

enum Spans {
    Pending,
    Ready(Rc<Vec<Span>>),
}

#[derive(Clone, Copy)]
enum Align {
    /// Put the row's content (or the matched line) this far below the top.
    Top(f64),
    /// Put the row's own top edge, margin included, this far below the top (restoring a
    /// position captured by `anchor`).
    RowTop(f64),
    /// Keep a find match a third of the way down when it has to move.
    Match,
}

pub struct Inner {
    pub root: gtk::Overlay,
    scroller: gtk::ScrolledWindow,
    list: gtk::ListView,
    store: gio::ListStore,
    doc: RefCell<Option<Rc<DocState>>>,
    style: RefCell<Rc<Style>>,
    column: Cell<Option<Columns>>,
    bound: RefCell<HashMap<usize, BoundRow>>,
    find: RefCell<FindState>,
    find_bar: gtk::Box,
    find_entry: gtk::SearchEntry,
    find_count: gtk::Label,
    find_prev: gtk::Button,
    find_next: gtk::Button,
    zoom_hud: gtk::Box,
    zoom_label: gtk::Label,
    zoom_reset: gtk::Button,
    zoom_level: Cell<u32>,
    zoom_timer: RefCell<Option<glib::SourceId>>,
    zoom_hover: Cell<bool>,
    spans: RefCell<HashMap<usize, Spans>>,
    host: RefCell<Option<Weak<dyn ReaderHost>>>,
    active_heading: Cell<Option<usize>>,
    heading_idle: Cell<bool>,
    path: RefCell<PathBuf>,
    animation: RefCell<Option<adw::TimedAnimation>>,
    reveal_generation: Cell<u64>,
    zoom_accum: Cell<f64>,
    /// The first `g` of `gg`, `gt` or `gT` was just typed.
    pending_g: Cell<bool>,
    /// When the document was set, for `MDOW_PERF`.
    opened: Cell<Option<std::time::Instant>>,
    /// The row expected at the top of the view: which rows to build before any are laid out.
    near_top: Cell<usize>,
    /// A tick callback is building the rows that were put off.
    filling: Cell<bool>,
    /// The frame and how many of its layout passes built rows that had come into view.
    layout_passes: Cell<(i64, u32)>,
    layout_hook: RefCell<Option<(gdk::FrameClock, glib::SignalHandlerId)>>,
}

#[derive(Clone)]
pub struct Reader(Rc<Inner>);

struct HostRef(Weak<Inner>);

impl Host for HostRef {
    fn activate_link(&self, uri: &str) {
        if let Some(inner) = self.0.upgrade() {
            Reader(inner).activate_link(uri);
        }
    }
    fn spans(&self, chunk: &CodeChunk) -> Option<Rc<Vec<Span>>> {
        self.0.upgrade().and_then(|inner| Reader(inner).spans_for(chunk))
    }
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    crate::set_accessible_name(&button, tooltip);
    button.add_css_class("flat");
    button.set_tooltip_text(Some(tooltip));
    button.set_valign(gtk::Align::Center);
    button
}

impl Reader {
    pub fn new(style: Rc<Style>, column: Option<Columns>) -> Reader {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let factory = gtk::SignalListItemFactory::new();
        let list = gtk::ListView::new(
            Some(gtk::NoSelection::new(Some(store.clone()))),
            Some(factory.clone()),
        );
        list.set_single_click_activate(false);
        list.set_can_focus(false);
        // Rows get their natural height; the default (minimum) collapses shrinkable pictures.
        list.set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        list.set_tab_behavior(gtk::ListTabBehavior::Item);
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&list)
            .vexpand(true)
            .hexpand(true)
            .build();
        scroller.add_css_class("mdow-reader");
        // Focusable, so Page Down / Space / arrows scroll the document as in a browser.
        scroller.set_focusable(true);

        let root = gtk::Overlay::new();
        root.set_child(Some(&scroller));
        root.set_vexpand(true);
        root.set_hexpand(true);

        // Find bar: a floating sheet in the top-right corner.
        let find_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        find_bar.add_css_class("mdow-find");
        find_bar.set_halign(gtk::Align::End);
        find_bar.set_valign(gtk::Align::Start);
        find_bar.set_visible(false);
        let find_entry = gtk::SearchEntry::new();
        find_entry.set_placeholder_text(Some("Search…"));
        find_entry.set_search_delay(80);
        find_entry.set_width_chars(18);
        let find_count = gtk::Label::new(None);
        find_count.add_css_class("mdow-find-count");
        let hint = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        hint.add_css_class("mdow-find-hint");
        let kbd = gtk::Label::new(Some("↵"));
        kbd.add_css_class("mdow-kbd");
        hint.append(&kbd);
        hint.append(&gtk::Label::new(Some("next")));
        hint.set_tooltip_text(Some("Enter: next match · Shift+Enter: previous · Esc: close"));
        let find_prev = icon_button("go-up-symbolic", "Previous match (Shift+Enter)");
        let find_next = icon_button("go-down-symbolic", "Next match (Enter)");
        let find_close = icon_button("window-close-symbolic", "Close (Esc)");
        find_bar.append(&find_entry);
        find_bar.append(&find_count);
        find_bar.append(&hint);
        find_bar.append(&find_prev);
        find_bar.append(&find_next);
        find_bar.append(&find_close);
        root.add_overlay(&find_bar);

        // Zoom indicator.
        let zoom_hud = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        zoom_hud.add_css_class("mdow-zoom");
        zoom_hud.set_halign(gtk::Align::End);
        zoom_hud.set_valign(gtk::Align::End);
        zoom_hud.set_margin_end(16);
        zoom_hud.set_margin_bottom(16);
        zoom_hud.set_visible(false);
        let zoom_out = icon_button("list-remove-symbolic", "Zoom out");
        let zoom_label = gtk::Label::new(Some("100%"));
        let zoom_in = icon_button("list-add-symbolic", "Zoom in");
        let zoom_reset = icon_button("edit-undo-symbolic", "Reset zoom");
        zoom_hud.append(&zoom_out);
        zoom_hud.append(&zoom_label);
        zoom_hud.append(&zoom_in);
        zoom_hud.append(&zoom_reset);
        root.add_overlay(&zoom_hud);

        let inner = Rc::new(Inner {
            root,
            scroller,
            list,
            store,
            doc: RefCell::new(None),
            style: RefCell::new(style),
            column: Cell::new(column),
            bound: RefCell::new(HashMap::new()),
            find: RefCell::new(FindState::default()),
            find_bar,
            find_entry,
            find_count,
            find_prev,
            find_next,
            zoom_hud,
            zoom_label,
            zoom_reset,
            zoom_level: Cell::new(100),
            zoom_timer: RefCell::new(None),
            zoom_hover: Cell::new(false),
            spans: RefCell::new(HashMap::new()),
            host: RefCell::new(None),
            active_heading: Cell::new(None),
            heading_idle: Cell::new(false),
            path: RefCell::new(PathBuf::new()),
            animation: RefCell::new(None),
            reveal_generation: Cell::new(0),
            zoom_accum: Cell::new(0.0),
            pending_g: Cell::new(false),
            opened: Cell::new(None),
            near_top: Cell::new(0),
            filling: Cell::new(false),
            layout_passes: Cell::new((0, 0)),
            layout_hook: RefCell::new(None),
        });
        let reader = Reader(inner);
        reader.connect_factory(&factory);
        reader.connect_find(&find_close);
        reader.connect_zoom(&zoom_out, &zoom_in);
        reader.connect_scrolling();
        reader.connect_row_building();
        reader
    }

    pub fn widget(&self) -> &gtk::Overlay {
        &self.0.root
    }

    pub fn set_host(&self, host: Weak<dyn ReaderHost>) {
        self.0.host.replace(Some(host));
    }

    fn host(&self) -> Option<Rc<dyn ReaderHost>> {
        self.0.host.borrow().as_ref().and_then(Weak::upgrade)
    }

    fn weak(&self) -> Weak<Inner> {
        Rc::downgrade(&self.0)
    }

    pub fn path(&self) -> PathBuf {
        self.0.path.borrow().clone()
    }

    pub fn headings(&self) -> Vec<Heading> {
        self.0.doc.borrow().as_ref().map(|d| d.doc.headings.clone()).unwrap_or_default()
    }

    pub fn has_document(&self) -> bool {
        self.0.doc.borrow().is_some()
    }

    pub fn active_heading(&self) -> Option<usize> {
        self.0.active_heading.get()
    }

    fn connect_factory(&self, factory: &gtk::SignalListItemFactory) {
        let weak = self.weak();
        factory.connect_setup(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            item.set_activatable(false);
            item.set_selectable(false);
            item.set_focusable(false);
            let clamp = adw::Clamp::new();
            if let Some(inner) = weak.upgrade() {
                Reader(inner).configure_clamp(&clamp, false);
            }
            item.set_child(Some(&clamp));
        });
        let weak = self.weak();
        factory.connect_bind(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let (Some(inner), Some(obj)) = (weak.upgrade(), item.item()) else { return };
            let Some(clamp) = item.child().and_downcast::<adw::Clamp>() else { return };
            let Ok(boxed) = obj.downcast::<glib::BoxedAnyObject>() else { return };
            let index = *boxed.borrow::<usize>();
            Reader(inner).bind_row(index, &clamp);
        });
        let weak = self.weak();
        factory.connect_unbind(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let (Some(inner), Some(obj)) = (weak.upgrade(), item.item()) else { return };
            let Ok(boxed) = obj.downcast::<glib::BoxedAnyObject>() else { return };
            let index = *boxed.borrow::<usize>();
            let Some(clamp) = item.child().and_downcast::<adw::Clamp>() else { return };
            // When the model is replaced, the new row at this index may already be bound to
            // another list item; only forget the entry if it is this item's.
            {
                let mut bound = inner.bound.borrow_mut();
                if bound.get(&index).is_some_and(|row| row.clamp == clamp) {
                    bound.remove(&index);
                }
            }
            clamp.set_child(None::<&gtk::Widget>);
        });
    }

    fn configure_clamp(&self, clamp: &adw::Clamp, wide: bool) {
        match self.0.column.get() {
            Some(columns) => {
                let width = if wide { columns.wide } else { columns.text };
                clamp.set_maximum_size(width);
                clamp.set_tightening_threshold(width);
            }
            None => {
                clamp.set_maximum_size(i32::MAX / 4);
                clamp.set_tightening_threshold(i32::MAX / 4);
            }
        }
    }

    fn bind_row(&self, index: usize, clamp: &adw::Clamp) {
        let top = self.0.near_top.get();
        if eager_rows() || (index + EAGER_ABOVE >= top && index < top + EAGER_BELOW) {
            self.build_row(index, clamp);
        } else {
            self.defer_row(index, clamp, None);
        }
    }

    /// Stand a blank in for the row, `height` tall or about as tall as the row should be.
    fn defer_row(&self, index: usize, clamp: &adw::Clamp, height: Option<i32>) {
        let height = height.or_else(|| self.estimate_height(index)).unwrap_or(1);
        let blank = gtk::Box::new(gtk::Orientation::Vertical, 0);
        blank.set_size_request(-1, height.max(1));
        clamp.set_child(Some(&blank));
        self.0
            .bound
            .borrow_mut()
            .insert(index, BoundRow { clamp: clamp.clone(), labels: Vec::new(), built: false });
        self.schedule_fill();
    }

    fn estimate_height(&self, index: usize) -> Option<i32> {
        let state = self.0.doc.borrow().clone()?;
        let block = state.doc.blocks.get(index)?;
        let style = self.0.style.borrow().clone();
        let mut width = self.0.list.width().max(400);
        if let Some(columns) = self.0.column.get() {
            width = width.min(columns.text);
        }
        // A character of running text is about half an em wide.
        let per_line = ((width - 2 * PAD_X) as f64 / (style.fs * 0.5)).max(20.0);
        let mut height = state.gaps[index] * style.fs
            + block_lines(block, per_line) * style.fs * style.line_height;
        if index == 0 {
            height += PAD_Y;
        }
        Some(height.round() as i32)
    }

    /// The bound rows that are still blanks.
    fn unbuilt(&self) -> Vec<(usize, adw::Clamp)> {
        let bound = self.0.bound.borrow();
        bound.iter().filter(|(_, r)| !r.built).map(|(i, r)| (*i, r.clamp.clone())).collect()
    }

    /// The rows in view and a few either side, first to last. `GtkListView` only lays out
    /// the rows in view, so the others' positions say nothing; hit-testing does.
    fn rows_near_view(&self) -> Option<(usize, usize)> {
        let (top, _) = self.anchor()?;
        // Nothing under the bottom edge means the document ends above it.
        let bottom = self.row_at(self.0.list.height() as f64 - 0.5).map_or(usize::MAX, |r| r.0);
        Some((top.saturating_sub(EAGER_ABOVE), bottom.saturating_add(EAGER_PAST)))
    }

    /// Build the blanks that have come into view or near it. Runs after every layout, and
    /// building one asks for another layout in the same frame, so a blank is never drawn.
    fn build_visible(&self, frame: i64) {
        let unbuilt = self.unbuilt();
        if unbuilt.is_empty() {
            return;
        }
        // GTK lays out at most four times a frame; past three, leave the rest to the next.
        let (last, passes) = self.0.layout_passes.get();
        let passes = if last == frame { passes } else { 0 };
        if passes >= 3 {
            return;
        }
        let Some((first, last)) = self.rows_near_view() else { return };
        let mut any = false;
        for (index, clamp) in unbuilt {
            if (first..=last).contains(&index) {
                self.build_row(index, &clamp);
                any = true;
            }
        }
        self.0.layout_passes.set((frame, passes + any as u32));
    }

    fn schedule_fill(&self) {
        if eager_rows() || self.0.filling.replace(true) {
            return;
        }
        // Once a frame rather than when idle: an idle would build every row before the next
        // frame is due, and that frame would then measure them all. Ticks stop while the
        // reader isn't showing, so a tab in the background builds its rows when it is.
        let weak = self.weak();
        let first = Cell::new(true);
        self.0.list.add_tick_callback(move |_, _| {
            let Some(inner) = weak.upgrade() else { return glib::ControlFlow::Break };
            // The frame the rows were put off for goes out first.
            if first.replace(false) || Reader(inner.clone()).fill_some() {
                return glib::ControlFlow::Continue;
            }
            inner.filling.set(false);
            glib::ControlFlow::Break
        });
    }

    /// Build a few of the blanks, nearest the view first. Returns whether any are left.
    fn fill_some(&self) -> bool {
        let top = self.0.near_top.get();
        let mut unbuilt = self.unbuilt();
        unbuilt.sort_by_key(|(index, _)| index.abs_diff(top));
        let started = std::time::Instant::now();
        let mut left = unbuilt.len();
        for (index, clamp) in unbuilt {
            if started.elapsed() > FILL_SLICE {
                break;
            }
            self.build_row(index, &clamp);
            left -= 1;
        }
        left > 0
    }

    fn connect_row_building(&self) {
        let weak = self.weak();
        self.0.list.connect_realize(move |list| {
            let (Some(inner), Some(clock)) = (weak.upgrade(), list.frame_clock()) else { return };
            let weak = Rc::downgrade(&inner);
            let id = clock.connect_local("layout", true, move |args| {
                let clock = args.first().and_then(|v| v.get::<gdk::FrameClock>().ok());
                if let (Some(inner), Some(clock)) = (weak.upgrade(), clock) {
                    Reader(inner).build_visible(clock.frame_counter());
                }
                None
            });
            inner.layout_hook.replace(Some((clock, id)));
        });
        let weak = self.weak();
        self.0.list.connect_unrealize(move |_| {
            if let Some(inner) = weak.upgrade()
                && let Some((clock, id)) = inner.layout_hook.take()
            {
                clock.disconnect(id);
            }
        });
    }

    fn build_row(&self, index: usize, clamp: &adw::Clamp) {
        let Some(state) = self.0.doc.borrow().clone() else { return };
        let Some(block) = state.doc.blocks.get(index) else { return };
        let style = self.0.style.borrow().clone();
        let host: Rc<dyn Host> = Rc::new(HostRef(self.weak()));
        let mut builder = Builder::new(&style, host);
        let widget = builder.build(block);
        let last = index + 1 == state.doc.blocks.len();
        let mut top = state.gaps[index] * style.fs;
        if index == 0 {
            top += PAD_Y;
        }
        widget.set_margin_top(top.round() as i32);
        widget.set_margin_bottom(if last { (PAD_Y * 2.0) as i32 } else { 0 });
        widget.set_margin_start(PAD_X);
        widget.set_margin_end(PAD_X);
        widget.set_hexpand(true);
        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        wrapper.add_css_class("md");
        wrapper.append(&widget);
        let wide = matches!(
            block,
            Block::Code(_)
                | Block::Mermaid(_)
                | Block::Table(_)
                | Block::Images(_)
                | Block::Math(_)
        );
        self.configure_clamp(clamp, wide);
        clamp.set_child(Some(&wrapper));
        self.0
            .bound
            .borrow_mut()
            .insert(index, BoundRow { clamp: clamp.clone(), labels: builder.labels, built: true });
        self.apply_find_row(index);
    }

    /// Rebuild the rows in and near the view (theme, fonts, zoom or width changed). The other
    /// bound rows become blanks of the height they had, times `scale`, and are built later.
    fn rebuild_bound(&self, scale: f64) {
        let showing = self.0.list.is_mapped();
        let near = if showing { self.rows_near_view() } else { None };
        let width = self.0.list.width();
        let rows: Vec<(usize, adw::Clamp)> =
            self.0.bound.borrow().iter().map(|(i, r)| (*i, r.clamp.clone())).collect();
        for (index, clamp) in rows {
            if eager_rows() || near.is_some_and(|(first, last)| (first..=last).contains(&index)) {
                self.build_row(index, &clamp);
                continue;
            }
            // The list view has measured every bound row at this width, so asking again is a
            // cache hit; a reader that isn't showing may never have been measured.
            let height =
                if showing { clamp.measure(gtk::Orientation::Vertical, width).1 } else { 0 };
            let height = (height > 0).then(|| (height as f64 * scale).round() as i32);
            self.defer_row(index, &clamp, height);
        }
    }

    pub fn set_style(&self, style: Rc<Style>, column: Option<Columns>) {
        // Restyling re-enters (changing the colour scheme notifies every window), and most
        // settings don't touch the rows at all: only rebuild them for a change they show.
        if *self.0.style.borrow() == style && self.0.column.get() == column {
            return;
        }
        let anchor = self.anchor();
        let scale = style.fs / self.0.style.borrow().fs;
        let zoomed = (scale - 1.0).abs() > f64::EPSILON;
        self.0.style.replace(style);
        self.0.column.set(column);
        self.rebuild_bound(scale);
        if zoomed && let Some((row, offset)) = anchor {
            self.reveal(row, None, Align::RowTop(offset), false);
        }
    }

    /// Show a parsed document. With `keep_position`, the row at the top stays put (live reload).
    pub fn set_document(&self, doc: Doc, path: &Path, keep_position: bool) {
        let anchor = if keep_position { self.anchor() } else { None };
        let value = self.0.scroller.vadjustment().value();
        let gaps = gaps(&doc.blocks);
        let n = doc.blocks.len();
        self.0.path.replace(path.to_path_buf());
        self.0.spans.borrow_mut().clear();
        self.0.opened.set(Some(std::time::Instant::now()));
        self.0.near_top.set(anchor.map_or(0, |(row, _)| row.min(n.saturating_sub(1))));
        self.0.doc.replace(Some(Rc::new(DocState { doc, gaps, texts: RefCell::new(None) })));
        let items: Vec<glib::BoxedAnyObject> = (0..n).map(glib::BoxedAnyObject::new).collect();
        self.0.store.splice(0, self.0.store.n_items(), &items);
        self.0.active_heading.set(None);
        if let Some((row, offset)) = anchor {
            self.reveal(row.min(n.saturating_sub(1)), None, Align::RowTop(offset), false);
        } else if keep_position && value > 0.0 {
            // No row laid out across the top (a scroll this very frame): keep the raw offset.
            let vadj = self.0.scroller.vadjustment();
            glib::idle_add_local_once(move || vadj.set_value(value));
        } else {
            self.0.scroller.vadjustment().set_value(0.0);
        }
        let query = self.0.find.borrow().query.clone();
        if self.0.find_bar.is_visible() && !query.is_empty() {
            self.search(&query, false);
        }
        self.schedule_heading_update();
    }

    fn spans_for(&self, chunk: &CodeChunk) -> Option<Rc<Vec<Span>>> {
        let id = chunk.source.id;
        if let Some(entry) = self.0.spans.borrow().get(&id) {
            return match entry {
                Spans::Ready(spans) => Some(spans.clone()),
                Spans::Pending => None,
            };
        }
        if chunk.source.lang.is_empty() {
            return None;
        }
        self.0.spans.borrow_mut().insert(id, Spans::Pending);
        let source = chunk.source.clone();
        let rows = chunk.rows.clone();
        let weak = self.weak();
        glib::spawn_future_local(async move {
            let src = source.clone();
            let spans = gio::spawn_blocking(move || highlight::highlight(&src.lang, &src.text))
                .await
                .unwrap_or_default();
            let Some(inner) = weak.upgrade() else { return };
            let reader = Reader(inner);
            // The document may have been replaced while this ran.
            let current = reader.0.doc.borrow().as_ref().is_some_and(|d| {
                matches!(d.doc.blocks.get(rows.start), Some(crate::doc::Block::Code(c)) if c.source.id == source.id)
            });
            if !current {
                return;
            }
            reader.0.spans.borrow_mut().insert(source.id, Spans::Ready(Rc::new(spans)));
            let pending = reader.0.spans.borrow().values().any(|s| matches!(s, Spans::Pending));
            if !pending && let Some(opened) = reader.0.opened.get() {
                crate::perf::mark("code highlighted", opened);
            }
            let bound: Vec<(usize, adw::Clamp)> = reader
                .0
                .bound
                .borrow()
                .iter()
                .filter(|(i, r)| rows.contains(i) && r.built)
                .map(|(i, r)| (*i, r.clamp.clone()))
                .collect();
            for (index, clamp) in bound {
                reader.build_row(index, &clamp);
            }
        });
        None
    }

    /// A key press with no text field focused. Letters work from anywhere in the window, since
    /// nothing else in it takes them; scrolling keys only when the document has focus, as the
    /// sidebar's lists and the buttons use them too. Returns whether the key was used.
    pub fn handle_key(&self, key: gdk::Key, state: gdk::ModifierType, focus_inside: bool) -> bool {
        if !(state & (gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK)).is_empty() {
            return false;
        }
        let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
        let widget: gtk::Widget = self.0.root.clone().upcast();
        if let Some(action) = self.motion_action(key) {
            let _ = widget.activate_action(action, None);
            return true;
        }
        let vadj = self.0.scroller.vadjustment();
        let page = vadj.page_size();
        let pending_g = self.0.pending_g.replace(false);
        let delta = match key {
            gdk::Key::j => 48.0,
            gdk::Key::k => -48.0,
            gdk::Key::d => page * 0.5,
            gdk::Key::u => -page * 0.5,
            gdk::Key::G => vadj.upper(),
            gdk::Key::g if pending_g => -vadj.upper(),
            gdk::Key::g => {
                self.0.pending_g.set(true);
                return true;
            }
            gdk::Key::t if pending_g => {
                let _ = widget.activate_action("win.next-tab", None);
                return true;
            }
            gdk::Key::T if pending_g => {
                let _ = widget.activate_action("win.prev-tab", None);
                return true;
            }
            gdk::Key::Page_Down if focus_inside => page * 0.9,
            gdk::Key::Page_Up if focus_inside => -page * 0.9,
            gdk::Key::space if focus_inside && !shift => page * 0.9,
            gdk::Key::space if focus_inside => -page * 0.9,
            gdk::Key::Down | gdk::Key::KP_Down if focus_inside && !shift => 48.0,
            gdk::Key::Up | gdk::Key::KP_Up if focus_inside && !shift => -48.0,
            gdk::Key::Home if focus_inside && !shift => -vadj.upper(),
            gdk::Key::End if focus_inside && !shift => vadj.upper(),
            _ => return false,
        };
        let target = (vadj.value() + delta).clamp(vadj.lower(), vadj.upper() - page);
        self.scroll_adjustment(target, false);
        true
    }

    /// The source of the first code block at or below the top of the view, for keyboard copy:
    /// its copy button only shows under the pointer.
    pub fn code_in_view(&self) -> Option<String> {
        let top = self.row_at(HEADING_OFFSET + 2.0).map_or(0, |(row, _)| row);
        let doc = self.0.doc.borrow();
        let state = doc.as_ref()?;
        let visible =
            (self.0.scroller.vadjustment().page_size() / self.0.style.borrow().fs) as usize;
        state.doc.blocks.iter().skip(top).take(visible.max(1)).find_map(|block| match block {
            Block::Code(chunk) => Some(chunk.source.text.clone()),
            _ => None,
        })
    }

    pub fn contains_focus(&self, focus: &gtk::Widget) -> bool {
        focus.is_ancestor(&self.0.root) || *focus == *self.0.root.upcast_ref::<gtk::Widget>()
    }

    /// Single keys for a read-only document, the same motions as the terminal apps around it.
    fn motion_action(&self, key: gdk::Key) -> Option<&'static str> {
        Some(match key {
            gdk::Key::slash => "win.find",
            gdk::Key::n => "win.find-next",
            gdk::Key::N => "win.find-prev",
            gdk::Key::H => "win.go-back",
            gdk::Key::L => "win.go-forward",
            gdk::Key::bracketleft => "win.previous-heading",
            gdk::Key::bracketright => "win.next-heading",
            gdk::Key::o => "win.palette",
            gdk::Key::b => "win.toggle-sidebar",
            gdk::Key::e => "win.open-in-editor",
            gdk::Key::r => "win.reload",
            gdk::Key::q => "win.close-tab",
            gdk::Key::question => "win.shortcuts",
            gdk::Key::y => "win.copy-code",
            _ => return None,
        })
    }

    pub fn activate_link(&self, uri: &str) {
        let path = self.path();
        match files::resolve_link(uri, &path) {
            LinkTarget::Anchor(anchor) => {
                self.scroll_to_anchor(&anchor);
            }
            LinkTarget::External(url) => {
                let window = self.0.root.root().and_downcast::<gtk::Window>();
                gtk::UriLauncher::new(&url).launch(window.as_ref(), gio::Cancellable::NONE, |_| {});
            }
            LinkTarget::Document(target, anchor) => {
                if target == path {
                    if let Some(anchor) = anchor {
                        self.scroll_to_anchor(&anchor);
                    }
                } else if let Some(host) = self.host() {
                    // Ctrl+click opens beside this document, a plain click replaces it.
                    let new_tab = gtk::gdk::Display::default()
                        .and_then(|d| d.default_seat())
                        .and_then(|seat| seat.keyboard())
                        .is_some_and(|keyboard| {
                            keyboard.modifier_state().contains(gdk::ModifierType::CONTROL_MASK)
                        });
                    host.open_document(target, anchor, new_tab);
                }
            }
            LinkTarget::File(target) => {
                let window = self.0.root.root().and_downcast::<gtk::Window>();
                gtk::FileLauncher::new(Some(&gio::File::for_path(target))).launch(
                    window.as_ref(),
                    gio::Cancellable::NONE,
                    |_| {},
                );
            }
        }
    }

    pub fn scroll_to_anchor(&self, anchor: &str) -> bool {
        let row = {
            let doc = self.0.doc.borrow();
            let Some(state) = doc.as_ref() else { return false };
            if anchor == "mdow-footnotes" {
                state.doc.blocks.iter().rposition(|b| matches!(b, crate::doc::Block::Footnotes(_)))
            } else {
                state
                    .doc
                    .anchors
                    .get(anchor)
                    .or_else(|| state.doc.anchors.get(&crate::doc::slugify(anchor)))
                    .copied()
            }
        };
        match row {
            Some(row) => {
                self.reveal(row, None, Align::Top(HEADING_OFFSET), true);
                true
            }
            None => false,
        }
    }

    pub fn with_document<T>(&self, f: impl FnOnce(&Doc) -> T) -> Option<T> {
        self.0.doc.borrow().as_ref().map(|state| f(&state.doc))
    }

    /// The row at the top of the view and its offset, to come back to later.
    pub fn position(&self) -> Option<(usize, f64)> {
        self.anchor()
    }

    pub fn restore_position(&self, row: usize, offset: f64) {
        let last = self.0.store.n_items().saturating_sub(1) as usize;
        self.reveal(row.min(last), None, Align::RowTop(offset), false);
    }

    /// Go to the next or previous heading from where the view is; before the first, to the top.
    pub fn jump_heading(&self, step: i32) {
        let rows: Vec<usize> = self
            .0
            .doc
            .borrow()
            .as_ref()
            .map(|d| d.doc.headings.iter().map(|h| h.row).collect())
            .unwrap_or_default();
        // A heading we jumped to sits just below the top edge, so look there.
        let Some((top, _)) = self.row_at(HEADING_OFFSET + 2.0) else { return };
        let target = if step > 0 {
            rows.iter().position(|row| *row > top)
        } else {
            rows.iter().rposition(|row| *row < top)
        };
        match target {
            Some(index) => self.reveal(rows[index], None, Align::Top(HEADING_OFFSET), false),
            None if step < 0 => {
                let vadj = self.0.scroller.vadjustment();
                self.scroll_adjustment(vadj.lower(), false);
            }
            None => {}
        }
    }

    pub fn scroll_to_heading(&self, index: usize) {
        let row =
            self.0.doc.borrow().as_ref().and_then(|d| d.doc.headings.get(index).map(|h| h.row));
        if let Some(row) = row {
            self.reveal(row, None, Align::Top(HEADING_OFFSET), true);
        }
    }

    /// The row under viewport height `y`, and where that row's top edge is. Hit-testing sees
    /// only what is laid out on screen; bound rows off screen keep stale positions.
    fn row_at(&self, y: f64) -> Option<(usize, f64)> {
        let list = &self.0.list;
        let x = (list.width() as f64 / 2.0).max(1.0);
        let mut widget =
            list.pick(x, y, gtk::PickFlags::INSENSITIVE | gtk::PickFlags::NON_TARGETABLE)?;
        loop {
            if let Some(clamp) = widget.downcast_ref::<adw::Clamp>() {
                let bound = self.0.bound.borrow();
                let index = bound.iter().find(|(_, row)| row.clamp == *clamp).map(|(i, _)| *i)?;
                let top = clamp.compute_point(list, &graphene::Point::new(0.0, 0.0))?.y() as f64;
                return Some((index, top));
            }
            if widget == *list.upcast_ref::<gtk::Widget>() {
                return None;
            }
            widget = widget.parent()?;
        }
    }

    /// The top row on screen and how far below the viewport top it starts.
    fn anchor(&self) -> Option<(usize, f64)> {
        self.row_at(0.5)
    }

    /// Bring `row` (optionally a byte offset in one of its labels) into view, then align it.
    fn reveal(&self, row: usize, label: Option<(usize, u32)>, align: Align, smooth: bool) {
        let generation = self.0.reveal_generation.get() + 1;
        self.0.reveal_generation.set(generation);
        let already = self.0.bound.borrow().contains_key(&row);
        let from = self.anchor().map_or(row, |(top, _)| top);
        self.0.near_top.set(row);
        if !already {
            self.0.list.scroll_to(row as u32, gtk::ListScrollFlags::NONE, None);
        }
        let smooth = smooth && already && adw::is_animations_enabled(&self.0.root);
        if smooth {
            // An animated scroll aims once, so the rows it crosses need their real heights.
            let span = from.min(row)..=from.max(row) + EAGER_BELOW;
            for (index, clamp) in self.unbuilt() {
                if span.contains(&index) {
                    self.build_row(index, &clamp);
                }
            }
        }
        let weak = self.weak();
        let tries = Cell::new(0u32);
        self.0.list.add_tick_callback(move |_, _| {
            let Some(inner) = weak.upgrade() else { return glib::ControlFlow::Break };
            let reader = Reader(inner);
            if reader.0.reveal_generation.get() != generation {
                return glib::ControlFlow::Break;
            }
            tries.set(tries.get() + 1);
            let tries = tries.get();
            match reader.target_y(row, label, !matches!(align, Align::RowTop(_))) {
                Some(y) => {
                    let vadj = reader.0.scroller.vadjustment();
                    let page = vadj.page_size();
                    let want = match align {
                        Align::Top(offset) | Align::RowTop(offset) => Some(y - offset),
                        Align::Match => {
                            let visible = y > 48.0 && y < page - 48.0;
                            (!visible).then(|| y - page / 3.0)
                        }
                    };
                    if let Some(delta) = want {
                        let target =
                            (vadj.value() + delta).clamp(vadj.lower(), vadj.upper() - page);
                        reader.scroll_adjustment(target, smooth && delta.abs() < page * 3.0);
                        // Rows near the target may resize once realised; settle again next frame.
                        if tries < 6 && !smooth && delta.abs() > 1.0 {
                            return glib::ControlFlow::Continue;
                        }
                    }
                    glib::ControlFlow::Break
                }
                None if tries < 12 => {
                    reader.0.list.scroll_to(row as u32, gtk::ListScrollFlags::NONE, None);
                    glib::ControlFlow::Continue
                }
                None => glib::ControlFlow::Break,
            }
        });
    }

    fn target_y(&self, row: usize, label: Option<(usize, u32)>, content: bool) -> Option<f64> {
        let blank = {
            let bound = self.0.bound.borrow();
            let entry = bound.get(&row)?;
            (!entry.built).then(|| entry.clamp.clone())
        };
        if let Some(clamp) = blank {
            // Its height and labels aren't real yet: build it and look again next frame.
            self.build_row(row, &clamp);
            return None;
        }
        let bound = self.0.bound.borrow();
        let entry = bound.get(&row)?;
        if entry.clamp.height() == 0 {
            return None;
        }
        if let Some((label_index, byte)) = label
            && let Some(label) = entry.labels.get(label_index)
        {
            let layout = label.layout();
            let rect = layout.index_to_pos(byte as i32);
            let (_, oy) = label.layout_offsets();
            let y = oy as f64 + rect.y() as f64 / gtk::pango::SCALE as f64;
            let point = label.compute_point(&self.0.list, &graphene::Point::new(0.0, y as f32))?;
            return Some(point.y() as f64);
        }
        let point = entry.clamp.compute_point(&self.0.list, &graphene::Point::new(0.0, 0.0))?;
        if !content {
            return Some(point.y() as f64);
        }
        // Skip the row's own top gap so headings land at the offset, not their margin.
        let child_top = entry
            .clamp
            .child()
            .and_then(|w| w.first_child())
            .map(|w| w.margin_top() as f64)
            .unwrap_or(0.0);
        Some(point.y() as f64 + child_top)
    }

    fn scroll_adjustment(&self, target: f64, smooth: bool) {
        let vadj = self.0.scroller.vadjustment();
        if let Some(animation) = self.0.animation.take() {
            animation.pause();
        }
        if !smooth {
            vadj.set_value(target);
            return;
        }
        let weak = vadj.downgrade();
        let animation = adw::TimedAnimation::new(
            &self.0.scroller,
            vadj.value(),
            target,
            320,
            adw::CallbackAnimationTarget::new(move |value| {
                if let Some(vadj) = weak.upgrade() {
                    vadj.set_value(value);
                }
            }),
        );
        animation.set_easing(adw::Easing::EaseOutCubic);
        animation.play();
        self.0.animation.replace(Some(animation));
    }

    fn connect_scrolling(&self) {
        let weak = self.weak();
        self.0.scroller.vadjustment().connect_value_changed(move |_| {
            if let Some(inner) = weak.upgrade() {
                let reader = Reader(inner);
                if let Some((row, _)) = reader.anchor() {
                    reader.0.near_top.set(row);
                }
                reader.schedule_heading_update();
            }
        });
        let weak = self.weak();
        self.0.scroller.vadjustment().connect_changed(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).schedule_heading_update();
            }
        });

        // Ctrl + wheel / pinch zoom.
        let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        wheel.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        wheel.connect_scroll(move |controller, _, dy| {
            let ctrl = controller.current_event_state().contains(gdk::ModifierType::CONTROL_MASK);
            if !ctrl {
                return glib::Propagation::Proceed;
            }
            let Some(inner) = weak.upgrade() else { return glib::Propagation::Stop };
            let reader = Reader(inner);
            let acc = reader.0.zoom_accum.get() + dy;
            if acc.abs() >= 1.0 {
                reader.0.zoom_accum.set(0.0);
                if let Some(host) = reader.host() {
                    host.zoom(if acc < 0.0 { 1 } else { -1 });
                }
            } else {
                reader.0.zoom_accum.set(acc);
            }
            glib::Propagation::Stop
        });
        self.0.scroller.add_controller(wheel);

        let pinch = gtk::GestureZoom::new();
        let weak = self.weak();
        let last_step = Rc::new(Cell::new(0i32));
        let begin_step = last_step.clone();
        pinch.connect_begin(move |_, _| begin_step.set(0));
        pinch.connect_scale_changed(move |_, scale| {
            let step = (scale.ln() / 0.1).trunc() as i32;
            if step != last_step.get()
                && let Some(inner) = weak.upgrade()
                && let Some(host) = Reader(inner).host()
            {
                host.zoom((step - last_step.get()).signum());
                last_step.set(step);
            }
        });
        self.0.scroller.add_controller(pinch);
    }

    fn schedule_heading_update(&self) {
        if self.0.heading_idle.replace(true) {
            return;
        }
        let weak = self.weak();
        glib::idle_add_local_once(move || {
            let Some(inner) = weak.upgrade() else { return };
            inner.heading_idle.set(false);
            Reader(inner).update_heading();
        });
    }

    fn update_heading(&self) {
        let Some(state) = self.0.doc.borrow().clone() else { return };
        let headings = &state.doc.headings;
        if headings.is_empty() {
            return;
        }
        let vadj = self.0.scroller.vadjustment();
        let page = vadj.page_size();
        let threshold = (page * 0.2).max(64.0).min(page - 1.0).max(0.5);
        let at_end = vadj.upper() > page && vadj.value() + page >= vadj.upper() - 2.0;
        // At the very bottom, the last heading on screen wins.
        let probe = if at_end { page - 1.0 } else { threshold };
        let Some((row, _)) = self.row_at(probe).or_else(|| self.row_at(0.5)) else {
            // Not laid out yet (a jump just happened): look again next frame.
            let weak = self.weak();
            self.0.list.add_tick_callback(move |_, _| {
                if let Some(inner) = weak.upgrade() {
                    Reader(inner).schedule_heading_update();
                }
                glib::ControlFlow::Break
            });
            return;
        };
        let index = headings.partition_point(|h| h.row <= row).checked_sub(1).or(Some(0));
        if index != self.0.active_heading.get() {
            self.0.active_heading.set(index);
            if let Some(host) = self.host() {
                host.heading_changed(self, index);
            }
        }
    }

    // ── Find ─────────────────────────────────────────────────────────────────

    fn connect_find(&self, close: &gtk::Button) {
        let weak = self.weak();
        self.0.find_entry.connect_search_changed(move |entry| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).search(&entry.text(), true);
            }
        });
        let weak = self.weak();
        self.0.find_entry.connect_activate(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).find_step(1);
            }
        });
        let weak = self.weak();
        self.0.find_entry.connect_next_match(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).find_step(1);
            }
        });
        let weak = self.weak();
        self.0.find_entry.connect_previous_match(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).find_step(-1);
            }
        });
        let weak = self.weak();
        self.0.find_entry.connect_stop_search(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).close_find();
            }
        });
        let shift_enter = gtk::EventControllerKey::new();
        let weak = self.weak();
        shift_enter.connect_key_pressed(move |_, key, _, state| {
            if matches!(key, gdk::Key::Return | gdk::Key::KP_Enter)
                && state.contains(gdk::ModifierType::SHIFT_MASK)
                && let Some(inner) = weak.upgrade()
            {
                Reader(inner).find_step(-1);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.0.find_entry.add_controller(shift_enter);
        let weak = self.weak();
        self.0.find_prev.connect_clicked(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).find_step(-1);
            }
        });
        let weak = self.weak();
        self.0.find_next.connect_clicked(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).find_step(1);
            }
        });
        let weak = self.weak();
        close.connect_clicked(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).close_find();
            }
        });
        self.update_find_count();
    }

    pub fn open_find(&self) {
        self.0.find_bar.set_visible(true);
        self.0.find_entry.grab_focus();
        self.0.find_entry.select_region(0, -1);
    }

    pub fn find_is_open(&self) -> bool {
        self.0.find_bar.is_visible()
    }

    pub fn close_find(&self) {
        if !self.0.find_bar.is_visible() {
            return;
        }
        self.0.find_bar.set_visible(false);
        self.0.find_entry.set_text("");
        self.search("", false);
        self.0.scroller.grab_focus();
    }

    fn texts(&self, state: &DocState) -> Rc<Vec<Vec<String>>> {
        if let Some(texts) = state.texts.borrow().clone() {
            return texts;
        }
        let style = self.0.style.borrow().clone();
        let texts: Vec<Vec<String>> = state
            .doc
            .blocks
            .iter()
            .map(|block| {
                let mut out = Vec::new();
                render::label_texts(block, &style, &mut out);
                out
            })
            .collect();
        let texts = Rc::new(texts);
        state.texts.replace(Some(texts.clone()));
        texts
    }

    fn search(&self, query: &str, reveal: bool) {
        let state = self.0.doc.borrow().clone();
        let mut matches = Vec::new();
        if let Some(state) = state.as_ref()
            && !query.is_empty()
        {
            let texts = self.texts(state);
            for (row, labels) in texts.iter().enumerate() {
                for (label, text) in labels.iter().enumerate() {
                    for (start, end) in render::find_all(text, query) {
                        matches.push(Match { row, label, start: start as u32, end: end as u32 });
                    }
                }
            }
        }
        let top = self.anchor().map(|a| a.0).unwrap_or(0);
        let current =
            (!matches.is_empty()).then(|| matches.iter().position(|m| m.row >= top).unwrap_or(0));
        {
            let mut find = self.0.find.borrow_mut();
            find.query = query.to_string();
            find.matches = matches;
            find.current = current;
        }
        self.apply_find_all();
        self.update_find_count();
        if reveal {
            self.reveal_current();
        }
    }

    pub fn find_step(&self, step: i32) {
        if !self.0.find_bar.is_visible() {
            self.open_find();
            return;
        }
        let (old, new) = {
            let mut find = self.0.find.borrow_mut();
            let n = find.matches.len();
            if n == 0 {
                return;
            }
            let old = find.current;
            let next = match old {
                Some(i) => (i as i64 + step as i64).rem_euclid(n as i64) as usize,
                None => 0,
            };
            find.current = Some(next);
            (old.map(|i| find.matches[i].row), find.matches[next].row)
        };
        if let Some(old) = old {
            self.apply_find_row(old);
        }
        self.apply_find_row(new);
        self.update_find_count();
        self.reveal_current();
    }

    fn reveal_current(&self) {
        let target = {
            let find = self.0.find.borrow();
            find.current.and_then(|i| find.matches.get(i).copied())
        };
        if let Some(m) = target {
            self.reveal(m.row, Some((m.label, m.start)), Align::Match, false);
        }
    }

    fn update_find_count(&self) {
        let find = self.0.find.borrow();
        let text = if find.query.is_empty() {
            String::new()
        } else if find.matches.is_empty() {
            "No results".to_string()
        } else {
            format!("{} of {}", find.current.map_or(0, |c| c + 1), find.matches.len())
        };
        self.0.find_count.set_text(&text);
        let any = !find.matches.is_empty();
        self.0.find_prev.set_sensitive(any);
        self.0.find_next.set_sensitive(any);
    }

    fn apply_find_all(&self) {
        let rows: Vec<usize> = self.0.bound.borrow().keys().copied().collect();
        for row in rows {
            self.apply_find_row(row);
        }
    }

    fn apply_find_row(&self, row: usize) {
        let find = self.0.find.borrow();
        let bound = self.0.bound.borrow();
        let Some(entry) = bound.get(&row) else { return };
        let palette = self.0.style.borrow().palette.clone();
        let first = find.matches.partition_point(|m| m.row < row);
        let mut per_label: HashMap<usize, Vec<(u32, u32, bool)>> = HashMap::new();
        for (i, m) in find.matches.iter().enumerate().skip(first) {
            if m.row != row {
                break;
            }
            per_label.entry(m.label).or_default().push((m.start, m.end, find.current == Some(i)));
        }
        for (index, label) in entry.labels.iter().enumerate() {
            match per_label.get(&index) {
                Some(ranges) => label.set_attributes(Some(&render::match_attrs(ranges, &palette))),
                None => {
                    if label.attributes().is_some() {
                        label.set_attributes(None);
                    }
                }
            }
        }
    }

    // ── Zoom indicator ───────────────────────────────────────────────────────

    fn connect_zoom(&self, zoom_out: &gtk::Button, zoom_in: &gtk::Button) {
        for (button, step) in [(zoom_out, -1), (zoom_in, 1), (&self.0.zoom_reset, 0)] {
            let weak = self.weak();
            button.connect_clicked(move |_| {
                if let Some(inner) = weak.upgrade()
                    && let Some(host) = Reader(inner).host()
                {
                    host.zoom(step);
                }
            });
        }
        let motion = gtk::EventControllerMotion::new();
        let weak = self.weak();
        motion.connect_enter(move |_, _, _| {
            if let Some(inner) = weak.upgrade() {
                inner.zoom_hover.set(true);
                if let Some(id) = inner.zoom_timer.take() {
                    id.remove();
                }
                inner.zoom_hud.remove_css_class("mdow-hidden");
            }
        });
        let weak = self.weak();
        motion.connect_leave(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.zoom_hover.set(false);
                Reader(inner).schedule_zoom_hide(1000);
            }
        });
        self.0.zoom_hud.add_controller(motion);
    }

    /// Reflect a zoom level; `announce` shows the indicator briefly even at 100%.
    pub fn set_zoom(&self, level: u32, announce: bool) {
        let changed = self.0.zoom_level.replace(level) != level;
        self.0.zoom_label.set_text(&format!("{level}%"));
        self.0.zoom_reset.set_opacity(if level == 100 { 0.0 } else { 1.0 });
        self.0.zoom_reset.set_sensitive(level != 100);
        if level != 100 {
            self.0.zoom_hud.set_visible(true);
            self.0.zoom_hud.remove_css_class("mdow-hidden");
            if let Some(id) = self.0.zoom_timer.take() {
                id.remove();
            }
        } else if changed && announce {
            self.0.zoom_hud.set_visible(true);
            self.0.zoom_hud.remove_css_class("mdow-hidden");
            self.schedule_zoom_hide(2000);
        } else if !announce {
            self.0.zoom_hud.set_visible(false);
        }
    }

    fn schedule_zoom_hide(&self, ms: u64) {
        if self.0.zoom_level.get() != 100 || self.0.zoom_hover.get() {
            return;
        }
        if let Some(id) = self.0.zoom_timer.take() {
            id.remove();
        }
        let weak = self.weak();
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(ms), move || {
            let Some(inner) = weak.upgrade() else { return };
            inner.zoom_timer.take();
            inner.zoom_hud.add_css_class("mdow-hidden");
            let weak = Rc::downgrade(&inner);
            glib::timeout_add_local_once(std::time::Duration::from_millis(160), move || {
                if let Some(inner) = weak.upgrade()
                    && inner.zoom_hud.has_css_class("mdow-hidden")
                {
                    inner.zoom_hud.set_visible(false);
                }
            });
        });
        self.0.zoom_timer.replace(Some(id));
    }

    pub fn focus(&self) {
        self.0.scroller.grab_focus();
    }

    /// Debug aid for screenshots: open find with a query.
    pub fn debug_find(&self, query: &str) {
        self.open_find();
        self.0.find_entry.set_text(query);
        self.search(query, true);
    }

    /// Benchmark: scroll down the document at a steady flick for `seconds` (turning round at the
    /// ends), then log how the frames went.
    pub fn bench_scroll(&self, seconds: f64) {
        const SPEED: f64 = 6000.0;
        let weak = self.weak();
        let began = std::time::Instant::now();
        let last = Cell::new(began);
        let direction = Cell::new(1.0);
        crate::perf::frames_begin();
        self.0.list.add_tick_callback(move |_, _| {
            let Some(inner) = weak.upgrade() else { return glib::ControlFlow::Break };
            let now = std::time::Instant::now();
            if (now - began).as_secs_f64() > seconds {
                crate::perf::frames_report("scroll");
                return glib::ControlFlow::Break;
            }
            let vadj = inner.scroller.vadjustment();
            let end = (vadj.upper() - vadj.page_size()).max(0.0);
            let step = SPEED * (now - last.replace(now)).as_secs_f64() * direction.get();
            let value = vadj.value() + step;
            if value >= end || value <= 0.0 {
                direction.set(-direction.get());
            }
            vadj.set_value(value.clamp(0.0, end));
            glib::ControlFlow::Continue
        });
    }

    /// Debug aid for screenshots: scroll so `row` sits at the top.
    pub fn scroll_to_row(&self, row: usize) {
        let last = self.0.store.n_items().saturating_sub(1) as usize;
        self.reveal(row.min(last), None, Align::Top(0.0), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::prelude::FontMapExt;

    /// Real prose, set at the computed width, lands near the measure it was asked for.
    #[test]
    fn text_column_holds_about_the_requested_characters_per_line() {
        crate::fonts::register();
        let context = pangocairo::FontMap::new().create_context();
        let family = crate::dialogs::content_pango_family(crate::prefs::ContentFont::Charter);
        let width = text_column(&context, family, 16.0, 70.0) - 2 * PAD_X;
        let text = "Atlas keeps every device in step with a local-first sync engine. Edits land \
            in an on-device log first, then replicate to peers in the background, so the app stays \
            instant offline and converges the moment a connection returns. Every mutation is an \
            operation with a timestamp, and merging is deterministic, so any two replicas that have \
            seen the same set of operations end up identical.";
        let layout = pango::Layout::new(&context);
        let mut font = pango::FontDescription::from_string(family);
        font.set_absolute_size(16.0 * pango::SCALE as f64);
        layout.set_font_description(Some(&font));
        layout.set_width(width * pango::SCALE);
        layout.set_wrap(pango::WrapMode::Word);
        layout.set_text(text);
        let full_lines = (layout.line_count() - 1) as f64;
        let last = layout.line_readonly(layout.line_count() - 1).map_or(0, |l| l.length()) as f64;
        let per_line = (text.len() as f64 - last) / full_lines;
        assert!((60.0..=80.0).contains(&per_line), "{per_line:.0} characters per line");
    }
}
