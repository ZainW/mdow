//! The companion panel: setup states, the conversation and the composer.

use super::logic::{self, DiffKind, ReplyBlock, ReplyLine};
use super::service::{DocumentContext, Runtime, Session};
use super::types::*;
use crate::doc::{self, Block};
use crate::files;
use crate::render::{Style, markup_of};
use gtk::glib::markup_escape_text;
use gtk::{gdk, gio, glib, pango, prelude::*};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

const INSTALL_COMMAND: &str = "curl -fsSL https://opencode.ai/install | bash";
const DOCS_URL: &str = "https://opencode.ai/docs";
/// Long diffs collapse past this many lines so the conversation stays readable.
const DIFF_PREVIEW_LINES: usize = 14;
const MENTION_LIMIT: usize = 8;
/// Streaming text arrives faster than it's worth drawing.
const RENDER_DELAY: Duration = Duration::from_millis(33);

fn label(text: &str, classes: &[&str]) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    for class in classes {
        label.add_css_class(class);
    }
    label
}

fn wrapped(text: &str, classes: &[&str]) -> gtk::Label {
    let label = label(text, classes);
    label.set_wrap(true);
    label.set_wrap_mode(pango::WrapMode::WordChar);
    label.set_selectable(true);
    label
}

fn ellipsized(text: &str, mode: pango::EllipsizeMode, classes: &[&str]) -> gtk::Label {
    let label = label(text, classes);
    label.set_ellipsize(mode);
    label.set_tooltip_text(Some(text));
    label
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    crate::set_accessible_name(&button, tooltip);
    button.add_css_class("flat");
    button.add_css_class("mdow-icon-button");
    button.set_tooltip_text(Some(tooltip));
    button.set_valign(gtk::Align::Center);
    button
}

fn pill_button(text: &str, kind: &str) -> gtk::Button {
    let button = gtk::Button::with_label(text);
    button.add_css_class("mdow-pill");
    button.add_css_class(kind);
    button.set_valign(gtk::Align::Center);
    button
}

fn hairline() -> gtk::Separator {
    let line = gtk::Separator::new(gtk::Orientation::Horizontal);
    line.add_css_class("mdow-companion-hairline");
    line
}

fn clear(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

/// Inline markdown as Pango markup, through the same parser as the reader.
fn inline_markup(text: &str, style: &Style) -> String {
    let parsed = doc::parse_markdown(&text.replace('\n', "  \n"), Path::new("/reply.md"));
    match parsed.blocks.as_slice() {
        [Block::Paragraph(inlines)] => markup_of(inlines, style).markup,
        _ => markup_escape_text(text).to_string(),
    }
}

fn reply_label(text: &str, style: &Style, classes: &[&str]) -> gtk::Label {
    let label = wrapped("", classes);
    label.set_markup(&inline_markup(text, style));
    label
}

fn reply_block(block: &ReplyBlock, style: &Style) -> gtk::Widget {
    match block {
        ReplyBlock::Paragraph(text) => {
            let lines = gtk::Box::new(gtk::Orientation::Vertical, 5);
            for line in logic::split_reply_lines(text) {
                match line {
                    ReplyLine::Text(text) => {
                        lines.append(&reply_label(&text, style, &["mdow-reply"]));
                    }
                    ReplyLine::Item { marker, depth, text } => {
                        let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                        row.set_margin_start(depth as i32 * 16);
                        let marker = label(&marker, &["mdow-reply", "mdow-companion-secondary"]);
                        marker.set_size_request(18, -1);
                        marker.set_valign(gtk::Align::Start);
                        row.append(&marker);
                        let body = reply_label(&text, style, &["mdow-reply"]);
                        body.set_hexpand(true);
                        row.append(&body);
                        lines.append(&row);
                    }
                }
            }
            lines.upcast()
        }
        ReplyBlock::Heading { level, text } => {
            let class = if *level <= 2 { "mdow-reply-title" } else { "mdow-reply-heading" };
            reply_label(text, style, &[class]).upcast()
        }
        ReplyBlock::Code { text, .. } => {
            let code = label(text, &["mdow-companion-mono"]);
            code.set_selectable(true);
            code.set_margin_start(11);
            code.set_margin_end(11);
            code.set_margin_top(9);
            code.set_margin_bottom(9);
            let scroller = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Automatic)
                .vscrollbar_policy(gtk::PolicyType::Never)
                .propagate_natural_height(true)
                .child(&code)
                .build();
            scroller.add_css_class("mdow-companion-card");
            scroller.upcast()
        }
    }
}

struct PartView {
    part: Part,
    active: bool,
    widget: gtk::Widget,
    /// For text parts: each block's source and widget, so only what changed is rebuilt.
    blocks: Vec<(ReplyBlock, gtk::Widget)>,
}

struct MessageView {
    id: String,
    parts_box: gtk::Box,
    footer: gtk::Box,
    footer_key: RefCell<String>,
    parts: RefCell<Vec<PartView>>,
    root: gtk::Box,
}

type OpenDocument = Rc<dyn Fn(&Path)>;

pub struct Panel {
    pub root: gtk::Box,
    session: Rc<Session>,
    runtime: Rc<Runtime>,
    style: RefCell<Rc<Style>>,
    open_document: RefCell<Option<OpenDocument>>,
    stack: gtk::Stack,
    setup: gtk::Box,
    setup_key: RefCell<String>,
    version: gtk::Label,
    new_chat: gtk::Button,
    messages_stack: gtk::Stack,
    empty: gtk::Box,
    empty_key: RefCell<Option<String>>,
    list: gtk::Box,
    scroller: gtk::ScrolledWindow,
    stick_to_bottom: Cell<bool>,
    views: RefCell<Vec<MessageView>>,
    tail: gtk::Box,
    tail_key: RefCell<String>,
    review: gtk::Box,
    review_key: RefCell<String>,
    mentions: gtk::ListBox,
    mention_results: RefCell<Vec<PathBuf>>,
    mention_index: Cell<usize>,
    mention_open: Cell<bool>,
    chips: gtk::Box,
    chips_key: RefCell<String>,
    input: gtk::TextView,
    placeholder: gtk::Label,
    at_button: gtk::Button,
    model_button: gtk::MenuButton,
    model_label: gtk::Label,
    send: gtk::Button,
    stop: gtk::Button,
    expanded: RefCell<HashMap<String, bool>>,
    document: RefCell<Option<PathBuf>>,
    folder: RefCell<Option<PathBuf>>,
    documents: RefCell<Vec<PathBuf>>,
    visible: Cell<bool>,
    render_pending: Cell<bool>,
    closed: Cell<bool>,
}

impl Panel {
    pub fn new(style: Rc<Style>) -> Rc<Panel> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("mdow-companion");
        root.set_size_request(300, -1);
        root.set_visible(false);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        header.add_css_class("mdow-companion-header");
        header.append(&label("Companion", &["mdow-companion-title"]));
        let version = ellipsized(
            "",
            pango::EllipsizeMode::End,
            &["mdow-companion-tertiary", "mdow-companion-small-mono"],
        );
        version.set_hexpand(true);
        header.append(&version);
        let new_chat = icon_button("document-edit-symbolic", "New Chat");
        header.append(&new_chat);
        root.append(&header);

