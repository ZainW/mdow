//! Printing and Export as PDF: the document's blocks set in Pango on a light page, whatever the
//! theme on screen. The reader is a virtualized list of widgets, so printing lays out the model
//! itself, line by line, and breaks pages between lines.

use crate::doc::{Block, Doc, Image, ImageSrc, Inline, List, Table};
use crate::media::{self, PrintImage};
use crate::reader::Reader;
use crate::render::{Style, markup_of, plain_links};
use crate::theme::{Palette, Rgba};
use gtk::glib::markup_escape_text;
use gtk::{cairo, gdk, glib, pango, prelude::*};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

const BODY_PT: f64 = 11.0;
const CODE_SCALE: f64 = 0.86;
const BODY_FAMILY: &str = "Charter, Bitstream Charter, Georgia, serif";
const CODE_FAMILY: &str = "Geist Mono, monospace";
const HEADING_SCALE: [f64; 6] = [1.9, 1.5, 1.2, 1.05, 1.0, 0.9];

/// What the blocks flatten to before they meet a page size.
enum Piece {
    Text {
        markup: String,
        family: &'static str,
        size: f64,
        indent: f64,
        space_before: f64,
        fill: Option<Rgba>,
        /// Headings stay with what follows them.
        keep_with_next: bool,
        /// Table cells: tab stops as fractions of the width.
        tab_columns: usize,
    },
    Rule {
        space_before: f64,
    },
    Picture {
        surface: cairo::ImageSurface,
        width: f64,
        height: f64,
        indent: f64,
        space_before: f64,
    },
}

fn text(markup: String, indent: f64, space_before: f64) -> Piece {
    Piece::Text {
        markup,
        family: BODY_FAMILY,
        size: BODY_PT,
        indent,
        space_before,
        fill: None,
        keep_with_next: false,
        tab_columns: 0,
    }
}

/// Screen markup, for a page: links can't be followed and code spans don't get a background box.
fn for_page(markup: &str) -> String {
    let mut out = plain_links(markup);
    while let Some(start) = out.find(" background=\"") {
        let value = start + " background=\"".len();
        match out[value..].find('"') {
            Some(end) => out.replace_range(start..value + end + 1, ""),
            None => break,
        }
    }
    out
}

fn surface_of(image: PrintImage) -> Option<cairo::ImageSurface> {
    cairo::ImageSurface::create_for_data(
        image.pixels,
        cairo::Format::ARgb32,
        image.width,
        image.height,
        image.stride,
    )
    .ok()
}

fn picture(image: PrintImage, unit: f64, indent: f64, space_before: f64) -> Option<Piece> {
    let (width, height) = (image.logical_width * unit, image.logical_height * unit);
    surface_of(image).map(|surface| Piece::Picture { surface, width, height, indent, space_before })
}

fn file_picture(path: &Path, indent: f64) -> Option<Piece> {
    let texture = gdk::Texture::from_filename(path).ok()?;
    let (width, height) = (texture.width(), texture.height());
    let stride = width as usize * 4;
    let mut pixels = vec![0u8; stride * height as usize];
    texture.download(&mut pixels, stride);
    let surface = cairo::ImageSurface::create_for_data(
        pixels,
        cairo::Format::ARgb32,
        width,
        height,
        stride as i32,
    )
    .ok()?;
    Some(Piece::Picture {
        surface,
        width: width as f64 * 0.75,
        height: height as f64 * 0.75,
        indent,
        space_before: 8.0,
    })
}

struct Flatten {
    style: Style,
    pieces: Vec<Piece>,
}

impl Flatten {
    fn blocks(&mut self, blocks: &[Block], indent: f64) {
        for block in blocks {
            self.block(block, indent);
        }
    }

