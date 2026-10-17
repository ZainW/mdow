//! Blocks → native GTK widgets: selectable Pango labels for text, real check buttons for task
//! lists, grids for tables and pictures for images and diagrams. The look follows the desktop's
//! `markdown.css`.

use crate::doc::{
    Align, Block, CodeChunk, Footnote, Image, ImageSrc, Inline, Item, List, Part, gaps,
};
use crate::highlight::{Span, Tok};
use crate::media;
use crate::theme::Palette;
use gtk::glib::markup_escape_text;
use gtk::{glib, pango, prelude::*};
use std::fmt::Write;
use std::rc::Rc;

pub struct Style {
    pub palette: Rc<Palette>,
    /// Body font size in px (zoom applied).
    pub fs: f64,
    pub code_family: &'static str,
    pub scale_factor: i32,
}

/// A label's Pango markup and the exact plain text it displays (for find).
#[derive(Default)]
pub struct Markup {
    pub markup: String,
    pub text: String,
}

impl Markup {
    fn text(&mut self, s: &str) {
        self.markup.push_str(&markup_escape_text(s));
        self.text.push_str(s);
    }
    fn tag(&mut self, s: &str) {
        self.markup.push_str(s);
    }
}

/// Padding either side of inline code; Pango backgrounds have no padding of their own.
const CODE_PAD: &str = "\u{2006}";

pub fn inline_markup(inlines: &[Inline], style: &Style, m: &mut Markup) {
    for inline in inlines {
        match inline {
            Inline::Text(t) => m.text(t),
            Inline::Code(code) => {
                let _ = write!(
                    m.markup,
                    "<span font_family=\"{}\" size=\"87.5%\" background=\"{}\" fallback=\"true\">",
                    style.code_family,
                    style.palette.muted.hex()
                );
                m.text(CODE_PAD);
                m.text(code);
                m.text(CODE_PAD);
                m.tag("</span>");
            }
            Inline::Strong(c) => wrap(m, "<b>", "</b>", c, style),
            Inline::Emph(c) => wrap(m, "<i>", "</i>", c, style),
            Inline::Strike(c) => wrap(m, "<s>", "</s>", c, style),
            Inline::Underline(c) => wrap(m, "<u>", "</u>", c, style),
            Inline::Mark(c) => {
                let open = format!("<span background=\"{}\">", style.palette.find_match.hex());
                wrap(m, &open, "</span>", c, style)
            }
            Inline::Sup(c) => wrap(m, "<sup>", "</sup>", c, style),
            Inline::Sub(c) => wrap(m, "<sub>", "</sub>", c, style),
            Inline::Link { url, children } => {
                let open = format!("<a href=\"{}\">", markup_escape_text(url));
                wrap(m, &open, "</a>", children, style)
            }
            Inline::Image { alt, .. } => {
                let open = format!("<span foreground=\"{}\">", style.palette.muted_fg.hex());
                m.tag(&open);
                m.text(if alt.is_empty() { "image" } else { alt });
                m.tag("</span>");
            }
            Inline::Break => m.text("\n"),
            Inline::Math(tex) => {
                let _ =
                    write!(m.markup, "<span font_family=\"{}\" size=\"92%\">", style.code_family);
                m.text(tex);
                m.tag("</span>");
            }
            Inline::FootnoteRef(r) => {
                m.tag("<sup><a href=\"#mdow-footnotes\">");
                m.text(r);
                m.tag("</a></sup>");
            }
        }
    }
}

fn wrap(m: &mut Markup, open: &str, close: &str, children: &[Inline], style: &Style) {
    m.tag(open);
    inline_markup(children, style, m);
    m.tag(close);
}

fn markup_of(inlines: &[Inline], style: &Style) -> Markup {
    let mut m = Markup::default();
    inline_markup(inlines, style, &mut m);
    // Trailing hard breaks would leave an empty last line.
    while m.text.ends_with('\n') && m.markup.ends_with('\n') {
        m.text.pop();
        m.markup.pop();
    }
    m
}

fn tok_color(tok: Tok, style: &Style) -> String {
    let s = &style.palette.syntax;
    match tok {
        Tok::Plain => s.fg,
        Tok::Comment => s.comment,
        Tok::Keyword => s.keyword,
        Tok::String => s.string,
        Tok::Entity => s.entity,
        Tok::Constant => s.constant,
    }
    .hex()
}

