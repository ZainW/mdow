//! Markdown (and HTML) parsed into the reader's block model.
//!
//! The top level of a document becomes a flat list of rows for a virtualized list view. Very
//! long code blocks and lists are split into several rows so no single row gets huge, the same
//! trick that keeps giant documents fast.

use comrak::nodes::{AlertType, ListType, Node, NodeValue, TableAlignment};
use comrak::{Arena, Options};
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const CODE_CHUNK_LINES: usize = 60;
pub const LIST_CHUNK_ITEMS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertKind {
    pub fn css(self) -> &'static str {
        match self {
            AlertKind::Note => "md-alert-note",
            AlertKind::Tip => "md-alert-tip",
            AlertKind::Important => "md-alert-important",
            AlertKind::Warning => "md-alert-warning",
            AlertKind::Caution => "md-alert-caution",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text(String),
    Code(String),
    Strong(Vec<Inline>),
    Emph(Vec<Inline>),
    Strike(Vec<Inline>),
    Underline(Vec<Inline>),
    Mark(Vec<Inline>),
    Sup(Vec<Inline>),
    Sub(Vec<Inline>),
    Link { url: String, children: Vec<Inline> },
    Image { src: String, alt: String },
    Break,
    Math(String),
    FootnoteRef(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Whole,
    First,
    Middle,
    Last,
}

#[derive(Debug)]
pub struct CodeSource {
    /// Unique per parsed block, so highlight results can be cached across rebinds.
    pub id: usize,
    pub lang: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct CodeChunk {
    pub source: Arc<CodeSource>,
    pub range: Range<usize>,
    pub part: Part,
    /// The rows every chunk of this code block occupies, filled in once rows are numbered.
    pub rows: Range<usize>,
}

impl CodeChunk {
    pub fn text(&self) -> &str {
        &self.source.text[self.range.clone()]
    }
}

#[derive(Clone, Debug)]
pub struct List {
    pub ordered: bool,
    pub start: usize,
    pub tight: bool,
    pub depth: usize,
    pub items: Vec<Item>,
    /// Set on the rows of a list split across rows: the list continues before / after.
    pub cont_before: bool,
    pub cont_after: bool,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub aligns: Vec<Align>,
    pub head: Vec<Vec<Inline>>,
    pub rows: Vec<Vec<Vec<Inline>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageSrc {
    File(PathBuf),
    Remote(String),
    Missing,
}

#[derive(Clone, Debug)]
pub struct Image {
    pub src: ImageSrc,
    pub alt: String,
    pub link: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Footnote {
    pub label: String,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug)]
pub enum Block {
    Heading { level: u8, inlines: Vec<Inline>, slug: String },
    Paragraph(Vec<Inline>),
    Code(CodeChunk),
    Mermaid(Arc<str>),
    Quote(Vec<Block>),
    Alert(AlertKind, Vec<Block>),
    List(List),
    Table(Table),
    Images(Vec<Image>),
    Rule,
    Math(String),
    Footnotes(Vec<Footnote>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub slug: String,
    pub row: usize,
}

#[derive(Debug, Default)]
pub struct Doc {
    pub blocks: Vec<Block>,
    pub headings: Vec<Heading>,
    /// Frontmatter `title:` (markdown) or `<title>` (HTML).
    pub title: Option<String>,
    /// Heading slug → row, for `#anchor` links.
    pub anchors: HashMap<String, usize>,
}

const HEADING_SCALE: [f64; 6] = [1.875, 1.5, 1.15, 1.0, 0.95, 0.875];
const HEADING_TOP: [f64; 6] = [2.0, 1.8, 1.5, 1.3, 1.2, 1.0];
const HEADING_BOTTOM: [f64; 6] = [0.6, 0.5, 0.4, 0.3, 0.25, 0.2];

impl Block {
    /// CSS margins, top and bottom, in units of the body font size.
    pub fn margins(&self) -> (f64, f64) {
        match self {
            Block::Heading { level, .. } => {
                let i = (*level as usize).clamp(1, 6) - 1;
                (HEADING_TOP[i] * HEADING_SCALE[i], HEADING_BOTTOM[i] * HEADING_SCALE[i])
            }
            Block::Paragraph(_) | Block::Images(_) | Block::Quote(_) => (0.0, 1.0),
            Block::Code(chunk) => match chunk.part {
                Part::Whole => (1.25, 1.25),
                Part::First => (1.25, 0.0),
                Part::Middle => (0.0, 0.0),
                Part::Last => (0.0, 1.25),
            },
            Block::Mermaid(_) => (1.5, 1.5),
            Block::Alert(..) | Block::Table(_) => (1.25, 1.25),
            Block::List(list) => {
                (if list.cont_before { 0.35 } else { 0.0 }, if list.cont_after { 0.0 } else { 1.0 })
            }
            Block::Rule => (2.0, 2.0),
            Block::Math(_) => (1.0, 1.0),
            Block::Footnotes(_) => (2.0, 0.0),
        }
    }
}

/// The gap above each block of a sequence, collapsing margins the way CSS does. The first
/// block sits flush with its container.
pub fn gaps(blocks: &[Block]) -> Vec<f64> {
    let mut out = Vec::with_capacity(blocks.len());
    let mut prev_bottom: Option<f64> = None;
    for block in blocks {
        let (top, bottom) = block.margins();
        out.push(match prev_bottom {
            None => 0.0,
            Some(prev) => prev.max(top),
        });
        prev_bottom = Some(bottom);
    }
    out
}

/// The desktop's heading slug: lowercase, ASCII letters, digits, spaces and dashes only.
pub fn slugify(text: &str) -> String {
    let kept: String = text
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c.is_whitespace() || *c == '-')
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join("-")
}

pub fn inline_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    fn walk(inlines: &[Inline], out: &mut String) {
        for inline in inlines {
            match inline {
                Inline::Text(t) | Inline::Code(t) | Inline::Math(t) => out.push_str(t),
                Inline::Strong(c)
                | Inline::Emph(c)
                | Inline::Strike(c)
                | Inline::Underline(c)
                | Inline::Mark(c)
                | Inline::Sup(c)
                | Inline::Sub(c)
                | Inline::Link { children: c, .. } => walk(c, out),
                Inline::Image { alt, .. } => out.push_str(alt),
                Inline::Break => out.push(' '),
                Inline::FootnoteRef(r) => out.push_str(r),
            }
        }
    }
    walk(inlines, &mut out);
    out
}

fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.alerts = true;
    options.extension.math_dollars = true;
    options.extension.superscript = false;
    options.extension.front_matter_delimiter = Some("---".to_string());
    options
}

static NEXT_CODE_ID: AtomicUsize = AtomicUsize::new(1);

struct Ctx<'p> {
    base_dir: &'p Path,
    title: Option<String>,
    footnotes: Vec<Footnote>,
    html_depth: usize,
}

pub fn parse_markdown(text: &str, path: &Path) -> Doc {
    let base_dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut ctx = Ctx { base_dir: &base_dir, title: None, footnotes: Vec::new(), html_depth: 0 };
    let mut blocks = parse_blocks(text, &mut ctx);
    if !ctx.footnotes.is_empty() {
        blocks.push(Block::Footnotes(std::mem::take(&mut ctx.footnotes)));
    }
    let title = ctx.title.take();
    finish(blocks, title)
}

/// HTML documents read through the same pipeline: converted to markdown, then parsed. Scripts and
/// styles never run or show; the result is a readable, native rendering of the page's content.
pub fn parse_html(html: &str, path: &Path) -> Doc {
    let title = html_title(html);
    let markdown = html_to_markdown(html);
    let mut doc = parse_markdown(&markdown, path);
    if doc.title.is_none() {
        doc.title = title;
    }
    doc
}

pub fn html_to_markdown(html: &str) -> String {
    htmd::HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "noscript", "template", "head", "iframe", "svg"])
        .build()
        .convert(html)
        .unwrap_or_default()
}