        let setup = gtk::Box::new(gtk::Orientation::Vertical, 10);
        setup.add_css_class("mdow-companion-setup");
        let setup_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&setup)
            .vexpand(true)
            .build();

        let list = gtk::Box::new(gtk::Orientation::Vertical, 18);
        list.add_css_class("mdow-companion-list");
        let tail = gtk::Box::new(gtk::Orientation::Vertical, 8);
        list.append(&tail);
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .vexpand(true)
            .build();
        let empty = gtk::Box::new(gtk::Orientation::Vertical, 0);
        empty.add_css_class("mdow-companion-empty");
        empty.set_valign(gtk::Align::Center);
        let empty_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&empty)
            .vexpand(true)
            .build();
        let messages_stack = gtk::Stack::new();
        messages_stack.add_named(&empty_scroll, Some("empty"));
        messages_stack.add_named(&scroller, Some("list"));

        let review = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        review.add_css_class("mdow-companion-review");
        review.set_visible(false);

        let mentions = gtk::ListBox::new();
        mentions.add_css_class("mdow-companion-mentions");
        mentions.set_selection_mode(gtk::SelectionMode::Single);
        mentions.set_visible(false);

        let chips = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        chips.add_css_class("mdow-companion-chips");
        let input = gtk::TextView::new();
        input.add_css_class("mdow-companion-input");
        input.set_wrap_mode(gtk::WrapMode::WordChar);
        input.set_accepts_tab(false);
        let input_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(160)
            .child(&input)
            .build();
        let placeholder = label("", &["mdow-companion-tertiary"]);
        placeholder.set_can_target(false);
        placeholder.set_valign(gtk::Align::Start);
        placeholder.set_ellipsize(pango::EllipsizeMode::End);
        let input_overlay = gtk::Overlay::new();
        input_overlay.set_child(Some(&input_scroll));
        input_overlay.add_overlay(&placeholder);

        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        let at_button = gtk::Button::with_label("@");
        at_button.add_css_class("flat");
        at_button.add_css_class("mdow-icon-button");
        at_button.set_tooltip_text(Some("Mention a Document"));
        bar.append(&at_button);
        let model_label = ellipsized("", pango::EllipsizeMode::End, &["mdow-companion-small-mono"]);
        model_label.set_max_width_chars(22);
        let model_button = gtk::MenuButton::new();
        model_button.set_child(Some(&model_label));
        model_button.add_css_class("flat");
        model_button.add_css_class("mdow-companion-model");
        model_button.set_valign(gtk::Align::Center);
        bar.append(&model_button);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        bar.append(&spacer);
        let send = gtk::Button::from_icon_name("go-up-symbolic");
        crate::set_accessible_name(&send, "Send");
        send.add_css_class("mdow-companion-send");
        send.set_tooltip_text(Some("Send (Enter)"));
        send.set_sensitive(false);
        bar.append(&send);
        let stop = gtk::Button::from_icon_name("media-playback-stop-symbolic");
        crate::set_accessible_name(&stop, "Stop");
        stop.add_css_class("mdow-companion-stop");
        stop.set_tooltip_text(Some("Stop (Ctrl+.)"));
        stop.set_visible(false);
        bar.append(&stop);

        let composer = gtk::Box::new(gtk::Orientation::Vertical, 8);
        composer.add_css_class("mdow-companion-composer");
        composer.append(&chips);
        composer.append(&input_overlay);
        composer.append(&bar);

        let chat = gtk::Box::new(gtk::Orientation::Vertical, 0);
        chat.append(&messages_stack);
        chat.append(&review);
        chat.append(&mentions);
        chat.append(&composer);

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.add_named(&setup_scroll, Some("setup"));
        stack.add_named(&chat, Some("chat"));
        root.append(&stack);

        let session = Session::new();
        let panel = Rc::new(Panel {
            root,
            runtime: session.runtime.clone(),
            session,
            style: RefCell::new(style),
            open_document: RefCell::new(None),
            stack,
            setup,
            setup_key: RefCell::new(String::new()),
            version,
            new_chat,
            messages_stack,
            empty,
            empty_key: RefCell::new(None),
            list,
            scroller,
            stick_to_bottom: Cell::new(true),
            views: RefCell::new(Vec::new()),
            tail,
            tail_key: RefCell::new(String::new()),
            review,
            review_key: RefCell::new(String::new()),
            mentions,
            mention_results: RefCell::new(Vec::new()),
            mention_index: Cell::new(0),
            mention_open: Cell::new(false),
            chips,
            chips_key: RefCell::new(String::new()),
            input,
            placeholder,
            at_button,
            model_button,
            model_label,
            send,
            stop,
            expanded: RefCell::new(HashMap::new()),
            document: RefCell::new(None),
            folder: RefCell::new(None),
            documents: RefCell::new(Vec::new()),
            visible: Cell::new(false),
            render_pending: Cell::new(false),
            closed: Cell::new(false),
        });
        panel.connect();
        panel.render();
        panel
    }

    fn weak(self: &Rc<Self>) -> Weak<Panel> {
        Rc::downgrade(self)
    }

    fn connect(self: &Rc<Self>) {
        let weak = self.weak();
        self.session.set_on_change(Rc::new(move || {
            if let Some(panel) = weak.upgrade() {
                panel.schedule_render();
            }
        }));
        let weak = self.weak();
        self.runtime.subscribe(Box::new(move || match weak.upgrade() {
            Some(panel) if !panel.closed.get() => {
                panel.schedule_render();
                true
            }
            _ => false,
        }));

        let weak = self.weak();
        self.new_chat.connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.expanded.borrow_mut().clear();
                panel.session.new_chat();
            }
        });
        let weak = self.weak();
        self.send.connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.send_input();
            }
        });
        let weak = self.weak();
        self.stop.connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.session.stop();
            }
        });
        let weak = self.weak();
        self.at_button.connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.start_mention();
            }
        });

        let weak = self.weak();
        self.input.buffer().connect_changed(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.input_changed();
            }
        });
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        keys.connect_key_pressed(move |_, key, _, state| match weak.upgrade() {
            Some(panel) if panel.input_key(key, state) => glib::Propagation::Stop,
            _ => glib::Propagation::Proceed,
        });
        self.input.add_controller(keys);

        let weak = self.weak();
        self.mentions.connect_row_activated(move |_, row| {
            if let Some(panel) = weak.upgrade() {
                panel.pick_mention(row.index() as usize);
            }
        });

        let weak = self.weak();
        let adjustment = self.scroller.vadjustment();
        adjustment.connect_value_changed(move |adj| {
            if let Some(panel) = weak.upgrade() {
                panel.stick_to_bottom.set(adj.value() + adj.page_size() >= adj.upper() - 40.0);
            }
        });
        // Long conversations follow the newest reply unless the user has scrolled back.
        let weak = self.weak();
        adjustment.connect_changed(move |adj| {
            if let Some(panel) = weak.upgrade()
                && panel.stick_to_bottom.get()
            {
                adj.set_value(adj.upper() - adj.page_size());
            }
        });
    }

    pub fn set_open_document(&self, open: OpenDocument) {
        self.open_document.replace(Some(open));
    }

    fn open_path(&self, path: &Path) {
        let open = self.open_document.borrow().clone();
        if let Some(open) = open {
            open(path);
        }
    }

    /// Restyles everything drawn from the palette or the fonts.
    pub fn set_style(self: &Rc<Self>, style: Rc<Style>) {
        self.style.replace(style);
        for view in self.views.borrow_mut().drain(..) {
            self.list.remove(&view.root);
        }
        self.schedule_render();
    }

    pub fn document_changed(
        self: &Rc<Self>,
        document: Option<PathBuf>,
        folder: Option<PathBuf>,
        documents: Vec<PathBuf>,
    ) {
        self.session
            .set_context(DocumentContext { document: document.clone(), folder: folder.clone() });
        self.document.replace(document);
        self.folder.replace(folder);
        self.documents.replace(documents);
        self.schedule_render();
    }

    pub fn set_shown(self: &Rc<Self>, shown: bool) {
        self.root.set_visible(shown);
        if shown == self.visible.replace(shown) {
            return;
        }
        self.runtime.panel_visibility_changed(shown);
        if shown {
            self.render();
            if !self.runtime.is_available() {
                self.reconnect();
            }
            self.focus_composer();
        }
    }

    pub fn is_shown(&self) -> bool {
        self.visible.get()
    }

    pub fn focus_composer(&self) {
        self.input.grab_focus();
    }

    /// The window closed.
    pub fn close(self: &Rc<Self>) {
        self.closed.set(true);
        if self.visible.replace(false) {
            self.runtime.panel_visibility_changed(false);
        }
        self.session.shutdown();
    }

    fn reconnect(self: &Rc<Self>) {
        let runtime = self.runtime.clone();
        let directory = self.session.directory();
        glib::spawn_future_local(async move {
            runtime.connect(Some(directory)).await;
        });
    }

    /// Window-wide shortcuts, active while there's something for them to do. Accept and Stop work
    /// from anywhere; Ctrl+Backspace only outside the composer, where it edits text.
    pub fn handle_shortcut(self: &Rc<Self>, key: gdk::Key, state: gdk::ModifierType) -> bool {
        if !self.visible.get() || !state.contains(gdk::ModifierType::CONTROL_MASK) {
            return false;
        }
        let conversation = self.session.conversation();
        match key {
            gdk::Key::period if conversation.streaming => {
                self.session.stop();
                true
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                self.review_pending(&conversation, PermissionDecision::Approve)
            }
            gdk::Key::BackSpace if !self.input.has_focus() => {
                self.review_pending(&conversation, PermissionDecision::Reject)
            }
            _ => false,
        }
    }

    /// Typing with focus on one of the panel's buttons is meant for the composer, not for the
    /// reader's single-key shortcuts.
    pub fn claim_typing(
        &self,
        key: gdk::Key,
        state: gdk::ModifierType,
        focus: Option<&gtk::Widget>,
    ) -> bool {
        let modifiers = gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::SUPER_MASK;
        let inside = focus.is_some_and(|f| {
            f.is_ancestor(&self.root) && !f.is::<gtk::Text>() && !f.is::<gtk::TextView>()
        });
        if !self.visible.get()
            || !inside
            || state.intersects(modifiers)
            || self.stack.visible_child_name().as_deref() != Some("chat")
        {
            return false;
        }
        let Some(text) = key.to_unicode().filter(|c| !c.is_control() && *c != ' ') else {
            return false;
        };
        self.input.grab_focus();
        self.input.buffer().insert_at_cursor(&text.to_string());
        true
    }

    fn review_pending(
        self: &Rc<Self>,
        conversation: &super::conversation::Conversation,
        decision: PermissionDecision,
    ) -> bool {
        let Some(permission) = conversation.pending_change().and_then(|c| c.permission_id.clone())
        else {
            return false;
        };
        self.session.review(&permission, decision);
        self.focus_composer();
        true
    }

    // ── Composer ─────────────────────────────────────────────────────────────

    fn input_text(&self) -> String {
        let buffer = self.input.buffer();
        buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
    }

    fn send_input(self: &Rc<Self>) {
        let text = self.input_text();
        if text.trim().is_empty() || self.session.conversation().streaming {
            return;
        }
        self.session.send(&text);
        self.input.buffer().set_text("");
        self.stick_to_bottom.set(true);
    }

    fn input_changed(self: &Rc<Self>) {
        let text = self.input_text();
        self.placeholder.set_visible(text.is_empty());
        self.send.set_sensitive(!text.trim().is_empty());
        match logic::mention_query(&text) {
            Some(query) => {
                let query = query.to_string();
                self.mention_open.set(true);
                self.mention_index.set(0);
                self.fill_mentions(&query);
            }
            None => self.close_mentions(),
        }
    }

    fn close_mentions(&self) {
        self.mention_open.set(false);
        self.mentions.set_visible(false);
    }

    fn fill_mentions(self: &Rc<Self>, query: &str) {
        let current = self.document.borrow().clone();
        let candidates: Vec<PathBuf> = self
            .documents
            .borrow()
            .iter()
            .filter(|path| Some(*path) != current.as_ref())
            .cloned()
            .collect();
        let root = self.folder.borrow().clone();
        let relative = |path: &Path| match root.as_deref().and_then(|r| path.strip_prefix(r).ok()) {
            Some(rest) => rest.to_string_lossy().into_owned(),
            None => files::display_name(path),
        };
        let results: Vec<PathBuf> = if query.is_empty() {
            candidates.into_iter().take(MENTION_LIMIT).collect()
        } else {
            let mut scored: Vec<(f64, PathBuf)> = candidates
                .into_iter()
                .filter_map(|path| {
                    let score = files::fuzzy_score(query, &files::display_name(&path))
                        .max(files::fuzzy_score(query, &relative(&path)));
                    (score > 0.0).then_some((score, path))
                })
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.into_iter().take(MENTION_LIMIT).map(|(_, path)| path).collect()
        };
        self.mentions.remove_all();
        for path in &results {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 7);
            let icon = gtk::Image::from_icon_name("text-x-generic-symbolic");
            icon.set_pixel_size(12);
            icon.add_css_class("mdow-companion-tertiary");
            row.append(&icon);
            row.append(&label(&files::display_name(path), &[]));
            let parent = path.parent().map(&relative).unwrap_or_default();
            let folder = ellipsized(
                if root.as_deref() == path.parent() { "" } else { &parent },
                pango::EllipsizeMode::Start,
                &["mdow-companion-tertiary", "mdow-companion-small-mono"],
            );
            folder.set_hexpand(true);
            folder.set_xalign(1.0);
            row.append(&folder);
            self.mentions.append(&row);
        }
        self.mentions.set_visible(!results.is_empty());
        self.mention_results.replace(results);
        self.select_mention(0);
    }

    fn select_mention(&self, index: usize) {
        self.mention_index.set(index);
        self.mentions.select_row(self.mentions.row_at_index(index as i32).as_ref());
    }

    fn start_mention(self: &Rc<Self>) {
        let mut text = self.input_text();
        if !text.is_empty() && !text.ends_with(' ') {
            text.push(' ');
        }
        text.push('@');
        self.input.buffer().set_text(&text);
        self.input.grab_focus();
    }

    fn pick_mention(self: &Rc<Self>, index: usize) {
        let Some(path) = self.mention_results.borrow().get(index).cloned() else { return };
        self.session.add_tag(&path);
        let text = self.input_text();
        self.input.buffer().set_text(logic::without_mention(&text));
        self.close_mentions();
        self.input.grab_focus();
    }

    /// Returns whether the composer used the key.
    fn input_key(self: &Rc<Self>, key: gdk::Key, state: gdk::ModifierType) -> bool {
        let count = self.mention_results.borrow().len();
        if self.mention_open.get() && count > 0 {
            let index = self.mention_index.get();
            match key {
                gdk::Key::Down => {
                    self.select_mention((index + 1) % count);
                    return true;
                }
                gdk::Key::Up => {
                    self.select_mention(if index == 0 { count - 1 } else { index - 1 });
                    return true;
                }
                gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::Tab => {
                    self.pick_mention(index);
                    return true;
                }
                gdk::Key::Escape => {
                    self.close_mentions();
                    return true;
                }
                _ => {}
            }
        }
        if !matches!(key, gdk::Key::Return | gdk::Key::KP_Enter) {
            return false;
        }
        // Shift- or Alt-Return adds a line; Ctrl-Return is for reviews; Return sends.
        if state.intersects(gdk::ModifierType::SHIFT_MASK | gdk::ModifierType::ALT_MASK) {
            self.input.buffer().insert_at_cursor("\n");
            return true;
        }
        if state.contains(gdk::ModifierType::CONTROL_MASK) {
            return false;
        }
        self.send_input();
        true
    }

    fn build_model_popover(self: &Rc<Self>, state: &ModelState) -> gtk::Popover {
        let popover = gtk::Popover::new();
        popover.set_has_arrow(false);
        popover.add_css_class("mdow-companion-models");
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search models"));
        search.set_margin_start(6);
        search.set_margin_end(6);
        search.set_margin_top(6);
        search.set_margin_bottom(6);
        content.append(&search);
        content.append(&hairline());
        let list = gtk::Box::new(gtk::Orientation::Vertical, 1);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .min_content_height(320)
            .min_content_width(300)
            .build();
        content.append(&scroll);
        popover.set_child(Some(&content));

        let options = Rc::new(state.options.clone());
        let current = state.current_value.clone();
        let matching = {
            let options = options.clone();
            move |query: &str| -> Vec<ModelOption> {
                options
                    .iter()
                    .filter(|o| {
                        query.trim().is_empty()
                            || files::fuzzy_score(query, &format!("{} {}", o.name, o.provider_name))
                                > 0.0
                    })
                    .cloned()
                    .collect()
            }
        };
        let choose = {
            let weak = self.weak();
            let popover = popover.downgrade();
            Rc::new(move |value: &str| {
                if let Some(popover) = popover.upgrade() {
                    popover.popdown();
                }
                if let Some(panel) = weak.upgrade() {
                    panel.session.select_model(value);
                }
            })
        };
        let fill = {
            let matching = matching.clone();
            let choose = choose.clone();
            let list = list.clone();
            move |query: &str| {
                clear(&list);
                let shown = matching(query);
                if shown.is_empty() {
                    let none = label("No matching models.", &["mdow-companion-secondary"]);
                    none.set_margin_top(16);
                    none.set_halign(gtk::Align::Center);
                    list.append(&none);
                    return;
                }
                let mut last_provider = None;
                for option in shown {
                    if last_provider.as_ref() != Some(&option.provider_id) {
                        let heading = label(&option.provider_name, &["mdow-companion-group"]);
                        list.append(&heading);
                        last_provider = Some(option.provider_id.clone());
                    }
                    let row = gtk::Button::new();
                    row.add_css_class("flat");
                    row.add_css_class("mdow-companion-model-row");
                    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                    let name = ellipsized(&option.name, pango::EllipsizeMode::End, &[]);
                    name.set_hexpand(true);
                    content.append(&name);
                    if current.as_ref() == Some(&option.value) {
                        let check = gtk::Image::from_icon_name("object-select-symbolic");
                        check.set_pixel_size(12);
                        check.add_css_class("mdow-companion-accent");
                        content.append(&check);
                    }
                    row.set_child(Some(&content));
                    let choose = choose.clone();
                    let value = option.value.clone();
                    row.connect_clicked(move |_| choose(&value));
                    list.append(&row);
                }
            }
        };
        fill("");
        search.connect_search_changed(move |entry| fill(&entry.text()));
        search.connect_activate(move |entry| {
            if let Some(first) = matching(&entry.text()).first() {
                choose(&first.value);
            }
        });
        popover
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    fn schedule_render(self: &Rc<Self>) {
        if self.render_pending.replace(true) {
            return;
        }
        let weak = self.weak();
        glib::timeout_add_local_once(RENDER_DELAY, move || {
            if let Some(panel) = weak.upgrade() {
                panel.render_pending.set(false);
                if panel.visible.get() {
                    panel.render();
                }
            }
        });
    }

    fn render(self: &Rc<Self>) {
        let status = self.runtime.status();
        let models = self.runtime.model_state();
        let conversation = self.session.conversation();
        let ready =
            status.as_ref().is_some_and(|s| s.availability == RuntimeAvailability::Available);
        let no_models = ready && !models.stale && models.options.is_empty();

        self.version.set_text(&match status.as_ref().and_then(|s| s.version.as_ref()) {
            Some(version) if ready => format!("OpenCode {version}"),
            _ => String::new(),
        });
        self.new_chat.set_sensitive(!conversation.messages.is_empty());

        if !ready {
            self.render_setup(status.as_ref());
            self.stack.set_visible_child_name("setup");
        } else if no_models && conversation.messages.is_empty() {
            let message = models
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "OpenCode has no models available yet.".to_string());
            self.show_setup("Connect a Model Provider", &message, &["opencode auth login"], false);
            self.stack.set_visible_child_name("setup");
        } else {
            self.render_chat(&conversation, &models);
            self.stack.set_visible_child_name("chat");
        }
    }

    fn render_setup(self: &Rc<Self>, status: Option<&RuntimeStatus>) {
        match status.map(|s| s.availability) {
            None => {
                if *self.setup_key.borrow() == "starting" {
                    return;
                }
                self.setup_key.replace("starting".to_string());
                clear(&self.setup);
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                row.set_halign(gtk::Align::Center);
                row.set_valign(gtk::Align::Center);
                row.set_vexpand(true);
                let spinner = gtk::Spinner::new();
                spinner.start();
                row.append(&spinner);
                row.append(&label("Starting OpenCode…", &["mdow-companion-secondary"]));
                self.setup.append(&row);
            }
            Some(RuntimeAvailability::Missing) => self.show_setup(
                "Connect OpenCode",
                "The companion runs on OpenCode, using the models and subscriptions you already use there. Install OpenCode 2, then sign in to a provider:",
                &[INSTALL_COMMAND, "opencode auth login"],
                true,
            ),
            Some(RuntimeAvailability::Outdated) => {
                let version = status.and_then(|s| s.version.clone()).unwrap_or_default();
                self.show_setup(
                    "Update OpenCode",
                    &format!(
                        "OpenCode {version} is installed, but the companion needs OpenCode 2 or newer."
                    ),
                    &["opencode upgrade"],
                    true,
                );
            }
            Some(_) => {
                let detail = status
                    .and_then(|s| s.detail.clone())
                    .unwrap_or_else(|| "Something went wrong while starting OpenCode.".to_string());
                self.show_setup("OpenCode Didn’t Start", &detail, &[], true);
            }
        }
    }

    fn show_setup(self: &Rc<Self>, title: &str, message: &str, commands: &[&str], docs: bool) {
        let key = format!("{title}\n{message}");
        if *self.setup_key.borrow() == key {
            return;
        }
        self.setup_key.replace(key);
        clear(&self.setup);
        self.setup.append(&label(title, &["mdow-companion-display"]));
        let text = wrapped(message, &["mdow-companion-secondary"]);
        text.set_selectable(false);
        self.setup.append(&text);
        for command in commands {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            row.add_css_class("mdow-companion-card");
            row.add_css_class("mdow-companion-command");
            row.append(&label("$", &["mdow-companion-tertiary", "mdow-companion-mono"]));
            let text = wrapped(command, &["mdow-companion-mono"]);
            text.set_hexpand(true);
            row.append(&text);
            let copy = icon_button("edit-copy-symbolic", "Copy");
            let command = command.to_string();
            copy.connect_clicked(move |button| button.clipboard().set_text(&command));
            row.append(&copy);
            self.setup.append(&row);
        }
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        buttons.set_margin_top(4);
        let retry = pill_button("Check Again", "mdow-pill-secondary");
        let weak = self.weak();
        retry.connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.setup_key.replace(String::new());
                panel.reconnect();
            }
        });
        buttons.append(&retry);
        if docs {
            let open = pill_button("OpenCode Docs", "mdow-pill-quiet");
            open.connect_clicked(|button| {
                let window = button.root().and_downcast::<gtk::Window>();
                gtk::UriLauncher::new(DOCS_URL).launch(
                    window.as_ref(),
                    gio::Cancellable::NONE,
                    |_| {},
                );
            });
            buttons.append(&open);
        }
        self.setup.append(&buttons);
    }

    fn render_chat(
        self: &Rc<Self>,
        conversation: &super::conversation::Conversation,
        models: &ModelState,
    ) {
        if conversation.messages.is_empty() {
            self.render_empty();
            self.messages_stack.set_visible_child_name("empty");
        } else {
            self.messages_stack.set_visible_child_name("list");
        }
        self.sync_messages(conversation);
        self.render_tail(conversation);
        self.render_review(conversation);
        self.render_composer(conversation, models);
    }

    fn render_empty(self: &Rc<Self>) {
        let document = self.document.borrow().as_deref().map(files::display_name);
        let scope =
            document.clone().or_else(|| self.folder.borrow().as_deref().map(files::display_name));
        if *self.empty_key.borrow() == scope {
            return;
        }
        self.empty_key.replace(scope.clone());
        clear(&self.empty);
        let title =
            scope.map_or("Ask about your documents".to_string(), |s| format!("Ask about {s}"));
        let title = wrapped(&title, &["mdow-companion-display"]);
        title.set_selectable(false);
        self.empty.append(&title);
        let text = wrapped(
            "Get answers, summaries and edits. Every change comes back as a diff for you to approve.",
            &["mdow-companion-secondary"],
        );
        text.set_selectable(false);
        text.set_margin_top(6);
        self.empty.append(&text);
        let prompts = gtk::Box::new(gtk::Orientation::Vertical, 4);
        prompts.set_margin_top(20);
        for (title, prompt) in suggestions(document.as_deref()) {
            let button = gtk::Button::with_label(&title);
            button.add_css_class("mdow-companion-suggestion");
            if let Some(text) = button.child().and_downcast::<gtk::Label>() {
                text.set_xalign(0.0);
                text.set_wrap(true);
            }
            let weak = self.weak();
            button.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    panel.session.send(&prompt);
                    panel.focus_composer();
                }
            });
            prompts.append(&button);
        }
        self.empty.append(&prompts);
    }

    fn sync_messages(self: &Rc<Self>, conversation: &super::conversation::Conversation) {
        let mut views = self.views.borrow_mut();
        let same = views
            .iter()
            .zip(&conversation.messages)
            .take_while(|(view, message)| view.id == message.id)
            .count();
        for view in views.drain(same..) {
            self.list.remove(&view.root);
        }
        for message in &conversation.messages[same.min(conversation.messages.len())..] {
            let view = self.new_message_view(message);
            self.list.insert_child_after(
                &view.root,
                views.last().map(|v| v.root.upcast_ref::<gtk::Widget>()),
            );
            views.push(view);
        }
        for (view, message) in views.iter().zip(&conversation.messages) {
            if message.role == MessageRole::Assistant {
                self.sync_parts(view, message);
                self.sync_footer(view, message);
            }
        }
    }

    fn new_message_view(self: &Rc<Self>, message: &Message) -> MessageView {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.add_css_class("mdow-companion-message");
        let parts_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let footer = gtk::Box::new(gtk::Orientation::Vertical, 6);
        if message.role == MessageRole::User {
            let bubble = wrapped(&message.content, &["mdow-companion-user"]);
            bubble.set_halign(gtk::Align::End);
            bubble.set_margin_start(36);
            root.append(&bubble);
        } else {
            root.append(&parts_box);
            root.append(&footer);
        }
        MessageView {
            id: message.id.clone(),
            parts_box,
            footer,
            footer_key: RefCell::new(String::new()),
            parts: RefCell::new(Vec::new()),
            root,
        }
    }

    fn sync_parts(self: &Rc<Self>, view: &MessageView, message: &Message) {
        let streaming = message.status == Some(MessageStatus::Streaming);
        let mut parts = view.parts.borrow_mut();
        for (index, part) in message.parts.iter().enumerate() {
            let active = streaming && matches!(part, Part::Thinking { done: false, .. });
            if let Some(existing) = parts.get_mut(index) {
                if existing.part == *part && existing.active == active {
                    continue;
                }
                if let (Part::Text(text), Part::Text(_)) = (part, &existing.part) {
                    self.sync_reply(existing, text);
                    existing.part = part.clone();
                    continue;
                }
            }
            let mut fresh = self.build_part(part, active, &message.id, index);
            match parts.get_mut(index) {
                Some(existing) => {
                    view.parts_box.insert_child_after(&fresh.widget, Some(&existing.widget));
                    view.parts_box.remove(&existing.widget);
                    std::mem::swap(existing, &mut fresh);
                }
                None => {
                    view.parts_box.append(&fresh.widget);
                    parts.push(fresh);
                }
            }
        }
        let kept = message.parts.len().min(parts.len());
        for extra in parts.drain(kept..) {
            view.parts_box.remove(&extra.widget);
        }
    }

    fn sync_reply(&self, view: &mut PartView, text: &str) {
        let style = self.style.borrow().clone();
        let Some(container) = view.widget.downcast_ref::<gtk::Box>() else { return };
        let blocks = logic::split_reply(text);
        for (index, block) in blocks.iter().enumerate() {
            if view.blocks.get(index).is_some_and(|(existing, _)| existing == block) {
                continue;
            }
            let widget = reply_block(block, &style);
            match view.blocks.get_mut(index) {
                Some(existing) => {
                    container.insert_child_after(&widget, Some(&existing.1));
                    container.remove(&existing.1);
                    *existing = (block.clone(), widget);
                }
                None => {
                    container.append(&widget);
                    view.blocks.push((block.clone(), widget));
                }
            }
        }
        let kept = blocks.len().min(view.blocks.len());
        for (_, extra) in view.blocks.drain(kept..) {
            container.remove(&extra);
        }
    }

    fn build_part(
        self: &Rc<Self>,
        part: &Part,
        active: bool,
        message_id: &str,
        index: usize,
    ) -> PartView {
        let key = format!("{message_id}:{index}");
        let widget: gtk::Widget = match part {
            Part::Text(_) => gtk::Box::new(gtk::Orientation::Vertical, 9).upcast(),
            Part::Thinking { text, .. } => self.reasoning_view(text, active, &key),
            Part::Tool(tool) => self.tool_view(tool),
            Part::Change(change) => self.change_view(change),
        };
        let mut view = PartView { part: part.clone(), active, widget, blocks: Vec::new() };
        if let Part::Text(text) = part {
            self.sync_reply(&mut view, text);
        }
        view
    }

    /// A row that reveals more when clicked, remembered across rebuilds under `key`.
    fn disclosure(
        self: &Rc<Self>,
        key: String,
        heading: &impl IsA<gtk::Widget>,
        content: &impl IsA<gtk::Widget>,
    ) -> gtk::Widget {
        let expanded = self.expanded.borrow().get(&key).copied().unwrap_or(false);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let button = gtk::Button::new();
        button.add_css_class("flat");
        button.add_css_class("mdow-companion-disclosure");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 7);
        row.append(heading);
        let chevron = gtk::Image::from_icon_name(if expanded {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        });
        chevron.set_pixel_size(10);
        chevron.add_css_class("mdow-companion-tertiary");
        row.append(&chevron);
        button.set_child(Some(&row));
        root.append(&button);
        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(120);
        revealer.set_reveal_child(expanded);
        revealer.set_child(Some(content));
        root.append(&revealer);
        let weak = self.weak();
        button.connect_clicked(move |_| {
            let open = !revealer.reveals_child();
            revealer.set_reveal_child(open);
            chevron.set_icon_name(Some(if open {
                "pan-down-symbolic"
            } else {
                "pan-end-symbolic"
            }));
            if let Some(panel) = weak.upgrade() {
                panel.expanded.borrow_mut().insert(key.clone(), open);
            }
        });
        root.upcast()
    }

    fn reasoning_view(self: &Rc<Self>, text: &str, active: bool, key: &str) -> gtk::Widget {
        let heading = gtk::Box::new(gtk::Orientation::Horizontal, 7);
        if active {
            let spinner = gtk::Spinner::new();
            spinner.start();
            heading.append(&spinner);
        }
        heading.append(&label(
            if active { "Thinking…" } else { "Thought" },
            &["mdow-companion-secondary"],
        ));
        let body = wrapped(text.trim(), &["mdow-companion-secondary", "mdow-companion-reasoning"]);
        self.disclosure(format!("thinking:{key}"), &heading, &body)
    }

    fn tool_view(self: &Rc<Self>, tool: &ToolPart) -> gtk::Widget {
        let heading = gtk::Box::new(gtk::Orientation::Horizontal, 7);
        match tool.state {
            ToolState::Pending | ToolState::Running => {
                let spinner = gtk::Spinner::new();
                spinner.start();
                heading.append(&spinner);
            }
            state => {
                let (icon, class) = match state {
                    ToolState::Error => ("dialog-warning-symbolic", "mdow-companion-removed"),
                    ToolState::Cancelled => ("list-remove-symbolic", "mdow-companion-tertiary"),
                    _ => ("object-select-symbolic", "mdow-companion-tertiary"),
                };
                let image = gtk::Image::from_icon_name(icon);
                image.set_pixel_size(12);
                image.add_css_class(class);
                heading.append(&image);
            }
        }
        let class = if tool.state == ToolState::Error {
            "mdow-companion-removed"
        } else {
            "mdow-companion-secondary"
        };
        heading.append(&ellipsized(
            &tool.name,
            pango::EllipsizeMode::Middle,
            &[class, "mdow-companion-small-mono"],
        ));
        let details = gtk::Box::new(gtk::Orientation::Vertical, 6);
        if let Some(input) = tool.input.as_deref().filter(|i| !i.is_empty()) {
            details.append(&detail_box("input", input, false));
        }
        if let Some(error) = &tool.error {
            details.append(&detail_box("error", error, true));
        } else if let Some(output) = tool.output.as_deref().filter(|o| !o.is_empty()) {
            details.append(&detail_box("output", output, false));
        }
        self.disclosure(format!("tool:{}", tool.tool_call_id), &heading, &details)
    }

    fn change_view(self: &Rc<Self>, change: &ChangePart) -> gtk::Widget {
        let settled = matches!(change.status, ChangeStatus::Rejected | ChangeStatus::Failed);
        let additions: i64 = change.files.iter().map(|f| f.additions).sum();
        let deletions: i64 = change.files.iter().map(|f| f.deletions).sum();
        let title = match change.files.as_slice() {
            [] => "Proposed change".to_string(),
            [file] => file.display_path.clone(),
            files => format!("{} documents", files.len()),
        };
        let added =
            !change.files.is_empty() && change.files.iter().all(|f| f.status == FileStatus::Added);
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("mdow-companion-change");
        if change.awaiting_review() {
            card.add_css_class("mdow-companion-change-pending");
        }
        if settled {
            card.add_css_class("mdow-companion-settled");
        }

        let top = gtk::Box::new(gtk::Orientation::Horizontal, 7);
        top.add_css_class("mdow-companion-change-top");
        let icon = gtk::Image::from_icon_name(if added {
            "document-new-symbolic"
        } else {
            "document-edit-symbolic"
        });
        icon.set_pixel_size(12);
        icon.add_css_class("mdow-companion-tertiary");
        top.append(&icon);
        let name = ellipsized(&title, pango::EllipsizeMode::Middle, &["mdow-companion-small-mono"]);
        name.set_hexpand(true);
        top.append(&name);
        if additions > 0 || deletions > 0 {
            top.append(&label(
                &format!("+{additions}"),
                &["mdow-companion-added", "mdow-companion-small-mono"],
            ));
            top.append(&label(
                &format!("−{deletions}"),
                &["mdow-companion-removed", "mdow-companion-small-mono"],
            ));
        }
        top.append(&status_pill(change));
        card.append(&top);

        for file in &change.files {
            card.append(&hairline());
            if change.files.len() > 1 {
                let path = ellipsized(
                    &file.display_path,
                    pango::EllipsizeMode::Middle,
                    &["mdow-companion-tertiary", "mdow-companion-small-mono", "mdow-companion-pad"],
                );
                card.append(&path);
            }
            card.append(&self.diff_view(file, settled));
        }

        if let Some(error) =
            change.error.as_deref().filter(|_| change.status == ChangeStatus::Failed)
        {
            card.append(&hairline());
            let text = wrapped(error, &["mdow-companion-removed", "mdow-companion-pad"]);
            card.append(&text);
        }

        if change.awaiting_review()
            || (change.status == ChangeStatus::Applied && !change.files.is_empty())
        {
            card.append(&hairline());
            let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            actions.add_css_class("mdow-companion-change-actions");
            if let [file] = change.files.as_slice()
                && file.status != FileStatus::Deleted
            {
                let open = pill_button("Open Document", "mdow-pill-quiet");
                let weak = self.weak();
                let path = PathBuf::from(&file.path);
                open.connect_clicked(move |_| {
                    if let Some(panel) = weak.upgrade() {
                        panel.open_path(&path);
                    }
                });
                actions.append(&open);
            }
            let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            spacer.set_hexpand(true);
            actions.append(&spacer);
            if let Some(permission) =
                change.permission_id.as_deref().filter(|_| change.awaiting_review())
            {
                self.append_review_buttons(&actions, permission);
            }
            card.append(&actions);
        }
        card.upcast()
    }

    fn append_review_buttons(self: &Rc<Self>, row: &gtk::Box, permission: &str) {
        for (text, class, decision, tooltip) in [
            (
                "Decline",
                "mdow-pill-secondary",
                PermissionDecision::Reject,
                "Decline (Ctrl+Backspace)",
            ),
            ("Accept", "mdow-pill-primary", PermissionDecision::Approve, "Accept (Ctrl+Enter)"),
        ] {
            let button = pill_button(text, class);
            button.set_tooltip_text(Some(tooltip));
            let weak = self.weak();
            let permission = permission.to_string();
            button.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    panel.session.review(&permission, decision);
                    panel.focus_composer();
                }
            });
            row.append(&button);
        }
    }

    fn diff_view(self: &Rc<Self>, file: &FileChange, collapsed: bool) -> gtk::Widget {
        let hunks = logic::parse_diff(&file.patch);
        let total: usize = hunks.iter().map(|h| h.lines.len()).sum();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("mdow-companion-diff");
        if hunks.is_empty() {
            let text = if file.status == FileStatus::Deleted {
                "Deletes this document."
            } else {
                "No preview available."
            };
            root.append(&label(text, &["mdow-companion-secondary", "mdow-companion-pad"]));
            return root.upcast();
        }
        let key = format!("diff:{}", file.path);
        let shown = self.expanded.borrow().get(&key).copied().unwrap_or(!collapsed);
        let lines = gtk::Box::new(gtk::Orientation::Vertical, 0);
        fill_diff(&lines, &hunks, shown || total <= DIFF_PREVIEW_LINES);
        root.append(&lines);
        if total > DIFF_PREVIEW_LINES {
            let toggle = gtk::Button::with_label(&show_more_label(shown, total));
            toggle.add_css_class("flat");
            toggle.add_css_class("mdow-companion-show-more");
            let weak = self.weak();
            let state = Cell::new(shown);
            toggle.connect_clicked(move |button| {
                let open = !state.get();
                state.set(open);
                fill_diff(&lines, &hunks, open);
                button.set_label(&show_more_label(open, total));
                if let Some(panel) = weak.upgrade() {
                    panel.expanded.borrow_mut().insert(key.clone(), open);
                }
            });
            root.append(&toggle);
        }
        root.upcast()
    }

    fn sync_footer(self: &Rc<Self>, view: &MessageView, message: &Message) {
        let streaming = message.status == Some(MessageStatus::Streaming);
        let waiting = streaming && is_waiting(&message.parts);
        let copyable =
            message.status == Some(MessageStatus::Complete) && !message.content.trim().is_empty();
        let key = format!(
            "{waiting}:{}:{:?}:{}:{copyable}",
            message.parts.is_empty(),
            message.status,
            message.citations.len(),
        );
        if *view.footer_key.borrow() == key {
            return;
        }
        view.footer_key.replace(key);
        clear(&view.footer);
        if waiting {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let spinner = gtk::Spinner::new();
            spinner.start();
            row.append(&spinner);
            let text = if message.parts.is_empty() { "Thinking…" } else { "Working…" };
            row.append(&label(text, &["mdow-companion-secondary"]));
            view.footer.append(&row);
        }
        if message.status == Some(MessageStatus::Cancelled) {
            view.footer.append(&label("Stopped", &["mdow-companion-tertiary"]));
        }
        if !message.citations.is_empty() {
            view.footer.append(&self.sources_view(message));
        }
        if copyable {
            let copy = icon_button("edit-copy-symbolic", "Copy Response");
            copy.add_css_class("mdow-companion-copy");
            copy.set_halign(gtk::Align::Start);
            let text = message.content.clone();
            copy.connect_clicked(move |button| {
                button.clipboard().set_text(&text);
                button.set_icon_name("object-select-symbolic");
                button.set_tooltip_text(Some("Copied"));
                let button = button.downgrade();
                glib::timeout_add_local_once(Duration::from_millis(1500), move || {
                    if let Some(button) = button.upgrade() {
                        button.set_icon_name("edit-copy-symbolic");
                        button.set_tooltip_text(Some("Copy Response"));
                    }
                });
            });
            view.footer.append(&copy);
        }
    }

    fn sources_view(self: &Rc<Self>, message: &Message) -> gtk::Widget {
        let count = message.citations.len();
        let heading = label(
            &if count == 1 { "Used 1 source".to_string() } else { format!("Used {count} sources") },
            &["mdow-companion-tertiary"],
        );
        let list = gtk::Box::new(gtk::Orientation::Vertical, 2);
        for citation in &message.citations {
            let row = gtk::Button::new();
            row.add_css_class("flat");
            row.add_css_class("mdow-companion-source");
            row.set_tooltip_text(Some(&citation.path));
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let icon = gtk::Image::from_icon_name("text-x-generic-symbolic");
            icon.set_pixel_size(11);
            content.append(&icon);
            content.append(&ellipsized(
                &citation.label,
                pango::EllipsizeMode::Middle,
                &["mdow-companion-small-mono"],
            ));
            row.set_child(Some(&content));
            let weak = self.weak();
            let path = PathBuf::from(&citation.path);
            row.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    panel.open_path(&path);
                }
            });
            list.append(&row);
        }
        self.disclosure(format!("sources:{}", message.id), &heading, &list)
    }

    fn render_tail(self: &Rc<Self>, conversation: &super::conversation::Conversation) {
        let errored =
            conversation.messages.last().is_some_and(|m| m.status == Some(MessageStatus::Error));
        let key = format!("{errored}:{:?}:{:?}", conversation.error, conversation.warnings);
        if *self.tail_key.borrow() == key {
            return;
        }
        self.tail_key.replace(key);
        clear(&self.tail);
        if errored
            && let Some(last_user) =
                conversation.messages.iter().rev().find(|m| m.role == MessageRole::User)
        {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.add_css_class("mdow-companion-error");
            let icon = gtk::Image::from_icon_name("dialog-error-symbolic");
            icon.set_pixel_size(12);
            icon.set_valign(gtk::Align::Start);
            icon.add_css_class("mdow-companion-removed");
            row.append(&icon);
            let text =
                wrapped(conversation.error.as_deref().unwrap_or("Something went wrong."), &[]);
            text.set_hexpand(true);
            row.append(&text);
            let retry = pill_button("Retry", "mdow-pill-secondary");
            let weak = self.weak();
            let prompt = last_user.content.clone();
            retry.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    panel.session.send(&prompt);
                }
            });
            row.append(&retry);
            self.tail.append(&row);
        }
        for warning in &conversation.warnings {
            self.tail.append(&wrapped(warning, &["mdow-companion-tertiary"]));
        }
    }

    /// Stays above the composer while OpenCode waits for a decision, wherever the change card has
    /// scrolled to.
    fn render_review(self: &Rc<Self>, conversation: &super::conversation::Conversation) {
        let change = conversation.pending_change();
        let key = format!(
            "{:?}:{:?}",
            change.and_then(|c| c.permission_id.clone()),
            self.document.borrow()
        );
        if *self.review_key.borrow() == key {
            return;
        }
        self.review_key.replace(key);
        clear(&self.review);
        let Some(change) = change else {
            self.review.set_visible(false);
            return;
        };
        self.review.set_visible(true);
        let first = change.files.first();
        let name = match change.files.len() {
            0 => "Suggested edit".to_string(),
            1 => first.map_or(String::new(), |f| f.display_path.clone()),
            more => {
                format!("{} and {} more", first.map_or("", |f| f.display_path.as_str()), more - 1)
            }
        };
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("mdow-companion-dot");
        dot.set_valign(gtk::Align::Center);
        self.review.append(&dot);
        let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
        text.set_hexpand(true);
        text.append(&label("Suggested edit", &["mdow-companion-strong"]));
        text.append(&ellipsized(
            &name,
            pango::EllipsizeMode::Middle,
            &["mdow-companion-tertiary", "mdow-companion-small-mono"],
        ));
        self.review.append(&text);
        let current = self.document.borrow().clone();
        if let Some(file) = first
            && file.status != FileStatus::Deleted
            && current.as_deref() != Some(Path::new(&file.path))
        {
            let show = pill_button("Show", "mdow-pill-quiet");
            show.set_tooltip_text(Some(&format!("Open {}", file.display_path)));
            let weak = self.weak();
            let path = PathBuf::from(&file.path);
            show.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    panel.open_path(&path);
                }
            });
            self.review.append(&show);
        }
        if let Some(permission) = &change.permission_id {
            self.append_review_buttons(&self.review, permission);
        }
    }

    fn render_composer(
        self: &Rc<Self>,
        conversation: &super::conversation::Conversation,
        models: &ModelState,
    ) {
        let document = self.document.borrow().clone();
        self.placeholder.set_text(&match &document {
            Some(path) => format!("Ask about {}, or ask for an edit…", files::display_name(path)),
            None => "Ask about your documents…".to_string(),
        });
        self.send.set_visible(!conversation.streaming);
        self.stop.set_visible(conversation.streaming);
        self.at_button.set_sensitive(!self.documents.borrow().is_empty());

        let chips_key = format!("{document:?}:{:?}", conversation.tags);
        if *self.chips_key.borrow() != chips_key {
            self.chips_key.replace(chips_key);
            clear(&self.chips);
            if let Some(path) = &document {
                let chip = chip("text-x-generic-symbolic", &files::display_name(path), true);
                chip.set_tooltip_text(Some(&format!(
                    "OpenCode sees that you are viewing {}",
                    path.display()
                )));
                self.chips.append(&chip);
            }
            for tag in &conversation.tags {
                let chip = chip("", &files::display_name(Path::new(&tag.path)), false);
                chip.set_tooltip_text(Some(&tag.path));
                let remove = gtk::Button::from_icon_name("window-close-symbolic");
                crate::set_accessible_name(&remove, "Remove");
                remove.add_css_class("flat");
                remove.add_css_class("mdow-companion-chip-remove");
                remove.set_tooltip_text(Some("Remove"));
                let weak = self.weak();
                let source = tag.source_id.clone();
                remove.connect_clicked(move |_| {
                    if let Some(panel) = weak.upgrade() {
                        panel.session.remove_tag(&source);
                    }
                });
                chip.append(&remove);
                self.chips.append(&chip);
            }
            self.chips.set_visible(self.chips.first_child().is_some());
        }

        let unavailable = models.stale || models.options.is_empty();
        let name = models.current().map(|c| c.name.clone()).unwrap_or_else(|| {
            if models.stale {
                "Loading models…".to_string()
            } else {
                "Choose a model".to_string()
            }
        });
        if self.model_label.text() != name {
            self.model_label.set_text(&name);
        }
        self.model_button.set_sensitive(!unavailable);
        self.model_button.set_tooltip_text(Some(&if unavailable {
            models.unavailable_reason.clone().unwrap_or_default()
        } else {
            models
                .current()
                .map_or_else(|| "Choose a model".to_string(), |c| c.provider_name.clone())
        }));
        // Rebuilt only while closed, so filtering isn't interrupted by a render.
        let open = self.model_button.popover().is_some_and(|p| p.is_visible());
        if !open && !unavailable {
            self.model_button.set_popover(Some(&self.build_model_popover(models)));
        }
    }
}

