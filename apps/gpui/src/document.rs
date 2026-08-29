use std::path::{Component, Path, PathBuf};

use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSource {
    pub canonical_path: PathBuf,
    pub source: String,
}

#[derive(Debug)]
pub enum DocumentError {
    Unsupported { path: PathBuf },
    Missing { path: PathBuf },
    InvalidUtf8 { path: PathBuf },
    Read { path: PathBuf, message: String },
}

impl DocumentError {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "Unsupported file type",
            Self::Missing { .. } => "File not found",
            Self::InvalidUtf8 { .. } => "This file is not UTF-8",
            Self::Read { .. } => "Couldn't read file",
        }
    }

    pub fn body(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => {
                "Mdow opens .md, .markdown, .mdx, .html, and .htm files. Choose a supported file or drop a folder."
            }
            Self::Missing { .. } => "This file may have been moved or renamed.",
            Self::InvalidUtf8 { .. } => "Mdow can only open files encoded as UTF-8.",
            Self::Read { .. } => {
                "Something went wrong trying to read this file. It might be corrupted or locked by another process."
            }
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::Unsupported { path }
            | Self::Missing { path }
            | Self::InvalidUtf8 { path }
            | Self::Read { path, .. } => path,
        }
    }
}

pub fn is_supported_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdx"
            )
        })
}

pub fn is_html_document(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "html" | "htm"))
}

pub fn is_supported_document(path: &Path) -> bool {
    is_supported_markdown(path) || is_html_document(path)
}

