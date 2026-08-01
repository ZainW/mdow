use std::path::{Component, Path, PathBuf};

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

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
                "Mdow opens .md, .markdown, and .mdx files. Choose a Markdown file or drop a folder."
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

pub fn load_source(path: &Path) -> Result<LoadedSource, DocumentError> {
    if !is_supported_markdown(path) {
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
    Code(String),
    Link {
        label: Vec<InlineSpan>,
        target: String,
    },
    SoftBreak,
    HardBreak,
}

impl InlineSpan {
    pub fn plain_text(&self) -> String {
        match self {
            Self::Text(text) | Self::Code(text) => text.clone(),
            Self::Emphasis(content) | Self::Strong(content) => plain_text_for_spans(content),
            Self::Link { label, .. } => plain_text_for_spans(label),
            Self::SoftBreak | Self::HardBreak => "\n".into(),
        }
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
        content: Vec<InlineSpan>,
    },
    TaskItem {
        checked: bool,
        depth: usize,
        content: Vec<InlineSpan>,
    },
    Blockquote(Vec<InlineSpan>),
    ThematicBreak,
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Table(TableBlock),
    Image {
        alt: String,
        source: String,
    },
    RawText(String),
}

impl DocumentBlock {
    fn plain_text(&self) -> String {
        match self {
            Self::Heading { content, .. }
            | Self::Paragraph(content)
            | Self::Blockquote(content)
            | Self::ListItem { content, .. }
            | Self::TaskItem { content, .. } => plain_text_for_spans(content),
            Self::ThematicBreak => String::new(),
            Self::CodeBlock { code, .. } => code.clone(),
            Self::Table(table) => table.plain_text(),
            Self::Image { alt, .. } | Self::RawText(alt) => alt.clone(),
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
        std::iter::once(&self.headers)
            .chain(self.rows.iter())
            .map(|row| {
                row.iter()
                    .map(|cell| plain_text_for_spans(cell))
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
    Link(String),
    Image(String),
    Flatten,
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
    content: Vec<InlineSpan>,
    children: Vec<DocumentBlock>,
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

/// Parses Markdown into the model consumed by the native reader.
pub fn parse_document(path: PathBuf, source: String) -> ParsedDocument {
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
    let mut blockquotes = Vec::<Vec<InlineSpan>>::new();
    let mut code_block = None::<CodeContext>;
    let mut table = None::<TableContext>;
    let mut html_block = None::<String>;

    for event in Parser::new_ext(&source, options) {
        if code_block.is_some() {
            match event {
                Event::End(TagEnd::CodeBlock) => {
                    let code = code_block.take().expect("code block is present");
                    push_block(
                        DocumentBlock::CodeBlock {
                            language: code.language,
                            code: code.code,
                        },
                        &mut blocks,
                        &mut item_stack,
                        &mut blockquotes,
                    );
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
            Event::Start(Tag::BlockQuote(_)) => blockquotes.push(Vec::new()),
            Event::End(TagEnd::BlockQuote(_)) => {
                if let Some(content) = blockquotes.pop() {
                    push_block(
                        DocumentBlock::Blockquote(content),
                        &mut blocks,
                        &mut item_stack,
                        &mut blockquotes,
                    );
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
                    content: Vec::new(),
                    children: Vec::new(),
                });
                push_inline_frame(&mut inline_stack, InlineContainer::Flatten);
            }
            Event::End(TagEnd::Item) => {
                if let (Some(frame), Some(item)) = (inline_stack.pop(), item_stack.last_mut()) {
                    append_inline_content(&mut item.content, frame.into_spans());
                }
                if let Some(item) = item_stack.pop() {
                    let block = match item.checked {
                        Some(checked) => DocumentBlock::TaskItem {
                            checked,
                            depth: item.depth,
                            content: item.content,
                        },
                        None => DocumentBlock::ListItem {
                            kind: item.kind,
                            depth: item.depth,
                            content: item.content,
                        },
                    };
                    let mut item_blocks = vec![block];
                    item_blocks.extend(item.children);
                    if let Some(parent) = item_stack.last_mut() {
                        parent.children.extend(item_blocks);
                    } else {
                        for block in item_blocks {
                            push_block(block, &mut blocks, &mut item_stack, &mut blockquotes);
                        }
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
            Event::Start(Tag::Strikethrough | Tag::Superscript | Tag::Subscript) => {
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
            Event::FootnoteReference(label) => {
                push_inline_span(&mut inline_stack, InlineSpan::Text(format!("[^{label}]")))
            }
            Event::InlineMath(math) | Event::DisplayMath(math) => {
                push_inline_span(&mut inline_stack, InlineSpan::Text(math.into_string()))
            }
            Event::Start(_) | Event::End(_) => {}
        }
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
        source,
        blocks,
        headings,
    }
}

/// Resolves a local Markdown target without checking whether it exists on disk.
pub fn resolve_local_target(document_path: &Path, target: &str) -> Option<PathBuf> {
    let target = target.split('#').next().unwrap_or_default();
    if target.is_empty() || has_uri_scheme(target) {
        return None;
    }

    let path = Path::new(target);
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

fn push_paragraph_content(
    content: Vec<InlineSpan>,
    blocks: &mut Vec<DocumentBlock>,
    item_stack: &mut [ItemContext],
    blockquotes: &mut [Vec<InlineSpan>],
) {
    if content.is_empty() {
        return;
    }
    if let Some(item) = item_stack.last_mut() {
        append_inline_content(&mut item.content, content);
    } else if let Some(blockquote) = blockquotes.last_mut() {
        append_inline_content(blockquote, content);
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
    blockquotes: &mut [Vec<InlineSpan>],
) {
    if let Some(item) = item_stack.last_mut() {
        item.children.push(block);
    } else if let Some(blockquote) = blockquotes.last_mut() {
        let text = block.plain_text();
        if !text.is_empty() {
            append_inline_content(blockquote, vec![InlineSpan::Text(text)]);
        }
    } else {
        blocks.push(block);
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

fn has_uri_scheme(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    matches!(characters.next(), Some(character) if character.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

fn normalize_lexically(path: &Path) -> PathBuf {
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
                content: vec![InlineSpan::Text("Done".into())],
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
    fn emits_nested_list_items_after_their_parent() {
        let parsed = parse_document(
            PathBuf::from("/tmp/list.md"),
            "- Parent\n  - Child\n".into(),
        );

        assert_eq!(
            parsed.blocks,
            vec![
                DocumentBlock::ListItem {
                    kind: ListKind::Unordered,
                    depth: 0,
                    content: vec![InlineSpan::Text("Parent".into())],
                },
                DocumentBlock::ListItem {
                    kind: ListKind::Unordered,
                    depth: 1,
                    content: vec![InlineSpan::Text("Child".into())],
                },
            ]
        );
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
}