fn suggestions(document: Option<&str>) -> Vec<(String, String)> {
    match document {
        Some(name) => vec![
            (
                "Summarize this document".to_string(),
                format!("Summarize {name} in a few bullet points."),
            ),
            (
                "Explain the key ideas".to_string(),
                format!("Explain the key ideas in {name} as if I'm new to the topic."),
            ),
            (
                "Fix spelling and grammar".to_string(),
                format!(
                    "Fix spelling and grammar mistakes in {name}. Don't change the meaning or style."
                ),
            ),
            (
                "Make it more concise".to_string(),
                format!(
                    "Tighten the writing in {name}: cut repetition and filler, keep every point."
                ),
            ),
        ],
        None => vec![
            (
                "What’s in this folder?".to_string(),
                "Give me a quick tour of the documents in this folder.".to_string(),
            ),
            (
                "Find open questions".to_string(),
                "Find open questions and TODOs across these documents.".to_string(),
            ),
        ],
    }
}

/// Nothing is streaming in right now, so show that OpenCode is still working.
fn is_waiting(parts: &[Part]) -> bool {
    match parts.last() {
        None => true,
        Some(Part::Tool(tool)) => tool.state == ToolState::Completed,
        Some(Part::Thinking { done, .. }) => *done,
        Some(Part::Change(change)) => change.status == ChangeStatus::Applied,
        _ => false,
    }
}