fn html_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let open_end = start + lower[start..].find('>')? + 1;
    let close = open_end + lower[open_end..].find("</title>")?;
    let title = decode_entities(html[open_end..close].trim());
    (!title.is_empty()).then_some(title)
}

fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn parse_blocks(text: &str, ctx: &mut Ctx) -> Vec<Block> {
    let arena = Arena::new();
    let root = comrak::parse_document(&arena, text, &options());
    let mut out = Vec::new();
    for child in root.children() {
        convert_block(child, ctx, 0, &mut out);
    }
    out
}

fn children_blocks<'a>(node: Node<'a>, ctx: &mut Ctx, depth: usize) -> Vec<Block> {
    let mut out = Vec::new();
    for child in node.children() {
        convert_block(child, ctx, depth, &mut out);
    }
    out
}

fn alert_kind(kind: AlertType) -> AlertKind {
    match kind {
        AlertType::Note => AlertKind::Note,
        AlertType::Tip => AlertKind::Tip,
        AlertType::Important => AlertKind::Important,
        AlertType::Warning => AlertKind::Warning,
        AlertType::Caution => AlertKind::Caution,
    }
}

fn convert_block<'a>(node: Node<'a>, ctx: &mut Ctx, depth: usize, out: &mut Vec<Block>) {
    let data = node.data();
    match &data.value {
        NodeValue::FrontMatter(raw) => {
            ctx.title = frontmatter_title(raw);
        }
        NodeValue::Paragraph => {
            let inlines = convert_inlines(node);
            paragraph_blocks(inlines, ctx, out);
        }
        NodeValue::Heading(heading) => {
            let inlines = convert_inlines(node);
            out.push(Block::Heading { level: heading.level, inlines, slug: String::new() });
        }
        NodeValue::CodeBlock(code) => {
            let lang = crate::highlight::fence_language(&code.info);
            let mut text = code.literal.clone();
            if text.ends_with('\n') {
                text.pop();
            }
            match lang.as_str() {
                "mermaid" => out.push(Block::Mermaid(Arc::from(text.as_str()))),
                "math" => out.push(Block::Math(text)),
                _ => out.push(Block::Code(CodeChunk {
                    range: 0..text.len(),
                    source: Arc::new(CodeSource {
                        id: NEXT_CODE_ID.fetch_add(1, Ordering::Relaxed),
                        lang,
                        text,
                    }),
                    part: Part::Whole,
                    rows: 0..0,
                })),
            }
        }
        NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => {
            out.push(Block::Quote(children_blocks(node, ctx, depth)));
        }
        NodeValue::Alert(alert) => {
            out.push(Block::Alert(alert_kind(alert.alert_type), children_blocks(node, ctx, depth)));
        }
        NodeValue::List(list) => {
            let mut items = Vec::new();
            for item in node.children() {
                let task = match &item.data().value {
                    NodeValue::TaskItem(task) => Some(task.symbol.is_some_and(|c| c != ' ')),
                    _ => None,
                };
                items.push(Item { task, blocks: children_blocks(item, ctx, depth + 1) });
            }
            out.push(Block::List(List {
                ordered: list.list_type == ListType::Ordered,
                start: list.start.max(if list.list_type == ListType::Ordered { 0 } else { 1 }),
                tight: list.tight,
                depth,
                items,
                cont_before: false,
                cont_after: false,
            }));
        }
        NodeValue::Table(table) => {
            let aligns = table
                .alignments
                .iter()
                .map(|a| match a {
                    TableAlignment::Left => Align::Left,
                    TableAlignment::Center => Align::Center,
                    TableAlignment::Right => Align::Right,
                    TableAlignment::None => Align::None,
                })
                .collect();
            let mut head = Vec::new();
            let mut rows = Vec::new();
            for row in node.children() {
                let is_header = matches!(row.data().value, NodeValue::TableRow(true));
                let cells: Vec<Vec<Inline>> =
                    row.children().map(|cell| convert_inlines(cell)).collect();
                if is_header { head = cells } else { rows.push(cells) }
            }
            out.push(Block::Table(Table { aligns, head, rows }));
        }
        NodeValue::ThematicBreak => out.push(Block::Rule),
        NodeValue::HtmlBlock(html) => {
            if ctx.html_depth < 2 {
                let markdown = html_to_markdown(&html.literal);
                if !markdown.trim().is_empty() {
                    ctx.html_depth += 1;
                    let blocks = parse_blocks(&markdown, ctx);
                    ctx.html_depth -= 1;
                    out.extend(blocks);
                }
            }
        }
        NodeValue::FootnoteDefinition(def) => {
            let blocks = children_blocks(node, ctx, depth);
            let label = (ctx.footnotes.len() + 1).to_string();
            let _ = &def.name;
            ctx.footnotes.push(Footnote { label, blocks });
        }
        NodeValue::DescriptionList => {
            for item in node.children() {
                for part in item.children() {
                    match &part.data().value {
                        NodeValue::DescriptionTerm => {
                            for p in part.children() {
                                let inlines = convert_inlines(p);
                                out.push(Block::Paragraph(vec![Inline::Strong(inlines)]));
                            }
                        }
                        _ => out.push(Block::Quote(children_blocks(part, ctx, depth))),
                    }
                }
            }
        }
        NodeValue::Document => {
            for child in node.children() {
                convert_block(child, ctx, depth, out);
            }
        }
        _ => {
            // Anything else that holds inline content reads as a paragraph.
            let inlines = convert_inlines(node);
            if !inlines.is_empty() {
                out.push(Block::Paragraph(inlines));
            }
        }
    }
}

