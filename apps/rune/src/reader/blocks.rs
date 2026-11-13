//! Blocks to Rune elements. Every text goes through [`Ctx::text`], which numbers it, so find can
//! address a match as (row, text, range): [`texts`] walks the blocks in the same order.

use super::inline::{Flat, InlineStyle, flatten};
use crate::app::App;
use crate::doc::{Align as CellAlign, Block, Image, ImageSrc, Inline, List, Part, Table};
use crate::highlight::{self, Tok};
use crate::media::{self, Asset, Media};
use crate::theme::{self, Palette};
use rune::{
    Align, Color, El, Family, Link, LinkPaint, Span, TextHighlight, TextStyle, col, el, icon, rich,
    row, text,
};
use std::cell::Cell;
use std::ops::Range;
use std::path::Path;

/// How a document is set: sizes in points, at the current zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadStyle {
    pub size: f32,
    pub body_family: &'static str,
    pub code_family: Family,
    pub line: f32,
    /// The text column's width.
    pub column: f32,
}

impl ReadStyle {
    pub fn code_size(&self) -> f32 {
        (self.size * 0.86).round()
    }
    pub fn block_spacing(&self) -> f32 {
        (self.size * 0.85).round()
    }
    pub fn body(&self, weight: f32) -> TextStyle {
        TextStyle {
            family: Family::Named(self.body_family),
            line_height: (self.size * self.line).round(),
            ..TextStyle::new(self.size, weight)
        }
    }
    pub fn code(&self, size: f32) -> TextStyle {
        TextStyle {
            family: self.code_family,
            line_height: (size * 1.18 * 1.2).round(),
            features: "'liga' 0, 'calt' 0",
            ..TextStyle::new(size, 400.0)
        }
    }
    pub fn heading_size(&self, level: u8) -> f32 {
        let scale = match level {
            1 => 2.0,
            2 => 1.45,
            3 => 1.2,
            4 => 1.06,
            _ => 1.0,
        };
        (self.size * scale).round()
    }
}

/// A find match inside one row: which of its texts, the byte range, and whether it's current.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub text: usize,
    pub range: Range<usize>,
    pub current: bool,
}

pub struct Ctx<'a> {
    pub p: &'a Palette,
    pub st: &'a ReadStyle,
    pub pane: u64,
    pub doc_path: &'a Path,
    pub media: &'a Media,
    pub hits: &'a [Hit],
    pub texts: Cell<usize>,
    /// The code block the pointer is over, for its copy button.
    pub copied: Option<usize>,
}