fn chip(icon: &str, title: &str, secondary: bool) -> gtk::Box {
    let chip = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    chip.add_css_class("mdow-companion-chip");
    if secondary {
        chip.add_css_class("mdow-companion-chip-secondary");
    }
    if icon.is_empty() {
        chip.append(&label("@", &["mdow-companion-tertiary", "mdow-companion-small-mono"]));
    } else {
        let image = gtk::Image::from_icon_name(icon);
        image.set_pixel_size(10);
        image.add_css_class("mdow-companion-tertiary");
        chip.append(&image);
    }
    let name = label(title, &["mdow-companion-small-mono"]);
    name.set_ellipsize(pango::EllipsizeMode::Middle);
    name.set_max_width_chars(20);
    chip.append(&name);
    chip
}

fn status_pill(change: &ChangePart) -> gtk::Widget {
    if change.applying() {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let spinner = gtk::Spinner::new();
        spinner.start();
        row.append(&spinner);
        row.append(&label("Applying", &["mdow-companion-secondary", "mdow-companion-small"]));
        return row.upcast();
    }
    let (text, class) = match change.status {
        ChangeStatus::Pending => ("Review", "mdow-companion-pill-review"),
        ChangeStatus::Applied => ("Applied", "mdow-companion-pill-applied"),
        ChangeStatus::Rejected => ("Declined", "mdow-companion-pill-declined"),
        ChangeStatus::Failed => ("Failed", "mdow-companion-pill-failed"),
    };
    let pill = label(text, &["mdow-companion-pill", class]);
    pill.set_valign(gtk::Align::Center);
    pill.upcast()
}