/// A paragraph of nothing but images (badges, screenshots) becomes an image row; a lone display
/// formula becomes a math block.
fn paragraph_blocks(inlines: Vec<Inline>, ctx: &Ctx, out: &mut Vec<Block>) {
    let blank = |i: &Inline| {
        matches!(i, Inline::Break) || matches!(i, Inline::Text(t) if t.trim().is_empty())
    };
    let meaningful: Vec<&Inline> = inlines.iter().filter(|i| !blank(i)).collect();
    if meaningful.is_empty() {
        return;
    }
    let image_of = |inline: &Inline| -> Option<Image> {
        match inline {
            Inline::Image { src, alt } => {
                Some(Image { src: resolve_image(src, ctx.base_dir), alt: alt.clone(), link: None })
            }
            Inline::Link { url, children } => {
                let inner: Vec<&Inline> = children.iter().filter(|i| !blank(i)).collect();
                match inner.as_slice() {
                    [Inline::Image { src, alt }] => Some(Image {
                        src: resolve_image(src, ctx.base_dir),
                        alt: alt.clone(),
                        link: Some(url.clone()),
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    };
    let images: Vec<Image> = meaningful.iter().filter_map(|i| image_of(i)).collect();
    if images.len() == meaningful.len() {
        out.push(Block::Images(images));
        return;
    }
    if let [Inline::Math(tex)] = meaningful.as_slice()
        && inlines.len() == 1
        && tex.contains('\n')
    {
        out.push(Block::Math(tex.clone()));
        return;
    }
    out.push(Block::Paragraph(inlines));
}

fn frontmatter_title(raw: &str) -> Option<String> {
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("title:") {
            let value = rest.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn resolve_image(src: &str, base_dir: &Path) -> ImageSrc {
    let src = src.trim();
    if src.is_empty() {
        return ImageSrc::Missing;
    }
    if src.starts_with("http://") || src.starts_with("https://") {
        return ImageSrc::Remote(src.to_string());
    }
    if let Some(rest) = src.strip_prefix("file://") {
        return ImageSrc::File(PathBuf::from(percent_decode(rest)));
    }
    if src.contains("://") || src.starts_with("data:") {
        return ImageSrc::Missing;
    }
    let clean = src.split(['?', '#']).next().unwrap_or(src);
    let decoded = percent_decode(clean);
    let path = Path::new(&decoded);
    ImageSrc::File(if path.is_absolute() { path.to_path_buf() } else { base_dir.join(path) })
}

/// Inline HTML tags rendered as formatting; the rest are dropped and their text kept.
fn html_wrap(tag: &str, href: Option<String>, children: Vec<Inline>) -> Vec<Inline> {
    match tag {
        "b" | "strong" => vec![Inline::Strong(children)],
        "i" | "em" | "cite" | "var" | "dfn" => vec![Inline::Emph(children)],
        "s" | "del" | "strike" => vec![Inline::Strike(children)],
        "u" | "ins" => vec![Inline::Underline(children)],
        "mark" => vec![Inline::Mark(children)],
        "sub" => vec![Inline::Sub(children)],
        "sup" => vec![Inline::Sup(children)],
        "code" | "kbd" | "samp" | "tt" => vec![Inline::Code(inline_text(&children))],
        "a" => match href {
            Some(url) => vec![Inline::Link { url, children }],
            None => children,
        },
        _ => children,
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(pos) = lower[from..].find(name) {
        let at = from + pos;
        from = at + name.len();
        let before_ok = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
        let rest = tag[at + name.len()..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let value = rest[1..].trim_start();
        let (quote, body) = match value.chars().next() {
            Some(q @ ('"' | '\'')) => (Some(q), &value[1..]),
            _ => (None, value),
        };
        let end = match quote {
            Some(q) => body.find(q).unwrap_or(body.len()),
            None => body.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(body.len()),
        };
        return Some(decode_entities(&body[..end]));
    }
    None
}

fn convert_inlines<'a>(node: Node<'a>) -> Vec<Inline> {
    // A stack of open inline HTML tags: (tag, href, collected children).
    let mut stack: Vec<(String, Option<String>, Vec<Inline>)> =
        vec![(String::new(), None, Vec::new())];
    for child in node.children() {
        let data = child.data();
        let push = |stack: &mut Vec<(String, Option<String>, Vec<Inline>)>, inline: Inline| {
            stack.last_mut().expect("root frame").2.push(inline);
        };
        match &data.value {
            NodeValue::Text(text) => push(&mut stack, Inline::Text(text.to_string())),
            NodeValue::SoftBreak | NodeValue::LineBreak => push(&mut stack, Inline::Break),
            NodeValue::Code(code) => push(&mut stack, Inline::Code(code.literal.clone())),
            NodeValue::Emph => push(&mut stack, Inline::Emph(convert_inlines(child))),
            NodeValue::Strong => push(&mut stack, Inline::Strong(convert_inlines(child))),
            NodeValue::Strikethrough => push(&mut stack, Inline::Strike(convert_inlines(child))),
            NodeValue::Underline => push(&mut stack, Inline::Underline(convert_inlines(child))),
            NodeValue::Highlight | NodeValue::Insert => {
                push(&mut stack, Inline::Mark(convert_inlines(child)))
            }
            NodeValue::Superscript => push(&mut stack, Inline::Sup(convert_inlines(child))),
            NodeValue::Subscript => push(&mut stack, Inline::Sub(convert_inlines(child))),
            NodeValue::Link(link) => push(
                &mut stack,
                Inline::Link { url: link.url.clone(), children: convert_inlines(child) },
            ),
            NodeValue::WikiLink(link) => push(
                &mut stack,
                Inline::Link { url: link.url.clone(), children: convert_inlines(child) },
            ),
            NodeValue::Image(link) => {
                let alt = inline_text(&convert_inlines(child));
                push(&mut stack, Inline::Image { src: link.url.clone(), alt })
            }
            NodeValue::FootnoteReference(r) => {
                push(&mut stack, Inline::FootnoteRef(r.ix.to_string()))
            }
            NodeValue::Math(math) => push(&mut stack, Inline::Math(math.literal.clone())),
            NodeValue::Raw(text) => push(&mut stack, Inline::Text(text.clone())),
            NodeValue::EscapedTag(tag) => push(&mut stack, Inline::Text(tag.to_string())),
            NodeValue::HtmlInline(html) => {
                let tag = html.trim();
                if tag.starts_with("<!--") {
                    continue;
                }
                let name_of = |s: &str| {
                    s.trim_start_matches(['<', '/'])
                        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase()
                };
                let name = name_of(tag);
                if tag.starts_with("</") {
                    if let Some(pos) = stack.iter().rposition(|(n, _, _)| *n == name)
                        && pos > 0
                    {
                        while stack.len() > pos {
                            let (n, href, children) = stack.pop().expect("frame");
                            let wrapped = html_wrap(&n, href, children);
                            stack.last_mut().expect("parent").2.extend(wrapped);
                        }
                    }
                } else if name == "br" {
                    push(&mut stack, Inline::Break);
                } else if name == "img" {
                    let src = attr(tag, "src").unwrap_or_default();
                    let alt = attr(tag, "alt").unwrap_or_default();
                    push(&mut stack, Inline::Image { src, alt });
                } else if !tag.ends_with("/>") && !name.is_empty() && name != "wbr" && name != "hr"
                {
                    let href = if name == "a" { attr(tag, "href") } else { None };
                    stack.push((name, href, Vec::new()));
                }
            }
            _ => {
                let nested = convert_inlines(child);
                stack.last_mut().expect("frame").2.extend(nested);
            }
        }
    }
    while stack.len() > 1 {
        let (_, _, children) = stack.pop().expect("frame");
        stack.last_mut().expect("parent").2.extend(children);
    }
    stack.pop().map(|f| f.2).unwrap_or_default()
}

/// Split long code blocks and lists into rows, number the rows, and index the headings.
fn finish(blocks: Vec<Block>, title: Option<String>) -> Doc {
    let mut rows: Vec<Block> = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block {
            Block::Code(chunk) => split_code(chunk, &mut rows),
            Block::List(list) if list.items.len() > LIST_CHUNK_ITEMS => split_list(list, &mut rows),
            other => rows.push(other),
        }
    }

    let mut headings = Vec::new();
    let mut anchors = HashMap::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut code_rows: HashMap<usize, Range<usize>> = HashMap::new();
    for (row, block) in rows.iter_mut().enumerate() {
        match block {
            Block::Heading { level, inlines, slug } => {
                let text = inline_text(inlines).trim().to_string();
                let base = slugify(&text);
                let count = seen.entry(base.clone()).or_insert(0);
                let unique = if *count == 0 { base.clone() } else { format!("{base}-{count}") };
                *count += 1;
                *slug = unique.clone();
                anchors.insert(unique.clone(), row);
                headings.push(Heading { level: *level, text, slug: unique, row });
            }
            Block::Code(chunk) => {
                code_rows
                    .entry(chunk.source.id)
                    .and_modify(|r| r.end = row + 1)
                    .or_insert(row..row + 1);
            }
            _ => {}
        }
    }
    for block in rows.iter_mut() {
        if let Block::Code(chunk) = block
            && let Some(range) = code_rows.get(&chunk.source.id)
        {
            chunk.rows = range.clone();
        }
    }
    Doc { blocks: rows, headings, title, anchors }
}

fn split_code(chunk: CodeChunk, rows: &mut Vec<Block>) {
    let text = &chunk.source.text;
    let mut starts = vec![0usize];
    let mut lines = 0;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            lines += 1;
            if lines % CODE_CHUNK_LINES == 0 && i + 1 < text.len() {
                starts.push(i + 1);
            }
        }
    }
    if starts.len() == 1 {
        rows.push(Block::Code(chunk));
        return;
    }
    let n = starts.len();
    for (i, &start) in starts.iter().enumerate() {
        // Each chunk but the last drops its trailing newline; the row boundary is the break.
        let end = if i + 1 < n { starts[i + 1] - 1 } else { text.len() };
        let part = match i {
            0 => Part::First,
            _ if i + 1 == n => Part::Last,
            _ => Part::Middle,
        };
        rows.push(Block::Code(CodeChunk {
            source: chunk.source.clone(),
            range: start..end,
            part,
            rows: 0..0,
        }));
    }
}

fn split_list(list: List, rows: &mut Vec<Block>) {
    let total = list.items.len();
    let mut items = list.items.into_iter();
    let mut index = 0;
    while index < total {
        let take: Vec<Item> = items.by_ref().take(LIST_CHUNK_ITEMS).collect();
        let count = take.len();
        rows.push(Block::List(List {
            ordered: list.ordered,
            start: list.start + index,
            tight: list.tight,
            depth: list.depth,
            items: take,
            cont_before: index > 0,
            cont_after: index + count < total,
        }));
        index += count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Doc {
        parse_markdown(text, Path::new("/docs/readme.md"))
    }

    #[test]
    fn slugs_match_the_desktop() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("  Getting   Started  "), "getting-started");
        assert_eq!(slugify("API v2.0 — Notes"), "api-v20-notes");
    }

    #[test]
    fn headings_get_unique_slugs_and_rows() {
        let doc = parse("# Intro\n\ntext\n\n## Intro\n\n## Usage\n");
        let slugs: Vec<_> = doc.headings.iter().map(|h| h.slug.as_str()).collect();
        assert_eq!(slugs, ["intro", "intro-1", "usage"]);
        assert_eq!(doc.anchors["usage"], 3);
    }

    #[test]
    fn parses_gfm_blocks() {
        let doc = parse(
            "> [!WARNING]\n> careful\n\n- [x] done\n- [ ] todo\n\n| a | b |\n|:-|-:|\n| 1 | 2 |\n\n---\n\n```mermaid\ngraph TD; A-->B\n```\n",
        );
        assert!(matches!(doc.blocks[0], Block::Alert(AlertKind::Warning, _)));
        let Block::List(list) = &doc.blocks[1] else { panic!("list") };
        assert_eq!(
            list.items.iter().map(|i| i.task).collect::<Vec<_>>(),
            [Some(true), Some(false)]
        );
        let Block::Table(table) = &doc.blocks[2] else { panic!("table") };
        assert_eq!(table.aligns, [Align::Left, Align::Right]);
        assert_eq!(table.rows.len(), 1);
        assert!(matches!(doc.blocks[3], Block::Rule));
        assert!(matches!(doc.blocks[4], Block::Mermaid(_)));
    }

    #[test]
    fn soft_breaks_are_line_breaks() {
        let doc = parse("one\ntwo\n");
        let Block::Paragraph(inlines) = &doc.blocks[0] else { panic!() };
        assert!(inlines.contains(&Inline::Break));
    }

    #[test]
    fn long_code_blocks_split_into_rows() {
        let code: String = (0..150).map(|i| format!("line {i}\n")).collect();
        let doc = parse(&format!("```\n{code}```\n"));
        assert_eq!(doc.blocks.len(), 3);
        let parts: Vec<Part> = doc
            .blocks
            .iter()
            .map(|b| match b {
                Block::Code(c) => c.part,
                _ => panic!(),
            })
            .collect();
        assert_eq!(parts, [Part::First, Part::Middle, Part::Last]);
        let joined: Vec<&str> = doc
            .blocks
            .iter()
            .map(|b| match b {
                Block::Code(c) => c.text(),
                _ => "",
            })
            .collect();
        assert_eq!(joined.join("\n"), code.trim_end_matches('\n'));
        let Block::Code(first) = &doc.blocks[0] else { panic!() };
        assert_eq!(first.rows, 0..3);
    }

    #[test]
    fn long_lists_continue_numbering_across_rows() {
        let list: String = (1..=60).map(|i| format!("{i}. item\n")).collect();
        let doc = parse(&list);
        let starts: Vec<usize> = doc
            .blocks
            .iter()
            .map(|b| match b {
                Block::List(l) => l.start,
                _ => 0,
            })
            .collect();
        assert_eq!(starts, [1, 25, 49]);
    }

    #[test]
    fn badge_rows_become_images() {
        let doc = parse("[![ci](https://x/ci.svg)](https://x) ![logo](img/logo.png)\n");
        let Block::Images(images) = &doc.blocks[0] else { panic!("{:?}", doc.blocks[0]) };
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].link.as_deref(), Some("https://x"));
        assert_eq!(images[1].src, ImageSrc::File(PathBuf::from("/docs/img/logo.png")));
    }

    #[test]
    fn html_blocks_render_as_markdown() {
        let doc = parse("<p align=\"center\"><img src=\"a%20b.png\" alt=\"A\"></p>\n\nafter\n");
        let Block::Images(images) = &doc.blocks[0] else { panic!("{:?}", doc.blocks) };
        assert_eq!(images[0].src, ImageSrc::File(PathBuf::from("/docs/a b.png")));
    }

    #[test]
    fn inline_html_formats() {
        let doc = parse("press <kbd>Ctrl</kbd> and <b>go</b><br>now\n");
        let Block::Paragraph(inlines) = &doc.blocks[0] else { panic!() };
        assert!(inlines.contains(&Inline::Code("Ctrl".into())));
        assert!(inlines.contains(&Inline::Strong(vec![Inline::Text("go".into())])));
        assert!(inlines.contains(&Inline::Break));
    }

    #[test]
    fn frontmatter_title_is_read() {
        let doc = parse("---\ntitle: \"My Doc\"\n---\n\n# Body\n");
        assert_eq!(doc.title.as_deref(), Some("My Doc"));
        assert!(matches!(doc.blocks[0], Block::Heading { .. }));
    }

    #[test]
    fn footnotes_collect_at_the_end() {
        let doc = parse("A note[^n].\n\n[^n]: The note.\n");
        assert!(matches!(doc.blocks.last(), Some(Block::Footnotes(f)) if f.len() == 1));
    }

    #[test]
    fn margins_collapse() {
        let doc = parse("para\n\n# Heading\n\npara\n");
        let g = gaps(&doc.blocks);
        assert_eq!(g[0], 0.0);
        assert!((g[1] - 2.0 * 1.875).abs() < 1e-9);
        assert!((g[2] - 0.6 * 1.875).abs() < 1e-9);
    }

    #[test]
    fn html_documents_keep_their_title() {
        let doc = parse_html(
            "<html><head><title>Hi &amp; bye</title><style>p{}</style></head><body><h1>Top</h1><p>x</p><script>alert(1)</script></body></html>",
            Path::new("/a/b.html"),
        );
        assert_eq!(doc.title.as_deref(), Some("Hi & bye"));
        assert_eq!(doc.headings[0].text, "Top");
        let all = format!("{:?}", doc.blocks);
        assert!(!all.contains("alert(1)"));
    }
}

#[cfg(test)]
mod bench {
    #[test]
    #[ignore]
    fn parse_big_file() {
        let path =
            std::env::var("MDOW_BENCH").unwrap_or_else(|_| "/tmp/mdow-fixture/big.md".into());
        let text = std::fs::read_to_string(&path).unwrap();
        let start = std::time::Instant::now();
        let doc = super::parse_markdown(&text, std::path::Path::new(&path));
        eprintln!(
            "parsed {} bytes into {} rows in {:?}",
            text.len(),
            doc.blocks.len(),
            start.elapsed()
        );
    }
}