    fn block(&mut self, block: &Block, indent: f64) {
        match block {
            Block::Heading { level, inlines, .. } => {
                let index = (*level).clamp(1, 6) as usize - 1;
                let markup =
                    format!("<b>{}</b>", for_page(&markup_of(inlines, &self.style).markup));
                self.pieces.push(Piece::Text {
                    markup,
                    family: BODY_FAMILY,
                    size: BODY_PT * HEADING_SCALE[index],
                    indent,
                    space_before: 18.0 - index as f64,
                    fill: None,
                    keep_with_next: true,
                    tab_columns: 0,
                });
            }
            Block::Paragraph(inlines) => {
                let markup = for_page(&markup_of(inlines, &self.style).markup);
                self.pieces.push(text(markup, indent, 7.0));
            }
            Block::Code(chunk) => {
                self.pieces.push(Piece::Text {
                    markup: markup_escape_text(chunk.text().trim_end_matches('\n')).to_string(),
                    family: CODE_FAMILY,
                    size: BODY_PT * CODE_SCALE,
                    indent,
                    space_before: 7.0,
                    fill: Some(self.style.palette.muted),
                    keep_with_next: false,
                    tab_columns: 0,
                });
            }
            Block::Mermaid(code) => {
                if let Some(image) = media::mermaid_for_print(code, &self.style.palette)
                    && let Some(piece) = picture(image, 0.75, indent, 8.0)
                {
                    self.pieces.push(piece);
                }
            }
            Block::Math(tex) => {
                let color = self.style.palette.foreground;
                let image = media::math_for_print(tex, BODY_PT * 1.2, color);
                match image.and_then(|image| picture(image, 1.0, indent + 16.0, 8.0)) {
                    Some(piece) => self.pieces.push(piece),
                    None => self.pieces.push(Piece::Text {
                        markup: markup_escape_text(tex).to_string(),
                        family: CODE_FAMILY,
                        size: BODY_PT * CODE_SCALE,
                        indent,
                        space_before: 7.0,
                        fill: None,
                        keep_with_next: false,
                        tab_columns: 0,
                    }),
                }
            }
            Block::Quote(blocks) => self.blocks(blocks, indent + 14.0),
            Block::Alert(kind, blocks) => {
                self.pieces.push(text(format!("<b>{}</b>", kind.title()), indent + 14.0, 7.0));
                self.blocks(blocks, indent + 14.0);
            }
            Block::List(list) => self.list(list, indent),
            Block::Table(table) => self.table(table, indent),
            Block::Images(images) => self.images(images, indent),
            Block::Rule => self.pieces.push(Piece::Rule { space_before: 12.0 }),
            Block::Footnotes(notes) => {
                self.pieces.push(Piece::Rule { space_before: 16.0 });
                for note in notes {
                    let start = self.pieces.len();
                    self.blocks(&note.blocks, indent);
                    if let Some(Piece::Text { markup, .. }) = self.pieces.get_mut(start) {
                        *markup =
                            format!("<sup>{}</sup> {markup}", markup_escape_text(&note.label));
                    }
                }
            }
        }
    }

    fn list(&mut self, list: &List, indent: f64) {
        for (number, item) in list.items.iter().enumerate() {
            let marker = match (item.task, list.ordered) {
                (Some(true), _) => "☑".to_string(),
                (Some(false), _) => "☐".to_string(),
                (None, true) => format!("{}.", list.start + number),
                (None, false) => "•".to_string(),
            };
            let start = self.pieces.len();
            self.blocks(&item.blocks, indent + 16.0);
            match self.pieces.get_mut(start) {
                Some(Piece::Text { markup, space_before, .. }) => {
                    *markup = format!("{marker} {markup}");
                    if list.tight {
                        *space_before = 2.0;
                    }
                }
                _ => self.pieces.insert(start, text(marker, indent + 16.0, 2.0)),
            }
        }
    }

    fn table(&mut self, table: &Table, indent: f64) {
        let columns = table.head.len().max(1);
        let row = |cells: &[Vec<Inline>], style: &Style, bold: bool| {
            let cells: Vec<String> = (0..columns)
                .map(|i| {
                    let inlines = cells.get(i).map(Vec::as_slice).unwrap_or(&[]);
                    let cell =
                        for_page(&markup_of(inlines, style).markup).replace(['\n', '\t'], " ");
                    if bold { format!("<b>{cell}</b>") } else { cell }
                })
                .collect();
            cells.join("\t")
        };
        let mut push = |markup: String, space_before: f64, keep_with_next: bool| {
            self.pieces.push(Piece::Text {
                markup,
                family: BODY_FAMILY,
                size: BODY_PT * 0.92,
                indent,
                space_before,
                fill: None,
                keep_with_next,
                tab_columns: columns,
            });
        };
        push(row(&table.head, &self.style, true), 8.0, true);
        for cells in &table.rows {
            push(row(cells, &self.style, false), 3.0, false);
        }
    }