fn detail_box(title: &str, text: &str, is_error: bool) -> gtk::Widget {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 3);
    root.add_css_class("mdow-companion-detail");
    root.append(&label(title, &["mdow-companion-tertiary", "mdow-companion-small-mono"]));
    let class = if is_error { "mdow-companion-removed" } else { "mdow-companion-small-mono" };
    let body = label(text, &["mdow-companion-small-mono", class]);
    body.set_selectable(true);
    let scroll = gtk::ScrolledWindow::builder()
        .propagate_natural_height(true)
        .max_content_height(160)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&body)
        .build();
    root.append(&scroll);
    root.upcast()
}

fn show_more_label(shown: bool, total: usize) -> String {
    if shown { "Show Less".to_string() } else { format!("Show All {total} Lines") }
}

fn fill_diff(container: &gtk::Box, hunks: &[logic::DiffHunk], show_all: bool) {
    clear(container);
    let mut remaining = if show_all { usize::MAX } else { DIFF_PREVIEW_LINES };
    for (index, hunk) in hunks.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        if index > 0 {
            let gap = label("⋯", &["mdow-companion-tertiary", "mdow-companion-small-mono"]);
            gap.set_margin_start(34);
            container.append(&gap);
        }
        for line in hunk.lines.iter().take(remaining) {
            container.append(&diff_row(line));
        }
        remaining = remaining.saturating_sub(hunk.lines.len());
    }
}