/// Markup for one chunk of a code block; `spans` index the whole block's text.
pub fn code_markup(chunk: &CodeChunk, spans: Option<&[Span]>, style: &Style) -> String {
    let text = chunk.text();
    let Some(spans) = spans.filter(|s| !s.is_empty()) else {
        return markup_escape_text(text).to_string();
    };
    let base = chunk.range.start as u32;
    let end = chunk.range.end as u32;
    let mut out = String::with_capacity(text.len() * 2);
    let mut at = base;
    let first = spans.partition_point(|s| s.end <= base);
    for span in &spans[first..] {
        if span.start >= end {
            break;
        }
        let s = span.start.max(base);
        let e = span.end.min(end);
        if s > at {
            out.push_str(&markup_escape_text(&chunk.source.text[at as usize..s as usize]));
        }
        let _ = write!(
            out,
            "<span foreground=\"{}\">{}</span>",
            tok_color(span.tok, style),
            markup_escape_text(&chunk.source.text[s as usize..e as usize])
        );
        at = e;
    }
    if at < end {
        out.push_str(&markup_escape_text(&chunk.source.text[at as usize..end as usize]));
    }
    out
}

/// What the builder needs from the reader that owns the widgets.
pub trait Host {
    fn activate_link(&self, uri: &str);
    /// Highlight spans for a code block, if ready; the host starts the work otherwise.
    fn spans(&self, chunk: &CodeChunk) -> Option<Rc<Vec<Span>>>;
}

pub struct Builder<'a> {
    pub style: &'a Style,
    pub host: Rc<dyn Host>,
    /// Searchable labels, in document order.
    pub labels: Vec<gtk::Label>,
}

fn px(v: f64) -> i32 {
    v.round() as i32
}

impl<'a> Builder<'a> {
    pub fn new(style: &'a Style, host: Rc<dyn Host>) -> Self {
        Builder { style, host, labels: Vec::new() }
    }

    fn text_label(&mut self, markup: &str, classes: &[&str]) -> gtk::Label {
        let label = gtk::Label::new(None);
        label.set_markup(markup);
        label.set_wrap(true);
        label.set_wrap_mode(pango::WrapMode::WordChar);
        label.set_natural_wrap_mode(gtk::NaturalWrapMode::Word);
        label.set_xalign(0.0);
        label.set_halign(gtk::Align::Fill);
        label.set_selectable(true);
        for class in classes {
            label.add_css_class(class);
        }
        let host = self.host.clone();
        label.connect_activate_link(move |_, uri| {
            host.activate_link(uri);
            glib::Propagation::Stop
        });
        self.labels.push(label.clone());
        label
    }

    pub fn build(&mut self, block: &Block) -> gtk::Widget {
        match block {
            Block::Heading { level, inlines, .. } => {
                let m = markup_of(inlines, self.style);
                let class = format!("md-h{}", (*level).clamp(1, 6));
                let markup = if *level == 6 {
                    format!("<span text_transform=\"uppercase\">{}</span>", m.markup)
                } else {
                    m.markup
                };
                self.text_label(&markup, &[&class]).upcast()
            }
            Block::Paragraph(inlines) => {
                let m = markup_of(inlines, self.style);
                self.text_label(&m.markup, &["md-p"]).upcast()
            }
            Block::Code(chunk) => self.code(chunk),
            Block::Mermaid(code) => self.mermaid(code.clone()),
            Block::Quote(blocks) => {
                let b = self.children(blocks, ChildMode::Normal);
                b.add_css_class("md-quote");
                b.upcast()
            }
            Block::Alert(kind, blocks) => {
                let b = self.children(blocks, ChildMode::Normal);
                b.add_css_class("md-alert");
                b.add_css_class(kind.css());
                b.upcast()
            }
            Block::List(list) => self.list(list),
            Block::Table(table) => self.table(table),
            Block::Images(images) => self.images(images),
            Block::Rule => {
                let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
                sep.add_css_class("md-hr");
                sep.upcast()
            }
            Block::Math(tex) => {
                let label = self.text_label(&markup_escape_text(tex), &["md-math"]);
                label.set_xalign(0.5);
                label.set_justify(gtk::Justification::Center);
                label.upcast()
            }
            Block::Footnotes(notes) => self.footnotes(notes),
        }
    }

