//! The document reader: a virtualized `GtkListView` with one row per top-level block, so even
//! huge documents open instantly and scroll smoothly. Floating on top: the find bar (top right)
//! and the zoom indicator (bottom right), as on desktop.

use crate::doc::{CodeChunk, Doc, Heading, gaps};
use crate::files::{self, LinkTarget};
use crate::highlight::{self, Span};
use crate::render::{self, Builder, Host, Style};
use adw::prelude::*;
use gtk::{gdk, gio, glib, graphene};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

/// What a reader asks of the window that owns it.
pub trait ReaderHost {
    fn open_document(&self, path: PathBuf, anchor: Option<String>);
    fn heading_changed(&self, reader: &Reader, index: Option<usize>);
    fn zoom(&self, step: i32);
}

/// Reading-frame padding, as the desktop's `px-12 py-8`.
const PAD_X: i32 = 48;
const PAD_Y: f64 = 32.0;
/// Where an anchored heading lands (the desktop's `scroll-margin-top: 1.5rem`).
const HEADING_OFFSET: f64 = 24.0;

struct BoundRow {
    clamp: adw::Clamp,
    labels: Vec<gtk::Label>,
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
    column: Cell<Option<i32>>,
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
    button.add_css_class("flat");
    button.set_tooltip_text(Some(tooltip));
    button.set_valign(gtk::Align::Center);
    button
}

impl Reader {
    pub fn new(style: Rc<Style>, column: Option<i32>) -> Reader {
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
        });
        let reader = Reader(inner);
        reader.connect_factory(&factory);
        reader.connect_find(&find_close);
        reader.connect_zoom(&zoom_out, &zoom_in);
        reader.connect_scrolling();
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

    pub fn title(&self) -> Option<String> {
        self.0.doc.borrow().as_ref().and_then(|d| d.doc.title.clone())
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
                Reader(inner).configure_clamp(&clamp);
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

    fn configure_clamp(&self, clamp: &adw::Clamp) {
        match self.0.column.get() {
            Some(width) => {
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
        self.configure_clamp(clamp);
        clamp.set_child(Some(&wrapper));
        self.0
            .bound
            .borrow_mut()
            .insert(index, BoundRow { clamp: clamp.clone(), labels: builder.labels });
        self.apply_find_row(index);
    }

    /// Rebuild the rows on screen (theme, fonts, zoom or width changed); the rest rebuild as
    /// they scroll into view.
    fn rebuild_bound(&self) {
        let rows: Vec<(usize, adw::Clamp)> =
            self.0.bound.borrow().iter().map(|(i, r)| (*i, r.clamp.clone())).collect();
        for (index, clamp) in rows {
            self.bind_row(index, &clamp);
        }
    }

    pub fn set_style(&self, style: Rc<Style>, column: Option<i32>) {
        let anchor = self.anchor();
        let zoomed = (self.0.style.borrow().fs - style.fs).abs() > f64::EPSILON;
        self.0.style.replace(style);
        self.0.column.set(column);
        self.rebuild_bound();
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
            let bound: Vec<(usize, adw::Clamp)> = reader
                .0
                .bound
                .borrow()
                .iter()
                .filter(|(i, _)| rows.contains(i))
                .map(|(i, r)| (*i, r.clamp.clone()))
                .collect();
            for (index, clamp) in bound {
                reader.bind_row(index, &clamp);
            }
        });
        None
    }

    fn activate_link(&self, uri: &str) {
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
                    host.open_document(target, anchor);
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
        if !already {
            self.0.list.scroll_to(row as u32, gtk::ListScrollFlags::NONE, None);
        }
        let smooth = smooth && already && adw::is_animations_enabled(&self.0.root);
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
                Reader(inner).schedule_heading_update();
            }
        });
        let weak = self.weak();
        self.0.scroller.vadjustment().connect_changed(move |_| {
            if let Some(inner) = weak.upgrade() {
                Reader(inner).schedule_heading_update();
            }
        });

        // Keyboard scrolling: the list's own key handling moves between items, not pages.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        keys.connect_key_pressed(move |controller, key, _, state| {
            let Some(inner) = weak.upgrade() else { return glib::Propagation::Proceed };
            let reader = Reader(inner);
            let focus = controller.widget().and_then(|w| w.root()).and_then(|r| r.focus());
            if focus.is_some_and(|f| f.is::<gtk::Text>() || f.is::<gtk::Editable>()) {
                return glib::Propagation::Proceed;
            }
            let mods = state & (gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK);
            if !mods.is_empty() {
                return glib::Propagation::Proceed;
            }
            let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
            let vadj = reader.0.scroller.vadjustment();
            let page = vadj.page_size();
            let delta = match key {
                gdk::Key::Page_Down => page * 0.9,
                gdk::Key::Page_Up => -page * 0.9,
                gdk::Key::space if !shift => page * 0.9,
                gdk::Key::space => -page * 0.9,
                gdk::Key::Down | gdk::Key::KP_Down if !shift => 48.0,
                gdk::Key::Up | gdk::Key::KP_Up if !shift => -48.0,
                gdk::Key::Home if !shift => -vadj.upper(),
                gdk::Key::End if !shift => vadj.upper(),
                _ => return glib::Propagation::Proceed,
            };
            let target = (vadj.value() + delta).clamp(vadj.lower(), vadj.upper() - page);
            let big = delta.abs() >= page * 0.5;
            reader.scroll_adjustment(target, big && delta.abs() < page * 3.0);
            glib::Propagation::Stop
        });
        self.0.scroller.add_controller(keys);

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

    /// Debug aid for screenshots: scroll so `row` sits at the top.
    pub fn scroll_to_row(&self, row: usize) {
        self.reveal(row, None, Align::Top(0.0), false);
    }
}