pub fn load_source(path: &Path) -> Result<LoadedSource, DocumentError> {
    if !is_supported_document(path) {
        return Err(DocumentError::Unsupported {
            path: path.to_owned(),
        });
    }
    if !path.exists() {
        return Err(DocumentError::Missing {
            path: path.to_owned(),
        });
    }
    let bytes = std::fs::read(path).map_err(|error| DocumentError::Read {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    let source = String::from_utf8(bytes).map_err(|_| DocumentError::InvalidUtf8 {
        path: path.to_owned(),
    })?;
    let canonical_path = path.canonicalize().map_err(|error| DocumentError::Read {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    Ok(LoadedSource {
        canonical_path,
        source,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDocument {
    pub path: PathBuf,
    pub title: String,
    pub frontmatter_title: Option<String>,
    pub source: String,
    pub blocks: Vec<DocumentBlock>,
    pub headings: Vec<Heading>,
}

impl ParsedDocument {
    pub fn plain_text(&self) -> String {
        self.blocks
            .iter()
            .map(DocumentBlock::plain_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineSpan {
    Text(String),
    Emphasis(Vec<InlineSpan>),
    Strong(Vec<InlineSpan>),
    Strikethrough(Vec<InlineSpan>),
    Code(String),
    Link {
        label: Vec<InlineSpan>,
        target: String,
    },
    FootnoteRef {
        label: String,
    },
    SoftBreak,
    HardBreak,
}

impl InlineSpan {
    pub fn plain_text(&self) -> String {
        match self {
            Self::Text(text) | Self::Code(text) => text.clone(),
            Self::Emphasis(content) | Self::Strong(content) | Self::Strikethrough(content) => {
                plain_text_for_spans(content)
            }
            Self::Link { label, .. } => plain_text_for_spans(label),
            Self::FootnoteRef { label } => footnote_ref_display(label),
            Self::SoftBreak | Self::HardBreak => "\n".into(),
        }
    }

    /// The text as painted by the reader: soft breaks render as spaces, hard breaks as newlines.
    pub fn painted_plain_text(&self) -> String {
        match self {
            Self::Text(text) | Self::Code(text) => text.clone(),
            Self::Emphasis(content) | Self::Strong(content) | Self::Strikethrough(content) => {
                painted_plain_text_for_spans(content)
            }
            Self::Link { label, .. } => painted_plain_text_for_spans(label),
            Self::FootnoteRef { label } => footnote_ref_display(label),
            Self::SoftBreak => " ".into(),
            Self::HardBreak => "\n".into(),
        }
    }
}

/// The visible form of a footnote reference: superscript digits when the label is numeric,
/// otherwise a bracketed label.
pub fn footnote_ref_display(label: &str) -> String {
    const SUPERSCRIPT_DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    if !label.is_empty() && label.bytes().all(|byte| byte.is_ascii_digit()) {
        label
            .bytes()
            .map(|byte| SUPERSCRIPT_DIGITS[usize::from(byte - b'0')])
            .collect()
    } else {
        format!("[{label}]")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentBlock {
    Heading {
        level: u8,
        content: Vec<InlineSpan>,
    },
    Paragraph(Vec<InlineSpan>),
    ListItem {
        kind: ListKind,
        depth: usize,
        children: Vec<DocumentBlock>,
    },
    TaskItem {
        checked: bool,
        depth: usize,
        children: Vec<DocumentBlock>,
    },
    Blockquote(Vec<InlineSpan>),
    Alert {
        kind: AlertKind,
        children: Vec<DocumentBlock>,
    },
    FootnoteSection {
        notes: Vec<(String, Vec<DocumentBlock>)>,
    },
    ThematicBreak,
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    MermaidCard {
        source: String,
    },
    Table(TableBlock),
    Image {
        alt: String,
        source: String,
    },
    RawText(String),
}

impl DocumentBlock {
    pub(crate) fn plain_text(&self) -> String {
        match self {
            Self::Heading { content, .. }
            | Self::Paragraph(content)
            | Self::Blockquote(content) => plain_text_for_spans(content),
            Self::ListItem { children, .. }
            | Self::TaskItem { children, .. }
            | Self::Alert { children, .. } => plain_text_for_blocks(children),
            Self::FootnoteSection { notes } => notes
                .iter()
                .map(|(_, blocks)| plain_text_for_blocks(blocks))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
            Self::ThematicBreak => String::new(),
            Self::CodeBlock { code, .. } => code.clone(),
            Self::MermaidCard { source } => source.clone(),
            Self::Table(table) => table.plain_text(),
            Self::Image { alt, .. } | Self::RawText(alt) => alt.clone(),
        }
    }

    /// The text as painted by the reader: soft breaks render as spaces, hard breaks as newlines.
    pub fn painted_plain_text(&self) -> String {
        match self {
            Self::Heading { content, .. }
            | Self::Paragraph(content)
            | Self::Blockquote(content) => painted_plain_text_for_spans(content),
            Self::ListItem { children, .. }
            | Self::TaskItem { children, .. }
            | Self::Alert { children, .. } => painted_plain_text_for_blocks(children),
            Self::FootnoteSection { notes } => notes
                .iter()
                .map(|(_, blocks)| painted_plain_text_for_blocks(blocks))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
            Self::ThematicBreak => String::new(),
            Self::CodeBlock { code, .. } => code.clone(),
            Self::MermaidCard { source } => source.clone(),
            Self::Table(table) => table.painted_plain_text(),
            Self::Image { alt, .. } | Self::RawText(alt) => alt.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Tip => "Tip",
            Self::Important => "Important",
            Self::Warning => "Warning",
            Self::Caution => "Caution",
        }
    }
}

impl From<BlockQuoteKind> for AlertKind {
    fn from(kind: BlockQuoteKind) -> Self {
        match kind {
            BlockQuoteKind::Note => Self::Note,
            BlockQuoteKind::Tip => Self::Tip,
            BlockQuoteKind::Important => Self::Important,
            BlockQuoteKind::Warning => Self::Warning,
            BlockQuoteKind::Caution => Self::Caution,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListKind {
    Unordered,
    Ordered { number: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableBlock {
    pub headers: Vec<Vec<InlineSpan>>,
    pub rows: Vec<Vec<Vec<InlineSpan>>>,
}

impl TableBlock {
    fn plain_text(&self) -> String {
        self.text_rows(plain_text_for_spans)
    }

    fn painted_plain_text(&self) -> String {
        self.text_rows(painted_plain_text_for_spans)
    }

    fn text_rows(&self, cell_text: impl Fn(&[InlineSpan]) -> String) -> String {
        std::iter::once(&self.headers)
            .chain(self.rows.iter())
            .map(|row| {
                row.iter()
                    .map(|cell| cell_text(cell))
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
}

#[derive(Debug, Clone)]
enum InlineContainer {
    Emphasis,
    Strong,
    Strikethrough,
    Link(String),
    Image(String),
    Flatten,
}

/// One open `>` quote: untyped quotes flatten to inline text, GFM alerts keep child blocks.
#[derive(Debug)]
enum QuoteFrame {
    Plain(Vec<InlineSpan>),
    Alert {
        kind: AlertKind,
        children: Vec<DocumentBlock>,
    },
}

#[derive(Debug)]
struct InlineFrame {
    container: InlineContainer,
    spans: Vec<InlineSpan>,
    pending_image: Option<ImageData>,
}

#[derive(Debug)]
struct ImageData {
    alt: String,
    source: String,
}

impl InlineFrame {
    fn new(container: InlineContainer) -> Self {
        Self {
            container,
            spans: Vec::new(),
            pending_image: None,
        }
    }

    fn push_span(&mut self, span: InlineSpan) {
        self.flush_pending_image();
        self.spans.push(span);
    }

    fn into_spans(mut self) -> Vec<InlineSpan> {
        self.flush_pending_image();
        self.spans
    }

    fn take_standalone_image(&mut self) -> Option<ImageData> {
        if self.spans.is_empty() {
            self.pending_image.take()
        } else {
            None
        }
    }

    fn flush_pending_image(&mut self) {
        if let Some(image) = self.pending_image.take() {
            self.spans.push(InlineSpan::Text(image.alt));
        }
    }
}

#[derive(Debug)]
struct ListContext {
    ordered: bool,
    next_number: u64,
}

#[derive(Debug)]
struct ItemContext {
    kind: ListKind,
    depth: usize,
    checked: Option<bool>,
    children: Vec<DocumentBlock>,
}

impl ItemContext {
    fn push_content(&mut self, content: Vec<InlineSpan>) {
        if !content.is_empty() {
            self.children.push(DocumentBlock::Paragraph(content));
        }
    }

    fn push_block(&mut self, block: DocumentBlock) {
        self.children.push(block);
    }

    fn into_block(self) -> DocumentBlock {
        match self.checked {
            Some(checked) => DocumentBlock::TaskItem {
                checked,
                depth: self.depth,
                children: self.children,
            },
            None => DocumentBlock::ListItem {
                kind: self.kind,
                depth: self.depth,
                children: self.children,
            },
        }
    }
}

#[derive(Debug)]
struct CodeContext {
    language: Option<String>,
    code: String,
}

#[derive(Debug)]
struct TableContext {
    table: TableBlock,
    in_header: bool,
    row: Vec<Vec<InlineSpan>>,
}

impl Default for TableContext {
    fn default() -> Self {
        Self {
            table: TableBlock {
                headers: Vec::new(),
                rows: Vec::new(),
            },
            in_header: false,
            row: Vec::new(),
        }
    }
}

fn split_frontmatter(source: &str) -> (Option<String>, &str) {
    let Some(rest) = source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))
    else {
        return (None, source);
    };
    let mut consumed = source.len() - rest.len();
    let mut title = None;
    for line in rest.split_inclusive('\n') {
        consumed += line.len();
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            return (title, &source[consumed..]);
        }
        if title.is_none()
            && let Some(value) = trimmed.strip_prefix("title:")
        {
            let value = value.trim().trim_matches(['\'', '"']);
            if !value.is_empty() {
                title = Some(value.to_owned());
            }
        }
    }
    (None, source)
}

/// Parses Markdown (or an HTML document) into the model consumed by the native reader.
pub fn parse_document(path: PathBuf, source: String) -> ParsedDocument {
    if is_html_document(&path) {
        return parse_html_document(path, source);
    }

    let mut options = Options::empty();
    options.insert(
        Options::ENABLE_TABLES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_GFM,
    );

    let mut blocks = Vec::new();
    let mut headings = Vec::new();
    let mut inline_stack = Vec::<InlineFrame>::new();
    let mut list_stack = Vec::<ListContext>::new();
    let mut item_stack = Vec::<ItemContext>::new();
    let mut blockquotes = Vec::<QuoteFrame>::new();
    let mut code_block = None::<CodeContext>;
    let mut table = None::<TableContext>;
    let mut html_block = None::<String>;
    let mut footnotes = Vec::<(String, Vec<DocumentBlock>)>::new();
    // While a footnote definition is open, `blocks` holds the footnote body and the main
    // document blocks wait here, so nested pushes need no special routing.
    let mut stashed_main_blocks = None::<(String, Vec<DocumentBlock>)>;
    let (frontmatter_title, markdown_body) = split_frontmatter(&source);

    for event in Parser::new_ext(markdown_body, options) {
        if code_block.is_some() {
            match event {
                Event::End(TagEnd::CodeBlock) => {
                    let code = code_block.take().expect("code block is present");
                    let block = if code
                        .language
                        .as_deref()
                        .is_some_and(|language| language.eq_ignore_ascii_case("mermaid"))
                    {
                        DocumentBlock::MermaidCard { source: code.code }
                    } else {
                        DocumentBlock::CodeBlock {
                            language: code.language,
                            code: code.code,
                        }
                    };
                    push_block(block, &mut blocks, &mut item_stack, &mut blockquotes);
                }
                Event::Text(text)
                | Event::Code(text)
                | Event::Html(text)
                | Event::InlineHtml(text) => code_block
                    .as_mut()
                    .expect("code block is present")
                    .code
                    .push_str(&text),
                Event::SoftBreak | Event::HardBreak => code_block
                    .as_mut()
                    .expect("code block is present")
                    .code
                    .push('\n'),
                _ => {}
            }
            continue;
        }

        if html_block.is_some() {
            match event {
                Event::End(TagEnd::HtmlBlock) => {
                    push_block(
                        DocumentBlock::RawText(html_block.take().expect("HTML block is present")),
                        &mut blocks,
                        &mut item_stack,
                        &mut blockquotes,
                    );
                }
                Event::Html(text) | Event::InlineHtml(text) | Event::Text(text) => html_block
                    .as_mut()
                    .expect("HTML block is present")
                    .push_str(&text),
                Event::SoftBreak | Event::HardBreak => html_block
                    .as_mut()
                    .expect("HTML block is present")
                    .push('\n'),
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(Tag::Paragraph) => {
                inline_stack.push(InlineFrame::new(InlineContainer::Flatten))
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some(mut frame) = inline_stack.pop() {
                    if let Some(image) = frame.take_standalone_image() {
                        push_block(
                            DocumentBlock::Image {
                                alt: image.alt,
                                source: image.source,
                            },
                            &mut blocks,
                            &mut item_stack,
                            &mut blockquotes,
                        );
                    } else {
                        push_paragraph_content(
                            frame.into_spans(),
                            &mut blocks,
                            &mut item_stack,
                            &mut blockquotes,
                        );
                    }
                }
            }
            Event::Start(Tag::Heading { .. }) => {
                inline_stack.push(InlineFrame::new(InlineContainer::Flatten))
            }
            Event::End(TagEnd::Heading(level)) => {
                if let Some(frame) = inline_stack.pop() {
                    let content = frame.into_spans();
                    let level = level as u8;
                    let text = plain_text_for_spans(&content);
                    headings.push(Heading { level, text });
                    push_block(
                        DocumentBlock::Heading { level, content },
                        &mut blocks,
                        &mut item_stack,
                        &mut blockquotes,
                    );
                }
            }
            Event::Start(Tag::BlockQuote(kind)) => blockquotes.push(match kind {
                Some(kind) => QuoteFrame::Alert {
                    kind: kind.into(),
                    children: Vec::new(),
                },
                None => QuoteFrame::Plain(Vec::new()),
            }),
            Event::End(TagEnd::BlockQuote(_)) => {
                if let Some(frame) = blockquotes.pop() {
                    let block = match frame {
                        QuoteFrame::Plain(content) => DocumentBlock::Blockquote(content),
                        QuoteFrame::Alert { kind, children } => {
                            DocumentBlock::Alert { kind, children }
                        }
                    };
                    push_block(block, &mut blocks, &mut item_stack, &mut blockquotes);
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(language) if !language.is_empty() => {
                        Some(language.into_string())
                    }
                    _ => None,
                };
                code_block = Some(CodeContext {
                    language,
                    code: String::new(),
                });
            }
            Event::Start(Tag::HtmlBlock) => html_block = Some(String::new()),
            Event::Start(Tag::List(first_number)) => list_stack.push(ListContext {
                ordered: first_number.is_some(),
                next_number: first_number.unwrap_or(1),
            }),
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let depth = list_stack.len().saturating_sub(1);
                let kind = list_stack.last_mut().map_or(ListKind::Unordered, |list| {
                    if list.ordered {
                        let number = list.next_number;
                        list.next_number += 1;
                        ListKind::Ordered { number }
                    } else {
                        ListKind::Unordered
                    }
                });
                item_stack.push(ItemContext {
                    kind,
                    depth,
                    checked: None,
                    children: Vec::new(),
                });
                push_inline_frame(&mut inline_stack, InlineContainer::Flatten);
            }
            Event::End(TagEnd::Item) => {
                if let (Some(frame), Some(item)) = (inline_stack.pop(), item_stack.last_mut()) {
                    item.push_content(frame.into_spans());
                }
                if let Some(item) = item_stack.pop() {
                    let item_block = item.into_block();
                    if let Some(parent) = item_stack.last_mut() {
                        flush_item_inline_frame(&mut inline_stack, parent);
                        parent.push_block(item_block);
                    } else {
                        push_block(item_block, &mut blocks, &mut item_stack, &mut blockquotes);
                    }
                }
            }
            Event::Start(Tag::Table(_)) => table = Some(TableContext::default()),
            Event::End(TagEnd::Table) => {
                if let Some(table) = table.take() {
                    push_block(
                        DocumentBlock::Table(table.table),
                        &mut blocks,
                        &mut item_stack,
                        &mut blockquotes,
                    );
                }
            }
            Event::Start(Tag::TableHead) => {
                if let Some(table) = table.as_mut() {
                    table.in_header = true;
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(table) = table.as_mut() {
                    table.table.headers = std::mem::take(&mut table.row);
                    table.in_header = false;
                }
            }
            Event::Start(Tag::TableRow) => {
                if let Some(table) = table.as_mut() {
                    table.row.clear();
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(table) = table.as_mut() {
                    let row = std::mem::take(&mut table.row);
                    if table.in_header {
                        table.table.headers = row;
                    } else {
                        table.table.rows.push(row);
                    }
                }
            }
            Event::Start(Tag::TableCell) => {
                inline_stack.push(InlineFrame::new(InlineContainer::Flatten))
            }
            Event::End(TagEnd::TableCell) => {
                if let (Some(frame), Some(table)) = (inline_stack.pop(), table.as_mut()) {
                    table.row.push(frame.into_spans());
                }
            }
            Event::Start(Tag::Emphasis) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Emphasis)
            }
            Event::End(TagEnd::Emphasis) => pop_inline_frame(&mut inline_stack),
            Event::Start(Tag::Strong) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Strong)
            }
            Event::End(TagEnd::Strong) => pop_inline_frame(&mut inline_stack),
            Event::Start(Tag::Strikethrough) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Strikethrough)
            }
            Event::Start(Tag::Superscript | Tag::Subscript) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Flatten)
            }
            Event::End(TagEnd::Strikethrough | TagEnd::Superscript | TagEnd::Subscript) => {
                pop_inline_frame(&mut inline_stack)
            }
            Event::Start(Tag::Link { dest_url, .. }) => push_inline_frame(
                &mut inline_stack,
                InlineContainer::Link(dest_url.into_string()),
            ),
            Event::End(TagEnd::Link) => pop_inline_frame(&mut inline_stack),
            Event::Start(Tag::Image { dest_url, .. }) => push_inline_frame(
                &mut inline_stack,
                InlineContainer::Image(dest_url.into_string()),
            ),
            Event::End(TagEnd::Image) => {
                if let Some(image) = pop_image_frame(&mut inline_stack) {
                    push_block(image, &mut blocks, &mut item_stack, &mut blockquotes);
                }
            }
            Event::Text(text) => {
                push_inline_span(&mut inline_stack, InlineSpan::Text(text.into_string()))
            }
            Event::Code(code) => {
                push_inline_span(&mut inline_stack, InlineSpan::Code(code.into_string()))
            }
            Event::InlineHtml(html) => {
                push_inline_span(&mut inline_stack, InlineSpan::Text(html.into_string()))
            }
            Event::Html(html) => push_block(
                DocumentBlock::RawText(html.into_string()),
                &mut blocks,
                &mut item_stack,
                &mut blockquotes,
            ),
            Event::SoftBreak => push_inline_span(&mut inline_stack, InlineSpan::SoftBreak),
            Event::HardBreak => push_inline_span(&mut inline_stack, InlineSpan::HardBreak),
            Event::Rule => push_block(
                DocumentBlock::ThematicBreak,
                &mut blocks,
                &mut item_stack,
                &mut blockquotes,
            ),
            Event::TaskListMarker(checked) => {
                if let Some(item) = item_stack.last_mut() {
                    item.checked = Some(checked);
                }
            }
            Event::FootnoteReference(label) => push_inline_span(
                &mut inline_stack,
                InlineSpan::FootnoteRef {
                    label: label.into_string(),
                },
            ),
            Event::Start(Tag::FootnoteDefinition(label)) => {
                let mut main_blocks = Vec::new();
                std::mem::swap(&mut blocks, &mut main_blocks);
                stashed_main_blocks = Some((label.into_string(), main_blocks));
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                if let Some((label, mut main_blocks)) = stashed_main_blocks.take() {
                    std::mem::swap(&mut blocks, &mut main_blocks);
                    footnotes.push((label, main_blocks));
                }
            }
            Event::InlineMath(math) | Event::DisplayMath(math) => {
                push_inline_span(&mut inline_stack, InlineSpan::Text(math.into_string()))
            }
            Event::Start(_) | Event::End(_) => {}
        }
    }

    if let Some((label, mut main_blocks)) = stashed_main_blocks.take() {
        std::mem::swap(&mut blocks, &mut main_blocks);
        footnotes.push((label, main_blocks));
    }
    if !footnotes.is_empty() {
        blocks.push(DocumentBlock::FootnoteSection { notes: footnotes });
    }

    let title = headings
        .iter()
        .find(|heading| heading.level == 1)
        .map(|heading| heading.text.clone())
        .filter(|title| !title.is_empty())
        .or_else(|| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "Untitled".into());

    ParsedDocument {
        path,
        title,
        frontmatter_title,
        source,
        blocks,
        headings,
    }
}

fn parse_html_document(path: PathBuf, source: String) -> ParsedDocument {
    let parent = path.parent().unwrap_or_else(|| Path::new("")).to_owned();
    let blocks = crate::html::html_to_blocks(&source, &parent);
    let mut headings = Vec::new();
    collect_headings(&blocks, &mut headings);

    let title = headings
        .iter()
        .find(|heading| heading.level == 1)
        .map(|heading| heading.text.clone())
        .filter(|title| !title.is_empty())
        .or_else(|| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "Untitled".into());

    ParsedDocument {
        path,
        title,
        frontmatter_title: None,
        source,
        blocks,
        headings,
    }
}

fn collect_headings(blocks: &[DocumentBlock], headings: &mut Vec<Heading>) {
    for block in blocks {
        match block {
            DocumentBlock::Heading { level, content } => headings.push(Heading {
                level: *level,
                text: plain_text_for_spans(content),
            }),
            DocumentBlock::ListItem { children, .. }
            | DocumentBlock::TaskItem { children, .. }
            | DocumentBlock::Alert { children, .. } => collect_headings(children, headings),
            _ => {}
        }
    }
}

/// Resolves a local Markdown target without checking whether it exists on disk.
pub fn resolve_local_target(document_path: &Path, target: &str) -> Option<PathBuf> {
    let path_end = target.find(['?', '#']).unwrap_or(target.len());
    let target = &target[..path_end];
    if target.is_empty() || has_uri_scheme(target) {
        return None;
    }
    let target = percent_decode_url_path(target)?;

    let path = Path::new(&target);
    let joined = if path.is_absolute() {
        path.to_owned()
    } else {
        document_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(path)
    };
    Some(normalize_lexically(&joined))
}

fn percent_decode_url_path(target: &str) -> Option<String> {
    let bytes = target.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }

        let high = hex_value(*bytes.get(index + 1)?)?;
        let low = hex_value(*bytes.get(index + 2)?)?;
        let byte = high * 16 + low;
        if matches!(byte, 0 | b'/' | b'\\') {
            return None;
        }
        decoded.push(byte);
        index += 3;
    }

    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn push_paragraph_content(
    content: Vec<InlineSpan>,
    blocks: &mut Vec<DocumentBlock>,
    item_stack: &mut [ItemContext],
    blockquotes: &mut [QuoteFrame],
) {
    if content.is_empty() {
        return;
    }
    if let Some(item) = item_stack.last_mut() {
        item.push_content(content);
    } else if let Some(frame) = blockquotes.last_mut() {
        match frame {
            QuoteFrame::Plain(spans) => append_inline_content(spans, content),
            QuoteFrame::Alert { children, .. } => children.push(DocumentBlock::Paragraph(content)),
        }
    } else {
        push_block(
            DocumentBlock::Paragraph(content),
            blocks,
            item_stack,
            blockquotes,
        );
    }
}

fn push_block(
    block: DocumentBlock,
    blocks: &mut Vec<DocumentBlock>,
    item_stack: &mut [ItemContext],
    blockquotes: &mut [QuoteFrame],
) {
    if let Some(item) = item_stack.last_mut() {
        item.push_block(block);
    } else if let Some(frame) = blockquotes.last_mut() {
        match frame {
            QuoteFrame::Plain(spans) => {
                let text = block.plain_text();
                if !text.is_empty() {
                    append_inline_content(spans, vec![InlineSpan::Text(text)]);
                }
            }
            QuoteFrame::Alert { children, .. } => children.push(block),
        }
    } else {
        blocks.push(block);
    }
}

fn flush_item_inline_frame(stack: &mut [InlineFrame], item: &mut ItemContext) {
    if let Some(frame) = stack.last_mut() {
        frame.flush_pending_image();
        item.push_content(std::mem::take(&mut frame.spans));
    }
}

fn append_inline_content(destination: &mut Vec<InlineSpan>, content: Vec<InlineSpan>) {
    if !destination.is_empty() {
        destination.push(InlineSpan::SoftBreak);
    }
    destination.extend(content);
}

fn push_inline_frame(stack: &mut Vec<InlineFrame>, container: InlineContainer) {
    stack.push(InlineFrame::new(container));
}

fn pop_inline_frame(stack: &mut Vec<InlineFrame>) {
    let Some(frame) = stack.pop() else {
        return;
    };

    match frame.container.clone() {
        InlineContainer::Emphasis => {
            push_inline_span(stack, InlineSpan::Emphasis(frame.into_spans()))
        }
        InlineContainer::Strong => push_inline_span(stack, InlineSpan::Strong(frame.into_spans())),
        InlineContainer::Strikethrough => {
            push_inline_span(stack, InlineSpan::Strikethrough(frame.into_spans()))
        }
        InlineContainer::Link(target) => push_inline_span(
            stack,
            InlineSpan::Link {
                label: frame.into_spans(),
                target,
            },
        ),
        InlineContainer::Image(_) => unreachable!("images are ended by pop_image_frame"),
        InlineContainer::Flatten => {
            if let Some(parent) = stack.last_mut() {
                parent.spans.extend(frame.into_spans());
            }
        }
    }
}

fn pop_image_frame(stack: &mut Vec<InlineFrame>) -> Option<DocumentBlock> {
    let frame = stack.pop()?;
    let InlineContainer::Image(source) = frame.container.clone() else {
        return None;
    };
    let image = ImageData {
        alt: plain_text_for_spans(&frame.into_spans()),
        source,
    };

    if let Some(parent) = stack.last_mut() {
        if parent.spans.is_empty() && parent.pending_image.is_none() {
            parent.pending_image = Some(image);
        } else {
            parent.flush_pending_image();
            parent.spans.push(InlineSpan::Text(image.alt));
        }
        None
    } else {
        Some(DocumentBlock::Image {
            alt: image.alt,
            source: image.source,
        })
    }
}

fn push_inline_span(stack: &mut [InlineFrame], span: InlineSpan) {
    if let Some(frame) = stack.last_mut() {
        frame.push_span(span);
    }
}

fn plain_text_for_spans(spans: &[InlineSpan]) -> String {
    spans.iter().map(InlineSpan::plain_text).collect()
}

fn painted_plain_text_for_spans(spans: &[InlineSpan]) -> String {
    spans.iter().map(InlineSpan::painted_plain_text).collect()
}

fn plain_text_for_blocks(blocks: &[DocumentBlock]) -> String {
    blocks
        .iter()
        .map(DocumentBlock::plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn painted_plain_text_for_blocks(blocks: &[DocumentBlock]) -> String {
    blocks
        .iter()
        .map(DocumentBlock::painted_plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn has_uri_scheme(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    matches!(characters.next(), Some(character) if character.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

pub(crate) fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if normalized.file_name().is_some() {
                    normalized.pop();
                } else if !normalized.has_root() {
                    normalized.push("..");
                }
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn frontmatter_title_is_extracted_and_not_rendered_as_markdown() {
        let parsed = parse_document(
            PathBuf::from("/tmp/showcase.md"),
            "---\ntitle: Reader title\n---\n# Visible heading\n".into(),
        );

        assert_eq!(parsed.frontmatter_title.as_deref(), Some("Reader title"));
        assert_eq!(parsed.blocks.len(), 1);
        assert_eq!(parsed.title, "Visible heading");
    }

    #[test]
    fn recognizes_supported_extensions_case_insensitively() {
        assert!(is_supported_markdown(Path::new("README.md")));
        assert!(is_supported_markdown(Path::new("notes.MARKDOWN")));
        assert!(is_supported_markdown(Path::new("component.MdX")));
        assert!(!is_supported_markdown(Path::new("notes.txt")));
    }

    #[test]
    fn loads_utf8_and_returns_a_canonical_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.md");
        fs::write(&path, "# Hello\n").unwrap();

        let loaded = load_source(&path).unwrap();

        assert_eq!(loaded.canonical_path, path.canonicalize().unwrap());
        assert_eq!(loaded.source, "# Hello\n");
    }

    #[test]
    fn reports_invalid_utf8_without_debug_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.md");
        fs::write(&path, [0xff, 0xfe]).unwrap();

        let error = load_source(&path).unwrap_err();

        assert!(matches!(error, DocumentError::InvalidUtf8 { .. }));
        assert_eq!(error.title(), "This file is not UTF-8");
    }

    #[test]
    fn parses_reader_blocks_and_inline_styles() {
        let parsed = parse_document(
            PathBuf::from("/tmp/guide.md"),
            "# Guide\n\nHello *quiet* **reader** with `code`.\n\n- [x] Done\n\n```rust\nlet n = 1;\n```\n".into(),
        );

        assert_eq!(parsed.title, "Guide");
        assert_eq!(
            parsed.headings,
            vec![Heading {
                level: 1,
                text: "Guide".into()
            }]
        );
        assert_eq!(
            parsed.blocks[0],
            DocumentBlock::Heading {
                level: 1,
                content: vec![InlineSpan::Text("Guide".into())],
            }
        );
        assert!(matches!(parsed.blocks[1], DocumentBlock::Paragraph(_)));
        assert_eq!(
            parsed.blocks[2],
            DocumentBlock::TaskItem {
                checked: true,
                depth: 0,
                children: vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                    "Done".into()
                )])],
            }
        );
        assert_eq!(
            parsed.blocks[3],
            DocumentBlock::CodeBlock {
                language: Some("rust".into()),
                code: "let n = 1;\n".into(),
            }
        );
    }

    #[test]
    fn keeps_raw_html_and_mdx_inert_and_readable() {
        let parsed = parse_document(
            PathBuf::from("/tmp/component.mdx"),
            "<aside>Note</aside>\n\n<Component value={1} />".into(),
        );
        let visible = parsed.plain_text();
        assert!(visible.contains("<aside>Note</aside>"));
        assert!(visible.contains("<Component value={1} />"));
    }

    #[test]
    fn strikethrough_parses_as_a_strikethrough_span() {
        let parsed = parse_document(
            PathBuf::from("/tmp/strike.md"),
            "before ~~gone~~ after\n".into(),
        );

        assert_eq!(
            parsed.blocks,
            vec![DocumentBlock::Paragraph(vec![
                InlineSpan::Text("before ".into()),
                InlineSpan::Strikethrough(vec![InlineSpan::Text("gone".into())]),
                InlineSpan::Text(" after".into()),
            ])]
        );
    }

    #[test]
    fn gfm_note_blockquotes_become_alerts_with_child_blocks() {
        let parsed = parse_document(
            PathBuf::from("/tmp/alert.md"),
            "> [!NOTE]\n> hello\n\n> plain quote\n".into(),
        );

        assert_eq!(
            parsed.blocks[0],
            DocumentBlock::Alert {
                kind: AlertKind::Note,
                children: vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                    "hello".into()
                )])],
            }
        );
        assert_eq!(
            parsed.blocks[1],
            DocumentBlock::Blockquote(vec![InlineSpan::Text("plain quote".into())]),
        );
    }

    #[test]
    fn footnotes_produce_a_ref_and_a_trailing_section_without_leaked_paragraphs() {
        let parsed = parse_document(
            PathBuf::from("/tmp/notes.md"),
            "Some claim.[^1]\n\nMore prose.\n\n[^1]: The evidence.\n".into(),
        );

        assert_eq!(
            parsed.blocks[0],
            DocumentBlock::Paragraph(vec![
                InlineSpan::Text("Some claim.".into()),
                InlineSpan::FootnoteRef { label: "1".into() },
            ])
        );
        assert_eq!(
            parsed.blocks[1],
            DocumentBlock::Paragraph(vec![InlineSpan::Text("More prose.".into())]),
        );
        assert_eq!(
            parsed.blocks[2],
            DocumentBlock::FootnoteSection {
                notes: vec![(
                    "1".into(),
                    vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                        "The evidence.".into()
                    )])],
                )],
            }
        );
        assert_eq!(parsed.blocks.len(), 3, "footnote body must not leak");
    }

    #[test]
    fn mermaid_fences_become_mermaid_cards() {
        let parsed = parse_document(
            PathBuf::from("/tmp/diagram.md"),
            "```Mermaid\ngraph TD;\n```\n\n```rust\nlet n = 1;\n```\n".into(),
        );

        assert_eq!(
            parsed.blocks[0],
            DocumentBlock::MermaidCard {
                source: "graph TD;\n".into(),
            }
        );
        assert!(matches!(parsed.blocks[1], DocumentBlock::CodeBlock { .. }));
    }

    #[test]
    fn painted_plain_text_renders_soft_breaks_as_spaces() {
        let span_level = InlineSpan::SoftBreak;
        assert_eq!(span_level.plain_text(), "\n");
        assert_eq!(span_level.painted_plain_text(), " ");

        let block = DocumentBlock::Paragraph(vec![
            InlineSpan::Text("one".into()),
            InlineSpan::SoftBreak,
            InlineSpan::Text("two".into()),
            InlineSpan::HardBreak,
            InlineSpan::Text("three".into()),
        ]);
        assert_eq!(block.painted_plain_text(), "one two\nthree");
    }

    #[test]
    fn footnote_refs_display_as_superscript_digits_or_bracketed_labels() {
        assert_eq!(footnote_ref_display("12"), "¹²");
        assert_eq!(footnote_ref_display("note"), "[note]");
    }

    #[test]
    fn html_documents_are_supported_and_parse_through_the_html_path() {
        assert!(is_supported_document(Path::new("page.HTML")));
        assert!(is_supported_document(Path::new("page.htm")));
        assert!(is_supported_document(Path::new("notes.md")));
        assert!(!is_supported_document(Path::new("notes.txt")));
        assert!(!is_supported_markdown(Path::new("page.html")));

        let parsed = parse_document(
            PathBuf::from("/tmp/page.html"),
            "<h1># Not markdown</h1><p>Body</p>".into(),
        );
        assert_eq!(parsed.title, "# Not markdown");
        assert_eq!(
            parsed.blocks,
            vec![
                DocumentBlock::Heading {
                    level: 1,
                    content: vec![InlineSpan::Text("# Not markdown".into())],
                },
                DocumentBlock::Paragraph(vec![InlineSpan::Text("Body".into())]),
            ]
        );
        assert_eq!(
            parsed.headings,
            vec![Heading {
                level: 1,
                text: "# Not markdown".into(),
            }]
        );
    }

    #[test]
    fn load_source_accepts_html_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("page.html");
        fs::write(&path, "<h1>Hello</h1>\n").unwrap();

        let loaded = load_source(&path).unwrap();

        assert_eq!(loaded.source, "<h1>Hello</h1>\n");
        assert_eq!(loaded.canonical_path, path.canonicalize().unwrap());
    }

    #[test]
    fn resolves_relative_targets_against_the_document() {
        assert_eq!(
            resolve_local_target(Path::new("/vault/guides/start.md"), "../images/hero.png"),
            Some(PathBuf::from("/vault/images/hero.png")),
        );
        assert_eq!(
            resolve_local_target(Path::new("/vault/start.md"), "https://mdow.dev"),
            None
        );
    }

    #[test]
    fn resolves_percent_encoded_url_paths_without_decoding_separators() {
        let document = Path::new("/vault/guides/start.md");

        assert_eq!(
            resolve_local_target(document, "../images/hero%20shot.png?raw=1#preview"),
            Some(PathBuf::from("/vault/images/hero shot.png")),
        );
        assert_eq!(
            resolve_local_target(document, "caf%C3%A9.md"),
            Some(PathBuf::from("/vault/guides/café.md")),
        );
        assert_eq!(resolve_local_target(document, "bad%2.md"), None);
        assert_eq!(resolve_local_target(document, "..%2Fsecret.md"), None);
        assert_eq!(resolve_local_target(document, "..%5Csecret.md"), None);
    }

    #[test]
    fn preserves_nested_list_items_inside_their_parent() {
        let parsed = parse_document(
            PathBuf::from("/tmp/list.md"),
            "- Parent\n  - Child\n".into(),
        );

        assert_eq!(
            parsed.blocks,
            vec![DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: vec![
                    DocumentBlock::Paragraph(vec![InlineSpan::Text("Parent".into())]),
                    DocumentBlock::ListItem {
                        kind: ListKind::Unordered,
                        depth: 1,
                        children: vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                            "Child".into()
                        )])],
                    },
                ],
            }]
        );
    }

    #[test]
    fn structured_lists_preserve_ordered_numbers() {
        let parsed = parse_document(
            PathBuf::from("/tmp/ordered-list.md"),
            "3. Third\n4. Fourth\n".into(),
        );

        let DocumentBlock::ListItem {
            kind: ListKind::Ordered { number: first },
            depth: first_depth,
            ..
        } = &parsed.blocks[0]
        else {
            panic!("first ordered item");
        };
        let DocumentBlock::ListItem {
            kind: ListKind::Ordered { number: second },
            depth: second_depth,
            ..
        } = &parsed.blocks[1]
        else {
            panic!("second ordered item");
        };

        assert_eq!((*first, *first_depth), (3, 0));
        assert_eq!((*second, *second_depth), (4, 0));
    }

    #[test]
    fn preserves_table_headers_and_body_rows() {
        let parsed = parse_document(
            PathBuf::from("/tmp/table.md"),
            "| Name | Value |\n| --- | --- |\n| one | 1 |\n| two | 2 |\n".into(),
        );

        assert_eq!(
            parsed.blocks,
            vec![DocumentBlock::Table(TableBlock {
                headers: vec![
                    vec![InlineSpan::Text("Name".into())],
                    vec![InlineSpan::Text("Value".into())],
                ],
                rows: vec![
                    vec![
                        vec![InlineSpan::Text("one".into())],
                        vec![InlineSpan::Text("1".into())],
                    ],
                    vec![
                        vec![InlineSpan::Text("two".into())],
                        vec![InlineSpan::Text("2".into())],
                    ],
                ],
            })]
        );
    }

    #[test]
    fn keeps_blockquote_nested_blocks_inert_and_in_source_order() {
        let parsed = parse_document(
            PathBuf::from("/tmp/quote.md"),
            "> Intro\n>\n> - Item\n>\n> ```rust\n> let n = 1;\n> ```\n>\n> <aside>Raw</aside>\n>\n> > Nested\n\nAfter\n"
                .into(),
        );

        assert!(matches!(parsed.blocks[0], DocumentBlock::Blockquote(_)));
        assert!(matches!(parsed.blocks[1], DocumentBlock::Paragraph(_)));
        assert_eq!(parsed.blocks.len(), 2);

        let quote = parsed.blocks[0].plain_text();
        let positions = [
            "Intro",
            "Item",
            "let n = 1;",
            "<aside>Raw</aside>",
            "Nested",
        ]
        .map(|text| quote.find(text).unwrap());
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn keeps_inline_images_in_their_paragraph_order() {
        let parsed = parse_document(
            PathBuf::from("/tmp/image.md"),
            "before ![alt](image.png) after".into(),
        );

        assert_eq!(
            parsed.blocks,
            vec![DocumentBlock::Paragraph(vec![
                InlineSpan::Text("before ".into()),
                InlineSpan::Text("alt".into()),
                InlineSpan::Text(" after".into()),
            ])]
        );
    }

    #[test]
    fn preserves_one_list_item_around_ordered_paragraph_code_paragraph_children() {
        let parsed = parse_document(
            PathBuf::from("/tmp/interleaved-list.md"),
            "- before\n\n  ```rust\n  let n = 1;\n  ```\n\n  after\n".into(),
        );

        assert_eq!(
            parsed.blocks,
            vec![DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: vec![
                    DocumentBlock::Paragraph(vec![InlineSpan::Text("before".into())]),
                    DocumentBlock::CodeBlock {
                        language: Some("rust".into()),
                        code: "let n = 1;\n".into(),
                    },
                    DocumentBlock::Paragraph(vec![InlineSpan::Text("after".into())]),
                ],
            }]
        );
        assert_eq!(
            parsed
                .blocks
                .iter()
                .filter(|block| matches!(block, DocumentBlock::ListItem { .. }))
                .count(),
            1,
        );
        assert_eq!(parsed.plain_text(), "before\nlet n = 1;\n\nafter");
    }
}