fn diff_row(line: &logic::DiffLine) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    row.add_css_class("mdow-diff-row");
    let (marker, class) = match line.kind {
        DiffKind::Add => ("+", "mdow-diff-add"),
        DiffKind::Remove => ("−", "mdow-diff-remove"),
        DiffKind::Context => (" ", "mdow-diff-context"),
    };
    row.add_css_class(class);
    let number = line.new_line.or(line.old_line).filter(|_| line.kind != DiffKind::Remove);
    let number = number.or(line.old_line).map(|n| n.to_string()).unwrap_or_default();
    let gutter = label(&number, &["mdow-companion-tertiary", "mdow-companion-small-mono"]);
    gutter.set_size_request(30, -1);
    gutter.set_xalign(1.0);
    gutter.set_margin_end(6);
    gutter.set_valign(gtk::Align::Start);
    row.append(&gutter);
    let mark = label(marker, &["mdow-diff-marker", "mdow-companion-small-mono"]);
    mark.set_size_request(12, -1);
    mark.set_valign(gtk::Align::Start);
    row.append(&mark);
    let text = wrapped(
        if line.text.is_empty() { " " } else { &line.text },
        &["mdow-companion-small-mono", "mdow-diff-text"],
    );
    text.set_hexpand(true);
    row.append(&text);
    row
}