impl Ctx<'_> {
    fn skip_text(&self) {
        self.texts.set(self.texts.get() + 1);
    }

    fn inline_style(&self, size: f32, weight: f32) -> InlineStyle<'_> {
        InlineStyle { p: self.p, size, code_family_mono: true, base_weight: weight }
    }

    /// This text's find matches: (range, current).
    fn hits_for(&self, index: usize) -> Vec<(Range<usize>, bool)> {
        self.hits.iter().filter(|h| h.text == index).map(|h| (h.range.clone(), h.current)).collect()
    }

    /// A selectable text with its links, strikes and find highlights.
    pub fn text(&self, flat: Flat, style: TextStyle, color: Color) -> El {
        let index = self.texts.get();
        self.texts.set(index + 1);
        let hits = self.hits_for(index);
        self.text_el(flat, style, color, &hits)
    }

    fn text_el(
        &self,
        flat: Flat,
        style: TextStyle,
        color: Color,
        hits: &[(Range<usize>, bool)],
    ) -> El {
        let has_links = !flat.links.is_empty();
        let mut e = rich(flat.text, flat.spans).font(style).color(color).wrap();
        for (range, url) in flat.links {
            e = e.link(
                Link::new(range, url.clone())
                    .hover(LinkPaint::new().underline(self.p.accent))
                    .help(url),
            );
        }
        if has_links {
            let pane = self.pane;
            e = e
                .on_link(move |s: &mut App, id: &str, tx: &mut rune::Tx| s.open_link(pane, id, tx));
        }
        for r in flat.strikes {
            e = e.strike(r.start, r.end, None);
        }
        e = e.selectable(true).selection_color(self.p.selection);
        for r in flat.uncopied {
            e = e.uncopied(r);
        }
        let highlights: Vec<TextHighlight> = hits
            .iter()
            .map(|(range, current)| {
                let bg = if *current { self.p.find_current } else { self.p.find_match };
                let hl = TextHighlight::new(range.clone(), bg).pad(1.0);
                if *current { hl.key("find-current") } else { hl }
            })
            .collect();
        if highlights.is_empty() { e } else { e.text_highlights(highlights) }
    }

    fn inlines(&self, inlines: &[Inline], size: f32, weight: f32, color: Color, line: f32) -> El {
        let flat = flatten(inlines, &self.inline_style(size, weight));
        let style = TextStyle {
            family: Family::Named(self.st.body_family),
            line_height: line.round(),
            ..TextStyle::new(size, weight)
        };
        let typeset = flat.maths.iter().any(|(_, tex)| {
            let key = media::inline_math_key(tex, self.p.id, size);
            matches!(self.media.get(&key), Some(Asset::Ready(..)))
        });
        if typeset { self.flow(flat, style, color) } else { self.text(flat, style, color) }
    }

    /// Text with typeset inline formulas: its words and formulas flow and wrap together, each
    /// formula sitting on the text's baseline. It counts as one text for find, and copies as one.
    fn flow(&self, flat: Flat, style: TextStyle, color: Color) -> El {
        let index = self.texts.get();
        self.texts.set(index + 1);
        let hits = self.hits_for(index);
        let size = style.size;
        let line = if style.line_height > 0.0 { style.line_height } else { size * 1.25 };
        // From the bottom of a line's box up to its baseline.
        let baseline = (line - size) / 2.0 + size * 0.17;
        // Pieces: a word (with the space after it) or a formula.
        let mut pieces: Vec<(Range<usize>, Option<&str>)> = Vec::new();
        let mut at = 0;
        let text = &flat.text;
        let words = |from: usize, to: usize, pieces: &mut Vec<(Range<usize>, Option<&str>)>| {
            let mut start = from;
            for (i, c) in text[from..to].char_indices() {
                let i = from + i;
                if c.is_whitespace() {
                    let end = i + c.len_utf8();
                    pieces.push((start..end, None));
                    start = end;
                }
            }
            if start < to {
                pieces.push((start..to, None));
            }
        };
        for (range, tex) in &flat.maths {
            words(at, range.start, &mut pieces);
            pieces.push((range.clone(), Some(tex.as_str())));
            at = range.end;
        }
        words(at, text.len(), &mut pieces);

        let clip = |r: &Range<usize>, piece: &Range<usize>| -> Option<Range<usize>> {
            let start = r.start.max(piece.start);
            let end = r.end.min(piece.end);
            (start < end).then(|| start - piece.start..end - piece.start)
        };
        let mut row = row().w_full().flex_wrap().items_end();
        for (piece, tex) in pieces {
            if let Some(tex) = tex {
                let key = media::inline_math_key(tex, self.p.id, size);
                if let Some(Asset::Ready(image, [w, h])) = self.media.get(&key) {
                    let depth = self.media.depth(&key);
                    row = row.child(
                        el().size(*w, *h)
                            .pixels(image.pixels())
                            .offset(0.0, depth - baseline)
                            .label(tex.to_string()),
                    );
                    continue;
                }
            }
            let mut sub = Flat { text: text[piece.clone()].to_string(), ..Default::default() };
            for s in &flat.spans {
                if let Some(r) = clip(&(s.start as usize..s.end as usize), &piece) {
                    let mut s = s.clone();
                    (s.start, s.end) = (r.start as u32, r.end as u32);
                    sub.spans.push(s);
                }
            }
            for (r, url) in &flat.links {
                if let Some(r) = clip(r, &piece) {
                    sub.links.push((r, url.clone()));
                }
            }
            sub.strikes = flat.strikes.iter().filter_map(|r| clip(r, &piece)).collect();
            sub.uncopied = flat.uncopied.iter().filter_map(|r| clip(r, &piece)).collect();
            let piece_hits: Vec<(Range<usize>, bool)> =
                hits.iter().filter_map(|(r, cur)| clip(r, &piece).map(|r| (r, *cur))).collect();
            let word = sub.text.ends_with('\n');
            if word {
                sub.text.pop();
                let len = sub.text.len();
                for s in &mut sub.spans {
                    s.end = s.end.min(len as u32);
                }
                for (r, _) in &mut sub.links {
                    r.end = r.end.min(len);
                }
            }
            row = row.child(self.text_el(sub, style, color, &piece_hits));
            if word {
                // A line break: the rest starts on a new line.
                row = row.child(el().w_full().h(0.0));
            }
        }
        // Copy the words as the text they are, not as cells side by side.
        row.selection_copy(|sel| {
            Some(
                sel.texts()
                    .iter()
                    .filter(|t| t.is_selected())
                    .map(|t| t.text())
                    .collect::<String>(),
            )
        })
    }

    fn paragraph(&self, inlines: &[Inline], color: Color) -> El {
        self.inlines(inlines, self.st.size, 400.0, color, self.st.size * self.st.line)
    }

    pub fn block(&self, b: &Block, width: f32, color: Color, list_depth: usize) -> El {
        let p = self.p;
        let st = self.st;
        match b {
            Block::Heading { level, inlines, .. } => {
                let size = st.heading_size(*level);
                let weight = if *level <= 2 { 700.0 } else { 600.0 };
                let color = if *level == 6 { p.secondary } else { color };
                self.inlines(inlines, size, weight, color, size * 1.12 * 1.2)
            }
            Block::Paragraph(inlines) => self.paragraph(inlines, color),
            Block::Code(chunk) => self.code(
                chunk.text(),
                &chunk.source.lang,
                chunk.part,
                chunk.source.id,
                &chunk.source.text,
            ),
            Block::Mermaid(code) => match self.media.get(&media::mermaid_key(code, p.id)) {
                Some(Asset::Ready(image, size)) => {
                    self.skip_text();
                    picture(image, *size, width).self_center()
                }
                Some(Asset::Failed(_)) => self.code(code, "mermaid", Part::Whole, usize::MAX, code),
                _ => {
                    self.skip_text();
                    placeholder(p, "Drawing diagram…", 120.0, width)
                }
            },
            Block::Math(tex) => match self.media.get(&media::math_key(tex, p.id, st.size * 1.1)) {
                Some(Asset::Ready(image, size)) => {
                    self.skip_text();
                    let scaled = fit(*size, width);
                    row().w_full().justify_center().child(picture(image, scaled, width))
                }
                Some(Asset::Failed(_)) => self.code(tex, "latex", Part::Whole, usize::MAX, tex),
                _ => {
                    let flat = Flat { text: tex.trim().to_string(), ..Default::default() };
                    row().w_full().justify_center().child(self.text(
                        flat,
                        st.code(st.code_size()),
                        p.secondary,
                    ))
                }
            },
            Block::Quote(blocks) => {
                let inner = self.blocks(blocks, width - 21.0, p.secondary, list_depth);
                row()
                    .w_full()
                    .child(el().w(3.0).radius(1.5).fill(p.quote_bar))
                    .child(inner.pl(18.0).grow(1.0).min_w(0.0).py(2.0))
            }
            Block::Alert(kind, blocks) => {
                let (tint, label) = p.alert(*kind);
                let title = row()
                    .gap(6.0)
                    .center_y()
                    .child(icon(alert_icon(*kind), (st.size * 0.95).round()).color(tint))
                    .child(
                        text(label.to_uppercase())
                            .font(TextStyle {
                                tracking: 0.6,
                                ..theme::mono((st.size * 0.72).round(), 600.0)
                            })
                            .color(tint),
                    );
                let inner = self.blocks(blocks, width - 37.0, color, list_depth);
                row()
                    .w_full()
                    .radius(6.0)
                    .fill(tint.alpha(if p.dark { 0.09 } else { 0.06 }))
                    .child(el().w(3.0).corners([6.0, 0.0, 0.0, 6.0]).fill(tint))
                    .child(
                        col()
                            .grow(1.0)
                            .min_w(0.0)
                            .pl(15.0)
                            .pr(16.0)
                            .py(12.0)
                            .gap(6.0)
                            .child(title)
                            .child(inner),
                    )
            }
            Block::List(list) => self.list(list, width, color, list_depth),
            Block::Table(table) => self.table(table, width, color),
            Block::Images(images) => self.images(images, width),
            Block::Rule => el().w_full().h(1.0).fill(p.strong_line),
            Block::Footnotes(notes) => {
                let mut c =
                    col().w_full().gap(6.0).child(el().w_full().h(1.0).fill(p.line).mb(10.0));
                let small = (st.size * 0.86).round();
                for note in notes {
                    let st_small = ReadStyle { size: small, ..*st };
                    let ctx = Ctx { st: &st_small, ..self.reborrow() };
                    let body = ctx.blocks(&note.blocks, width - 28.0, p.secondary, list_depth);
                    self.texts.set(ctx.texts.get());
                    c = c.child(
                        row()
                            .key(format!("fn-{}-{}", self.pane, note.label))
                            .w_full()
                            .items_start()
                            .child(
                                text(format!("{}.", note.label))
                                    .font(theme::ui(small, 500.0))
                                    .color(p.tertiary)
                                    .w(28.0)
                                    .pt(2.0),
                            )
                            .child(body.grow(1.0).min_w(0.0))
                            .child(back_link(self.pane, &note.label, small, p)),
                    );
                }
                c
            }
        }
    }

    fn reborrow(&self) -> Ctx<'_> {
        Ctx {
            p: self.p,
            st: self.st,
            pane: self.pane,
            doc_path: self.doc_path,
            media: self.media,
            hits: self.hits,
            texts: Cell::new(self.texts.get()),
            copied: self.copied,
        }
    }

    /// Blocks inside a quote, list item or footnote, spaced as at the top level.
    pub fn blocks(&self, blocks: &[Block], width: f32, color: Color, list_depth: usize) -> El {
        self.blocks_scaled(blocks, width, color, list_depth, 0.85)
    }

    /// `blocks`, with the gaps between them `scale` times the body size per unit.
    fn blocks_scaled(
        &self,
        blocks: &[Block],
        width: f32,
        color: Color,
        list_depth: usize,
        scale: f32,
    ) -> El {
        let gaps = crate::doc::gaps(blocks);
        let mut c = col().w_full().min_w(0.0);
        for (b, gap) in blocks.iter().zip(gaps) {
            let gap = (gap as f32 * self.st.size * scale).round();
            c = c.child(self.block(b, width, color, list_depth).mt(gap));
        }
        c
    }

    fn code(&self, code: &str, lang: &str, part: Part, id: usize, whole: &str) -> El {
        let p = self.p;
        let st = self.st;
        let size = st.code_size();
        let mut spans = Vec::new();
        if code.len() < 400_000 {
            for s in highlight::highlight(lang, code).iter() {
                if let Some(color) = p.syntax(s.tok) {
                    let mut span = Span::new(s.start as usize, s.end as usize).color(color);
                    if s.tok == Tok::Keyword {
                        span = span.weight(500.0);
                    }
                    spans.push(span);
                }
            }
        }
        let shown = if code.is_empty() { " ".to_string() } else { code.to_string() };
        let body =
            self.text(Flat { text: shown, spans, ..Default::default() }, st.code(size), p.text);
        let (top, bottom) = match part {
            Part::Whole => (true, true),
            Part::First => (true, false),
            Part::Middle => (false, false),
            Part::Last => (false, true),
        };
        let r = 8.0;
        let mut block = col()
            .w_full()
            .fill(p.well)
            .corners([
                if top { r } else { 0.0 },
                if top { r } else { 0.0 },
                if bottom { r } else { 0.0 },
                if bottom { r } else { 0.0 },
            ])
            .border_sides(
                [if top { 1.0 } else { 0.0 }, 1.0, if bottom { 1.0 } else { 0.0 }, 1.0],
                p.line,
            )
            .px(14.0)
            .pt(if top { 12.0 } else { 0.0 })
            .pb(if bottom { 12.0 } else { 0.0 })
            .scroll_x()
            .child(body);
        if top && id != usize::MAX {
            let label = crate::highlight::display_name(lang);
            let copied = self.copied == Some(id);
            let source = whole.to_string();
            let copy = row()
                .key(format!("copy-{}-{id}", self.pane))
                .gap(4.0)
                .center_y()
                .px(7.0)
                .h(22.0)
                .radius(5.0)
                .fill(p.elevated)
                .border(1.0, p.line)
                .hover(p.hover)
                .cursor(rune::Cursor::Pointer)
                .label("Copy code")
                .child(
                    icon(if copied { &crate::icons::CHECK } else { &crate::icons::COPY }, 13.0)
                        .color(p.secondary),
                )
                .child(
                    text(if copied { "Copied" } else { "Copy" })
                        .font(theme::ui(11.0, 500.0))
                        .color(p.secondary),
                )
                .on_press_tx(move |s: &mut App, tx: &mut rune::Tx| {
                    tx.copy(source.clone());
                    s.copied_code = Some((id, s.clock));
                    tx.wake_after(std::time::Duration::from_millis(1600));
                });
            let mut bar =
                row().absolute(0.0, 6.0).w_full().pr(8.0).justify_end_items().gap(8.0).center_y();
            if let Some(label) = label {
                bar = bar.child(
                    text(label)
                        .font(theme::mono(11.0, 500.0))
                        .color(p.tertiary)
                        .group_opacity(1.0, 0.0),
                );
            }
            bar = bar.child(copy.group_opacity(if copied { 1.0 } else { 0.0 }, 1.0));
            let whole_code = whole.to_string();
            block = block.hover_group().child(bar).on_secondary_press(
                move |s: &mut App, press, tx: &mut rune::Tx| {
                    s.reader_menu(Some(whole_code.clone()), press, tx)
                },
            );
        }
        block
    }

    fn list(&self, list: &List, width: f32, color: Color, depth: usize) -> El {
        let p = self.p;
        let st = self.st;
        let marker_w = if list.ordered {
            let last = list.start + list.items.len();
            (st.size * 0.62 * (last.to_string().len() as f32 + 1.0)).max(st.size * 1.4).round()
                + 4.0
        } else {
            (st.size * 1.4).round()
        };
        let gap = if list.tight { (st.size * 0.25).round() } else { st.block_spacing() };
        let mut c = col().w_full().gap(gap);
        let bullets = ["•", "◦", "▪"];
        for (i, item) in list.items.iter().enumerate() {
            let marker: El = match item.task {
                Some(done) => {
                    let size = (st.size * 0.95).round();
                    let mut b = el().size(size, size).radius(4.0).center();
                    b = if done {
                        b.fill(p.accent)
                            .child(icon(&crate::icons::CHECK, size - 3.0).color(Color::WHITE))
                    } else {
                        b.border(1.5, p.tertiary)
                    };
                    row().w(marker_w).h((st.size * st.line).round()).center_y().child(b)
                }
                None if list.ordered => text(format!("{}.", list.start + i))
                    .font(TextStyle { features: "'tnum' 1", ..st.body(400.0) })
                    .color(p.secondary)
                    .w(marker_w)
                    .align(Align::End)
                    .pr(8.0),
                None => text(bullets[depth.min(2)])
                    .font(st.body(400.0))
                    .color(p.secondary)
                    .w(marker_w)
                    .align(Align::Center),
            };
            let scale = if list.tight { 0.25 } else { 0.85 };
            let body = self.blocks_scaled(&item.blocks, width - marker_w, color, depth + 1, scale);
            c = c
                .child(row().w_full().items_start().child(marker).child(body.grow(1.0).min_w(0.0)));
        }
        c
    }

    fn table(&self, table: &Table, width: f32, color: Color) -> El {
        let p = self.p;
        let st = self.st;
        let columns = table_columns(table);
        // Columns share the width by their longest text, as an auto table layout would.
        let mut weights = vec![1.0f32; columns];
        for r in std::iter::once(&table.head).chain(table.rows.iter()) {
            for (i, cell) in r.iter().enumerate().take(columns) {
                let len = crate::doc::inline_text(cell).chars().count() as f32;
                weights[i] = weights[i].max(len.clamp(3.0, 60.0));
            }
        }
        // Each column's natural width: its longest cell on one line, plus padding.
        let pad =
            |i: usize| if i == 0 { 2.0 } else { 10.0 } + if i + 1 == columns { 2.0 } else { 10.0 };
        let mut natural = vec![0.0f32; columns];
        for (ri, r) in std::iter::once(&table.head).chain(table.rows.iter()).enumerate() {
            let per_char = st.size * if ri == 0 { 0.56 } else { 0.5 };
            for (i, cell) in r.iter().enumerate().take(columns) {
                let len = crate::doc::inline_text(cell).chars().count() as f32;
                natural[i] = natural[i].max((len * per_char).ceil() + pad(i));
            }
        }
        // A table with room sets its columns at their natural widths, as a browser does;
        // one without shares the width by content.
        let fits = natural.iter().sum::<f32>() <= width;
        let line = (st.size * (st.line - 0.12).max(1.1 * 1.2)).round();
        let cell = |ctx: &Ctx, inlines: &[Inline], i: usize, header: bool| -> El {
            let weight = if header { 600.0 } else { 400.0 };
            let t = ctx.inlines(inlines, st.size, weight, color, line);
            let align = match table.aligns.get(i).copied().unwrap_or(CellAlign::None) {
                CellAlign::Center => Align::Center,
                CellAlign::Right => Align::End,
                _ => Align::Start,
            };
            let c =
                if fits { col().w(natural[i].max(40.0)) } else { col().grow(weights[i]).w(0.0) };
            c.min_w(40.0)
                .py(7.0)
                .pl(if i == 0 { 2.0 } else { 10.0 })
                .pr(if i + 1 == columns { 2.0 } else { 10.0 })
                .child(t.w_full().align(align))
        };
        let mut c = if fits {
            col().w(natural.iter().map(|w| w.max(40.0)).sum::<f32>())
        } else {
            col().w_full()
        };
        if !table.head.is_empty() {
            let mut r = row().w_full().border_sides([0.0, 0.0, 1.0, 0.0], p.strong_line);
            for i in 0..columns {
                let empty = Vec::new();
                r = r.child(cell(self, table.head.get(i).unwrap_or(&empty), i, true));
            }
            c = c.child(r);
        }
        let n = table.rows.len();
        for (ri, cells) in table.rows.iter().enumerate() {
            let mut r = row().w_full();
            if ri + 1 < n {
                r = r.border_sides([0.0, 0.0, 0.5, 0.0], p.line);
            }
            for i in 0..columns {
                let empty = Vec::new();
                r = r.child(cell(self, cells.get(i).unwrap_or(&empty), i, false));
            }
            c = c.child(r);
        }
        let min_width = columns as f32 * 90.0;
        if min_width > width { c.w(min_width).scroll_wrap() } else { c }
    }

    fn images(&self, images: &[Image], width: f32) -> El {
        let p = self.p;
        let single = images.len() == 1;
        let mut r = row().w_full().flex_wrap().gap(6.0).items_end();
        if single {
            r = r.justify_center();
        }
        for image in images {
            let key = media::image_key(&image.src);
            let mut e = match (self.media.get(&key), &image.src) {
                (Some(Asset::Ready(img, size)), _) if size[0] > 0.0 => {
                    picture(img, fit(*size, width), width)
                }
                (Some(Asset::Failed(_)), _) | (_, ImageSrc::Missing) => {
                    let alt = if image.alt.trim().is_empty() {
                        "Image".to_string()
                    } else {
                        image.alt.clone()
                    };
                    row()
                        .gap(6.0)
                        .center_y()
                        .px(8.0)
                        .h(24.0)
                        .radius(5.0)
                        .border(1.0, p.line)
                        .child(text(alt).font(theme::ui(12.0, 500.0)).color(p.tertiary).truncate())
                }
                _ => el().size(if single { 160.0 } else { 80.0 }, 20.0).radius(4.0).fill(p.well),
            };
            if let Some(link) = image.link.clone() {
                let pane = self.pane;
                e = e.cursor(rune::Cursor::Pointer).help(link.clone()).on_press_tx(
                    move |s: &mut App, tx: &mut rune::Tx| s.open_link(pane, &link, tx),
                );
            }
            r = r.child(e);
        }
        r
    }
}