    fn images(&mut self, images: &[Image], indent: f64) {
        for image in images {
            let piece = match &image.src {
                ImageSrc::File(path) => file_picture(path, indent),
                ImageSrc::Remote(_) | ImageSrc::Missing => None,
            };
            let fallback = || {
                let alt = if image.alt.is_empty() { "image" } else { &image.alt };
                text(format!("<i>[{}]</i>", markup_escape_text(alt)), indent, 7.0)
            };
            self.pieces.push(piece.unwrap_or_else(fallback));
        }
    }
}

fn flatten(doc: &Doc) -> Vec<Piece> {
    let style = Style {
        palette: Rc::new(Palette::light()),
        fs: BODY_PT,
        code_family: CODE_FAMILY,
        scale_factor: 1,
        line_height: 1.4,
    };
    let mut flatten = Flatten { style, pieces: Vec::new() };
    flatten.blocks(&doc.blocks, 0.0);
    flatten.pieces
}

/// One drawing call, positioned on a page.
enum Op {
    Line { line: pango::LayoutLine, x: f64, baseline: f64 },
    Fill { x: f64, y: f64, width: f64, height: f64, color: Rgba },
    Picture { surface: cairo::ImageSurface, x: f64, y: f64, scale: f64 },
}

/// `pages` starts with one page and only ever grows.
fn current(pages: &mut [Vec<Op>]) -> &mut Vec<Op> {
    pages.last_mut().expect("there is always a page")
}

fn paginate(pieces: &[Piece], context: &gtk::PrintContext) -> Vec<Vec<Op>> {
    let (page_width, page_height) = (context.width(), context.height());
    let mut pages: Vec<Vec<Op>> = vec![Vec::new()];
    let mut y = 0.0;
    let new_page = |pages: &mut Vec<Vec<Op>>, y: &mut f64| {
        pages.push(Vec::new());
        *y = 0.0;
    };
    for piece in pieces {
        match piece {
            Piece::Text {
                markup,
                family,
                size,
                indent,
                space_before,
                fill,
                keep_with_next,
                tab_columns,
            } => {
                let layout = context.create_pango_layout();
                let mut font = pango::FontDescription::from_string(family);
                font.set_size((*size * pango::SCALE as f64) as i32);
                layout.set_font_description(Some(&font));
                let pad = if fill.is_some() { 6.0 } else { 0.0 };
                let width = (page_width - indent - pad * 2.0).max(40.0);
                layout.set_width((width * pango::SCALE as f64) as i32);
                layout.set_wrap(pango::WrapMode::WordChar);
                layout.set_markup(markup);
                layout.set_spacing((size * 0.28 * pango::SCALE as f64) as i32);
                if *tab_columns > 1 {
                    let columns = *tab_columns as i32;
                    let mut tabs = pango::TabArray::new(columns - 1, false);
                    let step = (width * pango::SCALE as f64) as i32 / columns;
                    for i in 1..columns {
                        tabs.set_tab(i - 1, pango::TabAlign::Left, step * i);
                    }
                    layout.set_tabs(Some(&tabs));
                }
                let height = layout.pixel_size().1 as f64;
                y += if y > 0.0 { *space_before } else { 0.0 };
                // A heading stays with the first lines of whatever follows it.
                let needed = if *keep_with_next { height + size * 4.0 } else { size * 1.4 };
                if y + needed > page_height && y > 0.0 {
                    new_page(&mut pages, &mut y);
                }
                let spacing = layout.spacing() as f64 / pango::SCALE as f64;
                y += pad;
                let mut iter = layout.iter();
                loop {
                    let (_, logical) = iter.line_extents();
                    let line_top = logical.y() as f64 / pango::SCALE as f64;
                    let line_height = logical.height() as f64 / pango::SCALE as f64;
                    if y + line_height > page_height {
                        new_page(&mut pages, &mut y);
                    }
                    if let Some(color) = fill {
                        // Exactly one line tall: a taller fill would paint over the line above.
                        current(&mut pages).push(Op::Fill {
                            x: *indent,
                            y,
                            width: page_width - indent,
                            height: line_height + spacing,
                            color: *color,
                        });
                    }
                    let baseline = y + iter.baseline() as f64 / pango::SCALE as f64 - line_top;
                    if let Some(line) = iter.line_readonly() {
                        current(&mut pages).push(Op::Line { line, x: indent + pad, baseline });
                    }
                    y += line_height + spacing;
                    if !iter.next_line() {
                        break;
                    }
                }
                y += pad;
            }
            Piece::Rule { space_before } => {
                y += *space_before;
                if y + 1.0 > page_height {
                    new_page(&mut pages, &mut y);
                }
                current(&mut pages).push(Op::Fill {
                    x: 0.0,
                    y,
                    width: page_width,
                    height: 0.5,
                    color: Rgba { r: 0.7, g: 0.7, b: 0.7, a: 1.0 },
                });
                y += 1.0;
            }
            Piece::Picture { surface, width, height, indent, space_before } => {
                let available = page_width - indent;
                let fit = (available / width).min(page_height / height).min(1.0);
                let (w, h) = (width * fit, height * fit);
                y += if y > 0.0 { *space_before } else { 0.0 };
                if y + h > page_height {
                    new_page(&mut pages, &mut y);
                }
                current(&mut pages).push(Op::Picture {
                    surface: surface.clone(),
                    x: *indent,
                    y,
                    scale: w / surface.width() as f64,
                });
                y += h;
            }
        }
    }
    pages
}