    fn children(&mut self, blocks: &[Block], mode: ChildMode) -> gtk::Box {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let gaps = match mode {
            ChildMode::Normal => gaps(blocks),
            ChildMode::Item { tight } => item_gaps(blocks, tight),
        };
        for (block, gap) in blocks.iter().zip(gaps) {
            let widget = self.build(block);
            widget.set_margin_top(px(gap * self.style.fs));
            container.append(&widget);
        }
        container
    }

    fn code(&mut self, chunk: &CodeChunk) -> gtk::Widget {
        let spans = self.host.spans(chunk);
        let markup = code_markup(chunk, spans.as_deref().map(|v| v.as_slice()), self.style);

        let label = gtk::Label::new(None);
        label.set_markup(&markup);
        label.set_xalign(0.0);
        label.set_yalign(0.0);
        label.set_selectable(true);
        label.add_css_class("md-code-text");
        let top = matches!(chunk.part, Part::Whole | Part::First);
        let bottom = matches!(chunk.part, Part::Whole | Part::Last);
        label.set_margin_top(if top { 14 } else { 0 });
        label.set_margin_bottom(if bottom { 14 } else { 0 });
        label.set_margin_start(18);
        label.set_margin_end(18);
        self.labels.push(label.clone());

        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .child(&label)
            .build();

        let surface = gtk::Box::new(gtk::Orientation::Vertical, 0);
        surface.add_css_class("md-code");
        surface.add_css_class(match chunk.part {
            Part::Whole => "md-part-whole",
            Part::First => "md-part-first",
            Part::Middle => "md-part-middle",
            Part::Last => "md-part-last",
        });
        surface.set_overflow(gtk::Overflow::Hidden);
        surface.append(&scroller);

        let overlay = gtk::Overlay::new();
        overlay.add_css_class("md-code-wrap");
        overlay.set_child(Some(&surface));

        if top {
            let tools = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            tools.set_halign(gtk::Align::End);
            tools.set_valign(gtk::Align::Start);
            tools.set_margin_top(8);
            tools.set_margin_end(8);
            let lang = &chunk.source.lang;
            if !lang.is_empty() {
                let badge = gtk::Label::new(Some(lang));
                badge.add_css_class("md-code-lang");
                badge.set_valign(gtk::Align::Center);
                badge.set_can_target(false);
                tools.append(&badge);
            }
            let copy = gtk::Button::from_icon_name("edit-copy-symbolic");
            copy.add_css_class("md-copy");
            copy.set_tooltip_text(Some("Copy code"));
            let source = chunk.source.clone();
            copy.connect_clicked(move |button| {
                button.clipboard().set_text(&source.text);
                button.set_icon_name("object-select-symbolic");
                button.add_css_class("md-copied");
                button.set_tooltip_text(Some("Copied"));
                let weak = button.downgrade();
                glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
                    if let Some(button) = weak.upgrade() {
                        button.set_icon_name("edit-copy-symbolic");
                        button.remove_css_class("md-copied");
                        button.set_tooltip_text(Some("Copy code"));
                    }
                });
            });
            tools.append(&copy);
            overlay.add_overlay(&tools);