/// The Companion section of Settings: what it runs on, whether it's connected, and the model new
/// conversations start with.
pub fn settings_section() -> gtk::Box {
    let runtime = Runtime::shared();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let status_text = match runtime.status() {
        None => "Starts when you open the companion".to_string(),
        Some(status) if status.availability == RuntimeAvailability::Available => {
            format!("Connected to OpenCode {}", status.version.unwrap_or_default())
                .trim()
                .to_string()
        }
        Some(status) => status.detail.unwrap_or_else(|| "Not connected".to_string()),
    };
    let status = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = label("Status", &["mdow-section-title"]);
    title.set_hexpand(true);
    status.append(&title);
    let detail = wrapped(&status_text, &["mdow-section-text"]);
    detail.set_xalign(1.0);
    detail.set_selectable(false);
    status.append(&detail);
    root.append(&status);

    let model = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.append(&label("Default model", &["mdow-section-title"]));
    text.append(&label("New conversations start with this model.", &["mdow-section-text"]));
    model.append(&text);
    let options = runtime.model_state().options;
    let saved = runtime.default_model_value();
    if options.is_empty() {
        model.append(&label(
            saved.as_deref().unwrap_or("OpenCode’s default"),
            &["mdow-section-text"],
        ));
    } else {
        let mut names = vec!["OpenCode’s default".to_string()];
        names.extend(options.iter().map(|o| format!("{} · {}", o.provider_name, o.name)));
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let dropdown = gtk::DropDown::from_strings(&names);
        dropdown.set_valign(gtk::Align::Center);
        dropdown.set_size_request(180, -1);
        let selected = saved
            .as_ref()
            .and_then(|value| options.iter().position(|o| &o.value == value))
            .map_or(0, |index| index + 1);
        dropdown.set_selected(selected as u32);
        dropdown.connect_selected_notify(move |dropdown| {
            let value = (dropdown.selected() as usize)
                .checked_sub(1)
                .and_then(|index| options.get(index))
                .map(|o| o.value.clone());
            Runtime::shared().set_default_model(value);
        });
        model.append(&dropdown);
    }
    root.append(&model);
    let note = wrapped(
        "The companion runs on OpenCode 2 on this computer and uses the providers you have signed in to there. You can also choose a model from the companion’s message box.",
        &["mdow-section-text"],
    );
    note.set_selectable(false);
    root.append(&note);
    root
}