trait ElExt {
    fn justify_end_items(self) -> Self;
    fn scroll_wrap(self) -> Self;
}

impl ElExt for El {
    fn justify_end_items(self) -> Self {
        self.justify_end()
    }
    /// A table wider than the column scrolls sideways inside it.
    fn scroll_wrap(self) -> Self {
        row().w_full().scroll_x().child(self)
    }
}

/// `size` scaled down to fit `width`, keeping its shape.
fn fit(size: [f32; 2], width: f32) -> [f32; 2] {
    if size[0] <= width || size[0] <= 0.0 { size } else { [width, size[1] * width / size[0]] }
}

fn picture(image: &rune::Image, size: [f32; 2], width: f32) -> El {
    let [w, h] = fit(size, width);
    el().size(w.max(1.0), h.max(1.0)).radius(2.0).pixels(image.pixels())
}

fn placeholder(p: &Palette, label: &str, height: f32, width: f32) -> El {
    row()
        .w(width.min(720.0))
        .h(height)
        .self_center()
        .center()
        .radius(8.0)
        .fill(p.well)
        .child(text(label.to_string()).font(theme::ui(12.0, 500.0)).color(p.tertiary))
}

/// The texts of a block in the order [`Ctx::block`] draws them, for find.
pub fn texts(b: &Block, out: &mut Vec<String>, st: &ReadStyle, p: &Palette) {
    let flat = |inlines: &[Inline]| {
        flatten(
            inlines,
            &InlineStyle { p, size: st.size, code_family_mono: true, base_weight: 400.0 },
        )
        .text
    };
    match b {
        Block::Heading { inlines, .. } | Block::Paragraph(inlines) => out.push(flat(inlines)),
        Block::Code(chunk) => out.push(chunk.text().to_string()),
        Block::Mermaid(code) => out.push(code.to_string()),
        Block::Math(tex) => out.push(tex.trim().to_string()),
        Block::Quote(blocks) | Block::Alert(_, blocks) => {
            for b in blocks {
                texts(b, out, st, p);
            }
        }
        Block::List(list) => {
            for item in &list.items {
                for b in &item.blocks {
                    texts(b, out, st, p);
                }
            }
        }
        Block::Table(table) => {
            let columns = table_columns(table);
            for r in std::iter::once(&table.head).filter(|h| !h.is_empty()).chain(table.rows.iter())
            {
                for i in 0..columns {
                    out.push(r.get(i).map(|c| flat(c)).unwrap_or_default());
                }
            }
        }
        Block::Images(_) | Block::Rule => {}
        Block::Footnotes(notes) => {
            for note in notes {
                for b in &note.blocks {
                    texts(b, out, st, p);
                }
            }
        }
    }
}

fn table_columns(table: &Table) -> usize {
    table
        .aligns
        .len()
        .max(table.head.len())
        .max(table.rows.iter().map(|r| r.len()).max().unwrap_or(0))
        .max(1)
}

fn alert_icon(kind: crate::doc::AlertKind) -> &'static rune::Icon {
    use crate::doc::AlertKind::*;
    match kind {
        Note => &crate::icons::ALERT_NOTE,
        Tip => &crate::icons::ALERT_TIP,
        Important => &crate::icons::ALERT_IMPORTANT,
        Warning => &crate::icons::ALERT_WARNING,
        Caution => &crate::icons::ALERT_CAUTION,
    }
}

/// The ↩ after a footnote, back to where it's referenced.
fn back_link(pane: u64, label: &str, size: f32, p: &Palette) -> El {
    let id = format!("#fnref-{label}");
    rich("↩", vec![Span::new(0, "↩".len()).color(p.accent)])
        .font(theme::ui(size, 500.0))
        .link(Link::new(0.."↩".len(), id).hover(LinkPaint::new().underline(p.accent)))
        .on_link(move |s: &mut App, id: &str, tx: &mut rune::Tx| s.open_link(pane, id, tx))
        .label("Back to reference")
        .pl(6.0)
}