            let motion = gtk::EventControllerMotion::new();
            let weak = overlay.downgrade();
            motion.connect_enter(move |_, _, _| {
                if let Some(o) = weak.upgrade() {
                    o.add_css_class("mdow-hover");
                }
            });
            let weak = overlay.downgrade();
            motion.connect_leave(move |_| {
                if let Some(o) = weak.upgrade() {
                    o.remove_css_class("mdow-hover");
                }
            });
            overlay.add_controller(motion);
        }
        overlay.upcast()
    }

    fn mermaid(&mut self, code: std::sync::Arc<str>) -> gtk::Widget {
        let picture = gtk::Picture::new();
        picture.set_can_shrink(false);
        picture.set_halign(gtk::Align::Center);
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .child(&picture)
            .build();
        let stack = gtk::Stack::new();
        stack.set_vhomogeneous(false);
        stack.set_hhomogeneous(false);
        stack.set_interpolate_size(false);
        stack.add_named(&scroller, Some("diagram"));
        let weak_picture = picture.downgrade();
        let weak_stack = stack.downgrade();
        media::render_mermaid(code, &self.style.palette, self.style.scale_factor, move |result| {
            let (Some(picture), Some(stack)) = (weak_picture.upgrade(), weak_stack.upgrade())
            else {
                return;
            };
            match result {
                Ok(paintable) => picture.set_paintable(Some(&paintable)),
                Err(err) => {
                    let label = gtk::Label::new(Some(&format!(
                        "Mermaid diagram could not be rendered:\n{err}"
                    )));
                    label.set_wrap(true);
                    label.set_xalign(0.0);
                    label.set_selectable(true);
                    label.add_css_class("md-mermaid-error");
                    stack.add_named(&label, Some("error"));
                    stack.set_visible_child_name("error");
                }
            }
        });
        stack.upcast()
    }

    fn list(&mut self, list: &List) -> gtk::Widget {
        let fs = self.style.fs;
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let is_task_list = list.items.iter().all(|i| i.task.is_some()) && !list.items.is_empty();
        for (i, item) in list.items.iter().enumerate() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            if i > 0 {
                row.set_margin_top(px(0.35 * fs));
            }
            match item.task {
                Some(checked) => {
                    let check = gtk::CheckButton::new();
                    check.set_active(checked);
                    check.set_can_target(false);
                    check.set_can_focus(false);
                    check.set_valign(gtk::Align::Start);
                    // Centre the box on the first line of text.
                    let line = fs * crate::theme::READER_LINE_HEIGHT;
                    check.set_margin_top(px((line - 16.0).max(0.0) / 2.0));
                    check.add_css_class("md-task");
                    check.set_margin_end(px(0.5 * fs));
                    if !is_task_list {
                        check.set_margin_start(px(0.25 * fs));
                    }
                    row.append(&check);
                }
                None => {
                    let marker = gtk::Label::new(Some(&marker_text(list, i)));
                    marker.add_css_class("md-marker");
                    marker.set_xalign(1.0);
                    marker.set_yalign(0.0);
                    marker.set_valign(gtk::Align::Start);
                    marker.set_size_request(px(1.25 * fs), -1);
                    marker.set_margin_end(px(0.35 * fs));
                    marker.set_can_target(false);
                    row.append(&marker);
                }
            }
            let content = self.item(item, list.tight);
            content.set_hexpand(true);
            row.append(&content);
            container.append(&row);
        }
        container.upcast()
    }

    fn item(&mut self, item: &Item, tight: bool) -> gtk::Box {
        self.children(&item.blocks, ChildMode::Item { tight })
    }

    fn table(&mut self, table: &crate::doc::Table) -> gtk::Widget {
        let grid = gtk::Grid::new();
        grid.set_hexpand(true);
        let cols = table.head.len().max(table.rows.iter().map(|r| r.len()).max().unwrap_or(0));
        let n_rows = table.rows.len();
        // Narrow tables wrap their cells to the column, like a browser's auto layout. Wide ones
        // keep cells on one line and scroll sideways: GTK can't trade height for width inside a
        // horizontal scroller, so wrapping and scrolling don't mix.
        let longest = std::iter::once(&table.head)
            .chain(table.rows.iter())
            .flat_map(|row| row.iter())
            .map(|cell| crate::doc::inline_text(cell).chars().count())
            .max()
            .unwrap_or(0);
        let scrolls = cols > 4 && longest <= 48;
        let place = |b: &mut Builder, inlines: &[Inline], row: usize, col: usize, header: bool| {
            let mut m = markup_of(inlines, b.style);
            if header {
                m.markup = format!("<span text_transform=\"uppercase\">{}</span>", m.markup);
            }
            let label = b.text_label(&m.markup, &[]);
            label.set_wrap(!scrolls);
            let align = table.aligns.get(col).copied().unwrap_or(Align::None);
            let (x, justify) = match align {
                Align::Center => (0.5, gtk::Justification::Center),
                Align::Right => (1.0, gtk::Justification::Right),
                _ => (0.0, gtk::Justification::Left),
            };
            label.set_xalign(x);
            label.set_justify(justify);
            label.set_valign(gtk::Align::Start);
            let cell = gtk::Box::new(gtk::Orientation::Vertical, 0);
            cell.add_css_class("md-cell");
            cell.add_css_class(if header { "md-th" } else { "md-td" });
            if col + 1 < cols {
                cell.add_css_class("md-cell-right");
            }
            if !header && row + 1 < n_rows {
                cell.add_css_class("md-cell-below");
            }
            cell.set_hexpand(true);
            cell.append(&label);
            grid.attach(&cell, col as i32, (if header { 0 } else { row + 1 }) as i32, 1, 1);
        };
        for col in 0..cols {
            let inlines = table.head.get(col).map(Vec::as_slice).unwrap_or(&[]);
            place(self, inlines, 0, col, true);
        }
        for (r, cells) in table.rows.iter().enumerate() {
            for col in 0..cols {
                let inlines = cells.get(col).map(Vec::as_slice).unwrap_or(&[]);
                place(self, inlines, r, col, false);
            }
        }
        let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
        frame.add_css_class("md-table");
        frame.set_overflow(gtk::Overflow::Hidden);
        if scrolls {
            let scroller = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Automatic)
                .vscrollbar_policy(gtk::PolicyType::Never)
                .propagate_natural_height(true)
                .child(&grid)
                .build();
            frame.append(&scroller);
        } else {
            frame.append(&grid);
        }
        frame.upcast()
    }

    fn images(&mut self, images: &[Image]) -> gtk::Widget {
        if let [image] = images {
            return self.image(image, false);
        }
        let wrap = adw::WrapBox::new();
        wrap.set_child_spacing(6);
        wrap.set_line_spacing(6);
        for image in images {
            wrap.append(&self.image(image, true));
        }
        wrap.upcast()
    }

    fn image(&mut self, image: &Image, inline: bool) -> gtk::Widget {
        let picture = gtk::Picture::new();
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::ScaleDown);
        picture.set_halign(gtk::Align::Start);
        picture.set_valign(gtk::Align::Start);
        picture.add_css_class("md-image");
        picture.set_overflow(gtk::Overflow::Hidden);
        if !image.alt.is_empty() {
            picture.set_alternative_text(Some(&image.alt));
            picture.set_tooltip_text(Some(&image.alt));
        }
        let stack = gtk::Stack::new();
        stack.set_hhomogeneous(false);
        stack.set_vhomogeneous(false);
        stack.set_halign(gtk::Align::Start);
        stack.add_named(&picture, Some("image"));
        if let Some(link) = image.link.clone() {
            let click = gtk::GestureClick::new();
            let host = self.host.clone();
            click.connect_released(move |gesture, _, _, _| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                host.activate_link(&link);
            });
            stack.add_controller(click);
            stack.set_cursor_from_name(Some("pointer"));
        }
        let alt = image.alt.clone();
        let missing = matches!(image.src, ImageSrc::Missing);
        let weak_picture = picture.downgrade();
        let weak_stack = stack.downgrade();
        let show_missing = move |stack: &gtk::Stack| {
            let text = if alt.is_empty() { "Image unavailable".to_string() } else { alt.clone() };
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            row.add_css_class("md-image-missing");
            row.append(&gtk::Image::from_icon_name("image-missing-symbolic"));
            let label = gtk::Label::new(Some(&text));
            label.set_wrap(true);
            label.set_xalign(0.0);
            row.append(&label);
            stack.add_named(&row, Some("missing"));
            stack.set_visible_child_name("missing");
        };
        if missing {
            show_missing(&stack);
        } else {
            media::load_image(&image.src, self.style.scale_factor, move |result| {
                let (Some(picture), Some(stack)) = (weak_picture.upgrade(), weak_stack.upgrade())
                else {
                    return;
                };
                match result {
                    Ok(paintable) => {
                        if inline {
                            // Badges and inline icons keep their own size.
                            picture.set_can_shrink(false);
                        }
                        picture.set_paintable(Some(&paintable));
                    }
                    Err(_) => show_missing(&stack),
                }
            });
        }
        stack.upcast()
    }

    fn footnotes(&mut self, notes: &[Footnote]) -> gtk::Widget {
        let fs = self.style.fs;
        let container = gtk::Box::new(gtk::Orientation::Vertical, px(0.35 * fs));
        container.add_css_class("md-footnotes");
        let rule = gtk::Separator::new(gtk::Orientation::Horizontal);
        rule.add_css_class("md-footnotes-rule");
        rule.set_margin_bottom(px(0.75 * fs));
        container.append(&rule);
        for note in notes {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            let marker = gtk::Label::new(Some(&format!("{}.", note.label)));
            marker.add_css_class("md-marker");
            marker.set_xalign(1.0);
            marker.set_valign(gtk::Align::Start);
            marker.set_size_request(px(1.25 * fs), -1);
            marker.set_margin_end(px(0.35 * fs));
            row.append(&marker);
            let content = self.children(&note.blocks, ChildMode::Item { tight: true });
            content.set_hexpand(true);
            row.append(&content);
            container.append(&row);
        }
        container.upcast()
    }
}