fn draw(ops: &[Op], context: &gtk::PrintContext) {
    let cr = context.cairo_context();
    for op in ops {
        let _ = cr.save();
        match op {
            Op::Fill { x, y, width, height, color } => {
                cr.set_source_rgba(color.r, color.g, color.b, color.a);
                cr.rectangle(*x, *y, *width, *height);
                let _ = cr.fill();
            }
            Op::Line { line, x, baseline } => {
                cr.set_source_rgb(0.08, 0.07, 0.06);
                cr.move_to(*x, *baseline);
                pangocairo::functions::show_layout_line(&cr, line);
            }
            Op::Picture { surface, x, y, scale } => {
                cr.translate(*x, *y);
                cr.scale(*scale, *scale);
                if cr.set_source_surface(surface, 0.0, 0.0).is_ok() {
                    let _ = cr.paint();
                }
            }
        }
        let _ = cr.restore();
    }
}

fn operation(reader: &Reader) -> Option<gtk::PrintOperation> {
    let pieces = reader.with_document(flatten)?;
    let operation = gtk::PrintOperation::new();
    operation.set_unit(gtk::Unit::Points);
    operation.set_use_full_page(false);
    operation.set_embed_page_setup(true);
    let name = reader.path();
    let name = name.file_name().map(|n| n.to_string_lossy().into_owned());
    operation.set_job_name(name.as_deref().unwrap_or("Mdow"));
    let setup = gtk::PageSetup::new();
    setup.set_top_margin(54.0, gtk::Unit::Points);
    setup.set_bottom_margin(54.0, gtk::Unit::Points);
    setup.set_left_margin(54.0, gtk::Unit::Points);
    setup.set_right_margin(54.0, gtk::Unit::Points);
    operation.set_default_page_setup(Some(&setup));
    let pages: Rc<RefCell<Vec<Vec<Op>>>> = Rc::default();
    {
        let pages = pages.clone();
        operation.connect_begin_print(move |operation, context| {
            let laid_out = paginate(&pieces, context);
            operation.set_n_pages(laid_out.len() as i32);
            pages.replace(laid_out);
        });
    }
    operation.connect_draw_page(move |_, context, page| {
        if let Some(ops) = pages.borrow().get(page as usize) {
            draw(ops, context);
        }
    });
    Some(operation)
}

pub fn print(window: &impl IsA<gtk::Window>, reader: &Reader) -> Result<(), glib::Error> {
    if let Some(operation) = operation(reader) {
        operation.run(gtk::PrintOperationAction::PrintDialog, Some(window))?;
    }
    Ok(())
}

pub fn export_pdf(
    window: &impl IsA<gtk::Window>,
    reader: &Reader,
    target: &Path,
) -> Result<(), glib::Error> {
    if let Some(operation) = operation(reader) {
        operation.set_export_filename(target);
        operation.run(gtk::PrintOperationAction::Export, Some(window))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::parse_markdown;

    #[test]
    fn every_block_flattens() {
        let doc = parse_markdown(
            "# Title\n\nBody with **bold**.\n\n- [x] done\n- open\n\n> quoted\n\n---\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n",
            Path::new("/tmp/x.md"),
        );
        let pieces = flatten(&doc);
        assert!(pieces.iter().any(|p| matches!(p, Piece::Rule { .. })));
        let texts: Vec<&String> = pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Text { markup, .. } => Some(markup),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains("☑")));
        assert!(texts.iter().any(|t| t.contains("<b>a</b>\t<b>b</b>")));
        assert!(texts.iter().any(|t| t.contains("fn main")));
    }
}