#[derive(Clone, Copy)]
enum ChildMode {
    Normal,
    Item { tight: bool },
}

/// Gaps inside a list item: `li > p` keeps half a line below (none in tight lists), and nested
/// lists sit 0.4em under their parent text.
fn item_gaps(blocks: &[Block], tight: bool) -> Vec<f64> {
    let margins = |b: &Block| match b {
        Block::Paragraph(_) => (0.0, if tight { 0.0 } else { 0.5 }),
        Block::List(_) => (0.4, 0.0),
        other => other.margins(),
    };
    let mut out = Vec::with_capacity(blocks.len());
    let mut prev: Option<f64> = None;
    for block in blocks {
        let (top, bottom) = margins(block);
        out.push(prev.map_or(0.0, |p| p.max(top)));
        prev = Some(bottom);
    }
    out
}

fn to_alpha(mut n: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push((b'a' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.iter().rev().collect()
}

fn to_roman(mut n: usize) -> String {
    if n == 0 {
        return "0".into();
    }
    const TABLE: [(usize, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (value, numeral) in TABLE {
        while n >= value {
            out.push_str(numeral);
            n -= value;
        }
    }
    out
}

/// CSS list markers: disc / circle / square by depth, and decimal / lower-alpha / lower-roman.
pub fn marker_text(list: &List, index: usize) -> String {
    if list.ordered {
        let n = list.start + index;
        match list.depth % 3 {
            0 => format!("{n}."),
            1 => format!("{}.", to_alpha(n.saturating_sub(1))),
            _ => format!("{}.", to_roman(n)),
        }
    } else {
        match list.depth % 3 {
            0 => "•",
            1 => "◦",
            _ => "▪",
        }
        .to_string()
    }
}

/// The plain text of every searchable label a block builds, in the order `Builder::build`
/// creates them.
pub fn label_texts(block: &Block, style: &Style, out: &mut Vec<String>) {
    match block {
        Block::Heading { inlines, .. } | Block::Paragraph(inlines) => {
            out.push(markup_of(inlines, style).text)
        }
        Block::Code(chunk) => out.push(chunk.text().to_string()),
        Block::Quote(blocks) | Block::Alert(_, blocks) => {
            for b in blocks {
                label_texts(b, style, out);
            }
        }
        Block::List(list) => {
            for item in &list.items {
                for b in &item.blocks {
                    label_texts(b, style, out);
                }
            }
        }
        Block::Table(table) => {
            let cols = table.head.len().max(table.rows.iter().map(|r| r.len()).max().unwrap_or(0));
            for col in 0..cols {
                let inlines = table.head.get(col).map(Vec::as_slice).unwrap_or(&[]);
                out.push(markup_of(inlines, style).text);
            }
            for cells in &table.rows {
                for col in 0..cols {
                    let inlines = cells.get(col).map(Vec::as_slice).unwrap_or(&[]);
                    out.push(markup_of(inlines, style).text);
                }
            }
        }
        Block::Math(tex) => out.push(tex.clone()),
        Block::Footnotes(notes) => {
            for note in notes {
                for b in &note.blocks {
                    label_texts(b, style, out);
                }
            }
        }
        Block::Mermaid(_) | Block::Images(_) | Block::Rule => {}
    }
}

/// Find-match highlight attributes for one label.
pub fn match_attrs(ranges: &[(u32, u32, bool)], palette: &Palette) -> pango::AttrList {
    let list = pango::AttrList::new();
    for &(start, end, active) in ranges {
        let bg = if active { palette.find_active } else { palette.find_match };
        let (r, g, b) = bg.u16s();
        let mut attr = pango::AttrColor::new_background(r, g, b);
        attr.set_start_index(start);
        attr.set_end_index(end);
        list.insert(attr);
        let fg = if active { Some(palette.find_active_fg) } else { palette.find_match_fg };
        if let Some(fg) = fg {
            let (r, g, b) = fg.u16s();
            let mut attr = pango::AttrColor::new_foreground(r, g, b);
            attr.set_start_index(start);
            attr.set_end_index(end);
            list.insert(attr);
        }
    }
    list
}

/// Byte ranges of case-insensitive matches of `needle` in `hay`.
pub fn find_all(hay: &str, needle: &str) -> Vec<(usize, usize)> {
    let needle: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if hay.is_ascii() && needle.iter().all(char::is_ascii) {
        let hay_b = hay.as_bytes();
        let n: Vec<u8> = needle.iter().map(|c| *c as u8).collect();
        let mut i = 0;
        while i + n.len() <= hay_b.len() {
            if hay_b[i..i + n.len()].iter().zip(&n).all(|(a, b)| a.to_ascii_lowercase() == *b) {
                out.push((i, i + n.len()));
                i += n.len();
            } else {
                i += 1;
            }
        }
        return out;
    }
    let chars: Vec<(usize, char)> = hay.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let mut j = i;
        let mut k = 0;
        'outer: while j < chars.len() && k < needle.len() {
            for lc in chars[j].1.to_lowercase() {
                if k >= needle.len() || lc != needle[k] {
                    break 'outer;
                }
                k += 1;
            }
            j += 1;
        }
        if k == needle.len() {
            let start = chars[i].0;
            let end = chars.get(j).map_or(hay.len(), |c| c.0);
            out.push((start, end));
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::parse_markdown;
    use std::path::Path;

    fn style() -> Style {
        Style {
            palette: Rc::new(Palette::light()),
            fs: 15.5,
            code_family: "Geist Mono",
            scale_factor: 1,
        }
    }

    #[test]
    fn markup_text_matches_pango_parse() {
        let doc = parse_markdown(
            "Some **bold**, `code` & <i>html</i> [link](https://a.b?x=1&y=2)\nnext line[^1]\n\n[^1]: note",
            Path::new("/a.md"),
        );
        let Block::Paragraph(inlines) = &doc.blocks[0] else { panic!() };
        let m = markup_of(inlines, &style());
        // GtkLabel handles <a> itself; Pango alone doesn't know the tag.
        let mut plain = m.markup.replace("</a>", "");
        while let Some(start) = plain.find("<a ") {
            let end = start + plain[start..].find('>').unwrap();
            plain.replace_range(start..=end, "");
        }
        let (_, parsed, _) = pango::parse_markup(&plain, '\0').unwrap();
        assert_eq!(parsed.as_str(), m.text);
        assert!(m.text.contains("\u{2006}code\u{2006}"));
    }

    #[test]
    fn code_markup_keeps_text_and_colors_tokens() {
        let doc = parse_markdown("```rust\nfn main() {} // <hi>\n```\n", Path::new("/a.md"));
        let Block::Code(chunk) = &doc.blocks[0] else { panic!() };
        let spans = crate::highlight::highlight("rust", &chunk.source.text);
        let markup = code_markup(chunk, Some(&spans), &style());
        let (_, parsed, _) = pango::parse_markup(&markup, '\0').unwrap();
        assert_eq!(parsed.as_str(), chunk.text());
        assert!(markup.contains(&style().palette.syntax.keyword.hex()));
    }

    #[test]
    fn markers_follow_css_list_styles() {
        let list = |ordered, depth, start| List {
            ordered,
            start,
            tight: true,
            depth,
            items: vec![],
            cont_before: false,
            cont_after: false,
        };
        assert_eq!(marker_text(&list(true, 0, 1), 2), "3.");
        assert_eq!(marker_text(&list(true, 1, 1), 1), "b.");
        assert_eq!(marker_text(&list(true, 2, 1), 3), "iv.");
        assert_eq!(marker_text(&list(false, 1, 1), 0), "◦");
        assert_eq!(to_alpha(26), "aa");
    }

    #[test]
    fn find_is_case_insensitive_with_byte_offsets() {
        assert_eq!(find_all("Hello hello HELLO", "hello"), [(0, 5), (6, 11), (12, 17)]);
        assert_eq!(find_all("Ünïcode ünÏ", "ÜNÏ"), [(0, 5), (10, 15)]);
        assert!(find_all("abc", "").is_empty());
    }
}
