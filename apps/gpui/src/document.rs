use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

pub use pulldown_cmark::Alignment;
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
    /// Map outline order to the containing virtualized reader block, including nested headings.
    pub fn heading_block(&self, heading_index: usize) -> Option<usize> {
        let mut remaining = heading_index;
        for (index, block) in self.blocks.iter().enumerate() {
            let count = heading_count(block);
            if remaining < count {
                return Some(index);
            }
            remaining -= count;
        }
        None
    }

    /// The reader block holding each outline heading, in outline order (non-decreasing).
    pub fn heading_blocks(&self) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .flat_map(|(index, block)| std::iter::repeat_n(index, heading_count(block)))
            .collect()
    }

    pub fn anchor_block(&self, fragment: &str) -> Option<usize> {
        let fragment = percent_decode_url_path(fragment)?;
        if fragment.is_empty() {
            return (!self.blocks.is_empty()).then_some(0);
        }
        if let Some(label) = fragment.strip_prefix("fn-")
            && let Some(block) = self.footnote_block(label)
        {
            return Some(block);
        }
        if let Some(label) = fragment.strip_prefix("fnref-")
            && let Some(block) = self.footnote_ref_block(label)
        {
            return Some(block);
        }
        let mut used = std::collections::HashSet::new();
        for (index, heading) in self.headings.iter().enumerate() {
            let base: String = heading
                .text
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
                .map(|c| if c.is_whitespace() { '-' } else { c })
                .collect();
            let mut slug = base.clone();
            let mut suffix = 0;
            while !used.insert(slug.clone()) {
                suffix += 1;
                slug = format!("{base}-{suffix}");
            }
            if slug == fragment {
                return self.heading_block(index);
            }
        }
        None
    }

    /// The reader block holding the footnote section entry for `label`.
    pub fn footnote_block(&self, label: &str) -> Option<usize> {
        self.blocks.iter().position(|block| {
            matches!(block, DocumentBlock::FootnoteSection { notes }
                if notes.iter().any(|(note, _)| note == label))
        })
    }

    /// The reader block holding the first reference to footnote `label`.
    pub fn footnote_ref_block(&self, label: &str) -> Option<usize> {
        self.blocks
            .iter()
            .position(|block| block_references_footnote(block, label))
    }

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
    /// TeX math: `$...$` (`display: false`) or `$$...$$` (`display: true`).
    Math {
        tex: String,
        display: bool,
    },
    /// The `↩` link at the end of a footnote that jumps back to its first reference.
    FootnoteBackref {
        label: String,
    },
    Superscript(Vec<InlineSpan>),
    Subscript(Vec<InlineSpan>),
    /// `<mark>` from inline HTML.
    Mark(Vec<InlineSpan>),
    /// `<kbd>` from inline HTML.
    Kbd(Vec<InlineSpan>),
    SoftBreak,
    HardBreak,
}

/// Raised or lowered text. GPUI text runs cannot shift the baseline or change size inside one
/// paragraph, so text whose every character has a Unicode super/subscript form is painted with
/// those characters; anything else keeps its characters and asks the font for its `sups`/`subs`
/// OpenType feature (Inter supports both; fonts without it paint regular glyphs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    Super,
    Sub,
}

fn script_char(character: char, script: Script) -> Option<char> {
    const SUPER_DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    const SUB_DIGITS: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
    if let Some(digit) = character.to_digit(10) {
        return Some(match script {
            Script::Super => SUPER_DIGITS[digit as usize],
            Script::Sub => SUB_DIGITS[digit as usize],
        });
    }
    match (script, character) {
        (_, ' ') => Some(' '),
        (Script::Super, '+') => Some('⁺'),
        (Script::Super, '-') => Some('⁻'),
        (Script::Super, '=') => Some('⁼'),
        (Script::Super, '(') => Some('⁽'),
        (Script::Super, ')') => Some('⁾'),
        (Script::Super, 'n') => Some('ⁿ'),
        (Script::Super, 'i') => Some('ⁱ'),
        (Script::Sub, '+') => Some('₊'),
        (Script::Sub, '-') => Some('₋'),
        (Script::Sub, '=') => Some('₌'),
        (Script::Sub, '(') => Some('₍'),
        (Script::Sub, ')') => Some('₎'),
        (Script::Sub, 'a') => Some('ₐ'),
        (Script::Sub, 'e') => Some('ₑ'),
        (Script::Sub, 'o') => Some('ₒ'),
        (Script::Sub, 'x') => Some('ₓ'),
        (Script::Sub, 'h') => Some('ₕ'),
        (Script::Sub, 'k') => Some('ₖ'),
        (Script::Sub, 'l') => Some('ₗ'),
        (Script::Sub, 'm') => Some('ₘ'),
        (Script::Sub, 'n') => Some('ₙ'),
        (Script::Sub, 'p') => Some('ₚ'),
        (Script::Sub, 's') => Some('ₛ'),
        (Script::Sub, 't') => Some('ₜ'),
        _ => None,
    }
}

/// The painted form of super/subscript text: Unicode script characters when every character
/// has one, otherwise the text unchanged (see [`Script`]).
pub fn script_text(text: &str, script: Script) -> std::borrow::Cow<'_, str> {
    if text.trim().is_empty() {
        return text.into();
    }
    match text
        .chars()
        .map(|character| script_char(character, script))
        .collect::<Option<String>>()
    {
        Some(mapped) => mapped.into(),
        None => text.into(),
    }
}

/// Whether [`script_text`] maps this text to Unicode script characters.
pub fn script_is_unicode(text: &str, script: Script) -> bool {
    matches!(script_text(text, script), std::borrow::Cow::Owned(_))
}

pub const FOOTNOTE_BACKREF: &str = "↩";

/// Link target of a footnote reference; resolved by [`ParsedDocument::anchor_block`].
pub fn footnote_target(label: &str) -> String {
    format!("#fn-{label}")
}

/// Link target of a footnote's backlink; resolved by [`ParsedDocument::anchor_block`].
pub fn footnote_backref_target(label: &str) -> String {
    format!("#fnref-{label}")
}

impl InlineSpan {
    pub fn plain_text(&self) -> String {
        self.text_with("\n", None)
    }

    /// The text find searches. It stays byte-aligned with what the reader paints, but soft breaks
    /// (painted as line breaks) fold to spaces so a one-line query can match across them.
    pub fn find_text(&self) -> String {
        self.text_with(" ", None)
    }

    fn text_with(&self, soft_break: &str, script: Option<Script>) -> String {
        let children = |content: &[InlineSpan], script: Option<Script>| {
            content
                .iter()
                .map(|span| span.text_with(soft_break, script))
                .collect::<String>()
        };
        match self {
            Self::Text(text) => match script {
                Some(script) => script_text(text, script).into_owned(),
                None => text.clone(),
            },
            Self::Code(text) => text.clone(),
            Self::Emphasis(content)
            | Self::Strong(content)
            | Self::Strikethrough(content)
            | Self::Mark(content)
            | Self::Kbd(content) => children(content, script),
            Self::Superscript(content) => children(content, Some(Script::Super)),
            Self::Subscript(content) => children(content, Some(Script::Sub)),
            Self::Link { label, .. } => children(label, script),
            Self::FootnoteRef { label } => footnote_ref_display(label),
            Self::Math { tex, .. } => tex.clone(),
            Self::FootnoteBackref { .. } => FOOTNOTE_BACKREF.into(),
            Self::SoftBreak => soft_break.into(),
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
    /// A plain `>` quote keeps its nested blocks (paragraphs, lists, code, nested quotes).
    Blockquote(Vec<DocumentBlock>),
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
        /// 1-based line ranges from fence meta such as `ts {1,3-5}` (Electron's `highlights`).
        highlights: LineHighlights,
    },
    MermaidCard {
        source: String,
    },
    /// A paragraph that holds only `$$...$$` display math.
    Math {
        tex: String,
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
            Self::Heading { content, .. } | Self::Paragraph(content) => {
                plain_text_for_spans(content)
            }
            Self::ListItem { children, .. }
            | Self::TaskItem { children, .. }
            | Self::Blockquote(children)
            | Self::Alert { children, .. } => plain_text_for_blocks(children),
            Self::FootnoteSection { notes } => notes
                .iter()
                .map(|(_, blocks)| plain_text_for_blocks(blocks))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
            Self::ThematicBreak => String::new(),
            Self::CodeBlock { code, .. } => code.clone(),
            Self::MermaidCard { source } | Self::Math { tex: source } => source.clone(),
            Self::Table(table) => table.plain_text(),
            Self::Image { alt, .. } | Self::RawText(alt) => alt.clone(),
        }
    }

    /// The text find searches. It stays byte-aligned with what the reader paints, but soft breaks
    /// (painted as line breaks) fold to spaces so a one-line query can match across them.
    pub fn find_text(&self) -> String {
        match self {
            Self::Heading { content, .. } | Self::Paragraph(content) => {
                find_text_for_spans(content)
            }
            Self::ListItem { children, .. }
            | Self::TaskItem { children, .. }
            | Self::Blockquote(children)
            | Self::Alert { children, .. } => find_text_for_blocks(children),
            Self::FootnoteSection { notes } => notes
                .iter()
                .map(|(_, blocks)| find_text_for_blocks(blocks))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
            Self::ThematicBreak => String::new(),
            Self::CodeBlock { code, .. } => code.clone(),
            Self::MermaidCard { source } | Self::Math { tex: source } => source.clone(),
            Self::Table(table) => table.find_text(),
            Self::Image { alt, .. } | Self::RawText(alt) => alt.clone(),
        }
    }
}

/// Highlighted code lines as inclusive 1-based ranges.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineHighlights(pub Vec<(usize, usize)>);

impl LineHighlights {
    pub fn contains(&self, line: usize) -> bool {
        self.0
            .iter()
            .any(|(start, end)| (*start..=*end).contains(&line))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A fenced code block's info string, split the way comark's `parseCodeblockInfo` does:
/// `lang {1,3-5} [file.ts] meta` (highlights and filename in either order).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeInfo {
    pub language: Option<String>,
    pub highlights: LineHighlights,
    pub filename: Option<String>,
    pub meta: Option<String>,
}

/// JavaScript `Number.parseInt(value, 10)` on a trimmed string: leading digits only.
fn parse_leading_int(value: &str) -> Option<usize> {
    let value = value.trim();
    let digits = value
        .char_indices()
        .take_while(|(_, character)| character.is_ascii_digit())
        .last()
        .map(|(index, character)| &value[..index + character.len_utf8()])?;
    digits.parse().ok()
}

pub fn parse_code_info(info: &str) -> CodeInfo {
    let mut result = CodeInfo::default();
    let mut remaining = info.trim();
    let language_end = remaining
        .find(|character: char| character.is_whitespace() || matches!(character, '[' | '{'))
        .unwrap_or(remaining.len());
    if language_end > 0 {
        result.language = Some(remaining[..language_end].to_owned());
        remaining = remaining[language_end..].trim();
    }
    loop {
        if let Some(rest) = remaining.strip_prefix('{') {
            let Some(close) = rest.find('}').filter(|close| *close > 0) else {
                break;
            };
            let mut ranges = Vec::new();
            for part in rest[..close].split(',') {
                let part = part.trim();
                if part.contains('-') {
                    let mut bounds = part.split('-');
                    if let (Some(start), Some(end)) = (
                        bounds.next().and_then(parse_leading_int),
                        bounds.next().and_then(parse_leading_int),
                    ) && start <= end
                    {
                        ranges.push((start, end));
                    }
                } else if let Some(line) = parse_leading_int(part) {
                    ranges.push((line, line));
                }
            }
            if !ranges.is_empty() {
                result.highlights = LineHighlights(ranges);
            }
            remaining = rest[close + 1..].trim();
        } else if remaining.starts_with('[') {
            let mut depth = 0_i32;
            let mut close = None;
            for (index, character) in remaining.char_indices() {
                match character {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(index);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(close) = close else {
                break;
            };
            result.filename = Some(remaining[1..close].replace("\\\\", ""));
            remaining = remaining[close + 1..].trim();
        } else {
            break;
        }
    }
    if !remaining.is_empty() {
        result.meta = Some(remaining.to_owned());
    }
    result
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
    /// GFM column alignment from the delimiter row; missing columns are unaligned.
    pub alignments: Vec<Alignment>,
}

impl TableBlock {
    pub fn alignment(&self, column_index: usize) -> Alignment {
        self.alignments
            .get(column_index)
            .copied()
            .unwrap_or(Alignment::None)
    }

    fn plain_text(&self) -> String {
        self.text_rows(plain_text_for_spans)
    }

    fn find_text(&self) -> String {
        self.text_rows(find_text_for_spans)
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
    Superscript,
    Subscript,
    /// An element opened by inline HTML; closed by its matching end tag or by the enclosing
    /// block ending.
    Html {
        tag: String,
        kind: HtmlInline,
    },
    Flatten,
}

/// The safe inline HTML subset the reader renders; every other tag keeps only its text.
#[derive(Debug, Clone)]
enum HtmlInline {
    Strong,
    Emphasis,
    Strikethrough,
    Code,
    Kbd,
    Superscript,
    Subscript,
    Mark,
    Link(String),
    Transparent,
}

impl HtmlInline {
    fn for_tag(tag: &str, attrs: &[(String, String)]) -> Self {
        match tag {
            "strong" | "b" => Self::Strong,
            "em" | "i" | "cite" | "var" | "dfn" => Self::Emphasis,
            "s" | "del" | "strike" => Self::Strikethrough,
            "code" | "samp" | "tt" => Self::Code,
            "kbd" => Self::Kbd,
            "sup" => Self::Superscript,
            "sub" => Self::Subscript,
            "mark" => Self::Mark,
            "a" => attrs
                .iter()
                .find(|(name, value)| name == "href" && !value.trim().is_empty())
                .map_or(Self::Transparent, |(_, href)| {
                    Self::Link(href.trim().to_owned())
                }),
            _ => Self::Transparent,
        }
    }
}

/// One open `>` quote. Plain quotes and GFM alerts both keep their child blocks.
#[derive(Debug)]
struct QuoteFrame {
    kind: Option<AlertKind>,
    children: Vec<DocumentBlock>,
    /// Open list items when the quote started; the quote is the innermost container while no
    /// newer item is open.
    item_depth: usize,
    /// Open lists when the quote started; lists inside the quote indent from zero again.
    list_base: usize,
}

impl QuoteFrame {
    fn into_block(self) -> DocumentBlock {
        match self.kind {
            Some(kind) => DocumentBlock::Alert {
                kind,
                children: self.children,
            },
            None => DocumentBlock::Blockquote(self.children),
        }
    }
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
        self.children.extend(paragraph_blocks(content));
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
    info: CodeInfo,
    code: String,
}

#[derive(Debug)]
struct TableContext {
    table: TableBlock,
    in_header: bool,
    row: Vec<Vec<InlineSpan>>,
}

impl TableContext {
    fn new(alignments: Vec<Alignment>) -> Self {
        Self {
            table: TableBlock {
                headers: Vec::new(),
                rows: Vec::new(),
                alignments,
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
            | Options::ENABLE_GFM
            | Options::ENABLE_MATH,
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
    let markdown_body = escape_non_math_dollars(markdown_body, options);
    let document_parent = path.parent().unwrap_or_else(|| Path::new("")).to_owned();

    for event in Parser::new_ext(&markdown_body, options) {
        if code_block.is_some() {
            match event {
                Event::End(TagEnd::CodeBlock) => {
                    let code = code_block.take().expect("code block is present");
                    let block = if code
                        .info
                        .language
                        .as_deref()
                        .is_some_and(|language| language.eq_ignore_ascii_case("mermaid"))
                    {
                        DocumentBlock::MermaidCard { source: code.code }
                    } else {
                        DocumentBlock::CodeBlock {
                            language: code.info.language,
                            code: code.code,
                            highlights: code.info.highlights,
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
                    let html = html_block.take().expect("HTML block is present");
                    for block in crate::html::html_to_blocks(&html, &document_parent) {
                        push_block(block, &mut blocks, &mut item_stack, &mut blockquotes);
                    }
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
                close_dangling_html(&mut inline_stack);
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
                close_dangling_html(&mut inline_stack);
                if let Some(frame) = inline_stack.pop() {
                    let content = frame.into_spans();
                    let level = level as u8;
                    push_block(
                        DocumentBlock::Heading { level, content },
                        &mut blocks,
                        &mut item_stack,
                        &mut blockquotes,
                    );
                }
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                // A quote opened inside a list item belongs to that item; park the item's
                // pending inline text first so source order holds.
                if let Some(item) = item_stack.last_mut() {
                    flush_item_inline_frame(&mut inline_stack, item);
                }
                blockquotes.push(QuoteFrame {
                    kind: kind.map(AlertKind::from),
                    children: Vec::new(),
                    item_depth: item_stack.len(),
                    list_base: list_stack.len(),
                })
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                if let Some(frame) = blockquotes.pop() {
                    push_block(
                        frame.into_block(),
                        &mut blocks,
                        &mut item_stack,
                        &mut blockquotes,
                    );
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Fenced(info) => parse_code_info(&info),
                    CodeBlockKind::Indented => CodeInfo::default(),
                };
                code_block = Some(CodeContext {
                    info,
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
                let list_base = blockquotes.last().map_or(0, |quote| quote.list_base);
                let depth = list_stack.len().saturating_sub(1 + list_base);
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
                close_dangling_html(&mut inline_stack);
                if let (Some(frame), Some(item)) = (inline_stack.pop(), item_stack.last_mut()) {
                    item.push_content(frame.into_spans());
                }
                if let Some(item) = item_stack.pop() {
                    let item_block = item.into_block();
                    if !quote_is_innermost(&item_stack, &blockquotes)
                        && let Some(parent) = item_stack.last_mut()
                    {
                        flush_item_inline_frame(&mut inline_stack, parent);
                    }
                    push_block(item_block, &mut blocks, &mut item_stack, &mut blockquotes);
                }
            }
            Event::Start(Tag::Table(alignments)) => table = Some(TableContext::new(alignments)),
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
                close_dangling_html(&mut inline_stack);
                if let (Some(frame), Some(table)) = (inline_stack.pop(), table.as_mut()) {
                    table.row.push(frame.into_spans());
                }
            }
            Event::Start(Tag::Emphasis) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Emphasis)
            }
            Event::Start(Tag::Strong) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Strong)
            }
            Event::Start(Tag::Strikethrough) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Strikethrough)
            }
            Event::Start(Tag::Superscript) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Superscript)
            }
            Event::Start(Tag::Subscript) => {
                push_inline_frame(&mut inline_stack, InlineContainer::Subscript)
            }
            Event::End(
                TagEnd::Emphasis
                | TagEnd::Strong
                | TagEnd::Strikethrough
                | TagEnd::Superscript
                | TagEnd::Subscript
                | TagEnd::Link,
            ) => {
                close_dangling_html(&mut inline_stack);
                pop_inline_frame(&mut inline_stack)
            }
            Event::Start(Tag::Link { dest_url, .. }) => push_inline_frame(
                &mut inline_stack,
                InlineContainer::Link(dest_url.into_string()),
            ),
            Event::Start(Tag::Image { dest_url, .. }) => push_inline_frame(
                &mut inline_stack,
                InlineContainer::Image(dest_url.into_string()),
            ),
            Event::End(TagEnd::Image) => {
                close_dangling_html(&mut inline_stack);
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
            Event::InlineHtml(html) => apply_inline_html(&html, &mut inline_stack),
            Event::Html(html) => {
                for block in crate::html::html_to_blocks(&html, &document_parent) {
                    push_block(block, &mut blocks, &mut item_stack, &mut blockquotes);
                }
            }
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
                    append_footnote_backref(&label, &mut main_blocks);
                    footnotes.push((label, main_blocks));
                }
            }
            Event::InlineMath(math) => push_inline_span(
                &mut inline_stack,
                InlineSpan::Math {
                    tex: math.into_string(),
                    display: false,
                },
            ),
            Event::DisplayMath(math) => push_inline_span(
                &mut inline_stack,
                InlineSpan::Math {
                    tex: math.into_string(),
                    display: true,
                },
            ),
            Event::Start(_) | Event::End(_) => {}
        }
    }

    if let Some((label, mut main_blocks)) = stashed_main_blocks.take() {
        std::mem::swap(&mut blocks, &mut main_blocks);
        append_footnote_backref(&label, &mut main_blocks);
        footnotes.push((label, main_blocks));
    }
    if !footnotes.is_empty() {
        blocks.push(DocumentBlock::FootnoteSection { notes: footnotes });
    }

    // Build navigation from rendered blocks, not parser events: plain blockquotes
    // flatten headings and must not shift the indices of later outline entries.
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

fn spans_reference_footnote(spans: &[InlineSpan], label: &str) -> bool {
    spans.iter().any(|span| match span {
        InlineSpan::FootnoteRef { label: found } => found == label,
        InlineSpan::Emphasis(content)
        | InlineSpan::Strong(content)
        | InlineSpan::Strikethrough(content)
        | InlineSpan::Superscript(content)
        | InlineSpan::Subscript(content)
        | InlineSpan::Mark(content)
        | InlineSpan::Kbd(content)
        | InlineSpan::Link { label: content, .. } => spans_reference_footnote(content, label),
        _ => false,
    })
}

fn block_references_footnote(block: &DocumentBlock, label: &str) -> bool {
    match block {
        DocumentBlock::Heading { content, .. } | DocumentBlock::Paragraph(content) => {
            spans_reference_footnote(content, label)
        }
        DocumentBlock::ListItem { children, .. }
        | DocumentBlock::TaskItem { children, .. }
        | DocumentBlock::Blockquote(children)
        | DocumentBlock::Alert { children, .. } => children
            .iter()
            .any(|child| block_references_footnote(child, label)),
        DocumentBlock::Table(table) => std::iter::once(&table.headers)
            .chain(table.rows.iter())
            .flatten()
            .any(|cell| spans_reference_footnote(cell, label)),
        _ => false,
    }
}

fn heading_count(block: &DocumentBlock) -> usize {
    match block {
        DocumentBlock::Heading { .. } => 1,
        DocumentBlock::ListItem { children, .. }
        | DocumentBlock::TaskItem { children, .. }
        | DocumentBlock::Alert { children, .. } => children.iter().map(heading_count).sum(),
        _ => 0,
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

/// Whether an inline `$...$` span pulldown-cmark accepted is not math under the Electron reader's
/// rules: it must close on the same line and must not start with a digit (`$5 or $10` is prices).
fn is_rejected_inline_math(tex: &str) -> bool {
    tex.starts_with(|character: char| character.is_ascii_digit()) || tex.contains('\n')
}

/// Escapes the opening `$` of every inline math span the Electron reader would not treat as math,
/// so the real parse reads those dollars as literal text instead of swallowing the rest of the
/// paragraph (including code spans) into a bogus formula. Escaping one opener can expose another
/// candidate later in the paragraph, so this repeats until the parse is stable.
fn escape_non_math_dollars(markdown: &str, options: Options) -> Cow<'_, str> {
    let mut markdown = Cow::Borrowed(markdown);
    if !markdown.contains('$') {
        return markdown;
    }
    for _ in 0..64 {
        let openers = Parser::new_ext(&markdown, options)
            .into_offset_iter()
            .filter_map(|(event, range)| match event {
                Event::InlineMath(tex) if is_rejected_inline_math(&tex) => Some(range.start),
                _ => None,
            })
            .filter(|&start| markdown.as_bytes().get(start) == Some(&b'$'))
            .collect::<Vec<_>>();
        if openers.is_empty() {
            break;
        }
        let mut escaped = String::with_capacity(markdown.len() + openers.len());
        let mut copied = 0;
        for start in openers {
            escaped.push_str(&markdown[copied..start]);
            escaped.push('\\');
            copied = start;
        }
        escaped.push_str(&markdown[copied..]);
        markdown = Cow::Owned(escaped);
    }
    markdown
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
    if !quote_is_innermost(item_stack, blockquotes)
        && let Some(item) = item_stack.last_mut()
    {
        item.push_content(content);
    } else {
        for block in paragraph_blocks(content) {
            push_block(block, blocks, item_stack, blockquotes);
        }
    }
}

/// A paragraph made only of `$$...$$` spans becomes standalone display math blocks, matching
/// how KaTeX lays out display math on its own line; anything else stays one paragraph.
fn paragraph_blocks(content: Vec<InlineSpan>) -> Vec<DocumentBlock> {
    let only_display_math = content
        .iter()
        .any(|span| matches!(span, InlineSpan::Math { display: true, .. }))
        && content.iter().all(|span| match span {
            InlineSpan::Math { display, .. } => *display,
            InlineSpan::Text(text) => text.trim().is_empty(),
            InlineSpan::SoftBreak | InlineSpan::HardBreak => true,
            _ => false,
        });
    if only_display_math {
        content
            .into_iter()
            .filter_map(|span| match span {
                InlineSpan::Math { tex, .. } => Some(DocumentBlock::Math { tex }),
                _ => None,
            })
            .collect()
    } else if content.is_empty() {
        Vec::new()
    } else {
        vec![DocumentBlock::Paragraph(content)]
    }
}

/// Whether the newest open quote is nested inside every open list item (so blocks go to it).
fn quote_is_innermost(item_stack: &[ItemContext], blockquotes: &[QuoteFrame]) -> bool {
    blockquotes
        .last()
        .is_some_and(|quote| quote.item_depth == item_stack.len())
}

fn push_block(
    block: DocumentBlock,
    blocks: &mut Vec<DocumentBlock>,
    item_stack: &mut [ItemContext],
    blockquotes: &mut [QuoteFrame],
) {
    if quote_is_innermost(item_stack, blockquotes) {
        blockquotes
            .last_mut()
            .expect("an innermost quote is open")
            .children
            .push(block);
    } else if let Some(item) = item_stack.last_mut() {
        item.push_block(block);
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

/// Ends the footnote body with a `↩` backlink, inline in its last paragraph when it has one.
fn append_footnote_backref(label: &str, note: &mut Vec<DocumentBlock>) {
    let backref = InlineSpan::FootnoteBackref {
        label: label.to_owned(),
    };
    if let Some(DocumentBlock::Paragraph(content)) = note.last_mut() {
        content.push(InlineSpan::Text(" ".into()));
        content.push(backref);
    } else {
        note.push(DocumentBlock::Paragraph(vec![backref]));
    }
}

/// Closes inline HTML elements still open when their enclosing Markdown container ends.
fn close_dangling_html(stack: &mut Vec<InlineFrame>) {
    while matches!(
        stack.last().map(|frame| &frame.container),
        Some(InlineContainer::Html { .. })
    ) {
        pop_inline_frame(stack);
    }
}

/// Applies one inline HTML token (`<kbd>`, `</kbd>`, `<br>`, a comment, ...) to the inline stack.
/// Supported tags open styled frames, unknown tags keep only their text, comments vanish.
fn apply_inline_html(html: &str, stack: &mut Vec<InlineFrame>) {
    let token = html.trim();
    if token.starts_with("<!") || token.starts_with("<?") {
        return;
    }
    if let Some(close) = token.strip_prefix("</") {
        let tag = close
            .trim_end_matches('>')
            .trim()
            .trim_end_matches('/')
            .to_ascii_lowercase();
        let open_html = stack
            .iter()
            .rev()
            .take_while(|frame| matches!(frame.container, InlineContainer::Html { .. }))
            .position(|frame| matches!(&frame.container, InlineContainer::Html { tag: open, .. } if *open == tag));
        if let Some(depth) = open_html {
            for _ in 0..=depth {
                pop_inline_frame(stack);
            }
        }
        return;
    }
    let Some((tag, attrs, self_closing, _)) = crate::html::scan_open_tag(token) else {
        push_inline_span(stack, InlineSpan::Text(html.to_owned()));
        return;
    };
    match tag.as_str() {
        "br" => push_inline_span(stack, InlineSpan::HardBreak),
        "img" => {
            let attr = |name: &str| {
                attrs
                    .iter()
                    .find(|(attr_name, _)| attr_name == name)
                    .map(|(_, value)| value.clone())
                    .unwrap_or_default()
            };
            push_inline_image(
                stack,
                ImageData {
                    alt: attr("alt"),
                    source: attr("src").trim().to_owned(),
                },
            );
        }
        _ if self_closing || crate::html::is_void_element(&tag) => {}
        _ => {
            let kind = HtmlInline::for_tag(&tag, &attrs);
            push_inline_frame(stack, InlineContainer::Html { tag, kind });
        }
    }
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
        InlineContainer::Superscript => {
            push_inline_span(stack, InlineSpan::Superscript(frame.into_spans()))
        }
        InlineContainer::Subscript => {
            push_inline_span(stack, InlineSpan::Subscript(frame.into_spans()))
        }
        InlineContainer::Image(_) => unreachable!("images are ended by pop_image_frame"),
        InlineContainer::Html { kind, .. } => {
            let spans = frame.into_spans();
            let span = match kind {
                HtmlInline::Strong => InlineSpan::Strong(spans),
                HtmlInline::Emphasis => InlineSpan::Emphasis(spans),
                HtmlInline::Strikethrough => InlineSpan::Strikethrough(spans),
                HtmlInline::Code => InlineSpan::Code(plain_text_for_spans(&spans)),
                HtmlInline::Kbd => InlineSpan::Kbd(spans),
                HtmlInline::Superscript => InlineSpan::Superscript(spans),
                HtmlInline::Subscript => InlineSpan::Subscript(spans),
                HtmlInline::Mark => InlineSpan::Mark(spans),
                HtmlInline::Link(target) => InlineSpan::Link {
                    label: spans,
                    target,
                },
                HtmlInline::Transparent => {
                    if let Some(parent) = stack.last_mut() {
                        parent.flush_pending_image();
                        parent.spans.extend(spans);
                    }
                    return;
                }
            };
            push_inline_span(stack, span);
        }
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
    if stack.is_empty() {
        Some(DocumentBlock::Image {
            alt: image.alt,
            source: image.source,
        })
    } else {
        push_inline_image(stack, image);
        None
    }
}

/// An image alone in its paragraph becomes an image block; one among text keeps its alt text.
fn push_inline_image(stack: &mut [InlineFrame], image: ImageData) {
    let Some(parent) = stack.last_mut() else {
        return;
    };
    if parent.spans.is_empty()
        && parent.pending_image.is_none()
        && matches!(parent.container, InlineContainer::Flatten)
    {
        parent.pending_image = Some(image);
    } else {
        parent.flush_pending_image();
        if !image.alt.is_empty() {
            parent.spans.push(InlineSpan::Text(image.alt));
        }
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

fn find_text_for_spans(spans: &[InlineSpan]) -> String {
    spans.iter().map(InlineSpan::find_text).collect()
}

fn plain_text_for_blocks(blocks: &[DocumentBlock]) -> String {
    blocks
        .iter()
        .map(DocumentBlock::plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_text_for_blocks(blocks: &[DocumentBlock]) -> String {
    blocks
        .iter()
        .map(DocumentBlock::find_text)
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
    fn heading_navigation_handles_nested_duplicate_and_encoded_headings() {
        let document = parse_document(
            PathBuf::from("guide.md"),
            "# Overview\n\n- ## Nested\n\n## Details\n\n## Details\n\n## Café & setup\n".into(),
        );
        assert_eq!(document.heading_block(0), Some(0));
        assert_eq!(document.heading_block(1), Some(1));
        assert_eq!(document.heading_block(2), Some(2));
        assert_eq!(document.heading_block(3), Some(3));
        assert_eq!(document.heading_block(99), None);
        assert_eq!(document.anchor_block("details-1"), Some(3));
        assert_eq!(document.anchor_block("caf%C3%A9--setup"), Some(4));
        assert_eq!(document.anchor_block("missing"), None);
        assert_eq!(document.anchor_block("%ZZ"), None);
        assert_eq!(document.anchor_block(""), Some(0));
        let quoted = parse_document(
            PathBuf::from("quote.md"),
            "> # Quoted title\n\n## Actual section\n".into(),
        );
        assert_eq!(quoted.headings.len(), 1);
        assert_eq!(quoted.anchor_block("actual-section"), Some(1));
    }

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
                highlights: LineHighlights::default(),
            }
        );
    }

    #[test]
    fn raw_html_blocks_and_mdx_keep_only_their_text() {
        let parsed = parse_document(
            PathBuf::from("/tmp/component.mdx"),
            "<aside>Note</aside>\n\n<Component value={1} />\n\n<script>alert(1)</script>\n\n<!-- hidden -->\n\nAfter"
                .into(),
        );
        assert_eq!(parsed.plain_text(), "Note\nAfter");
    }

    #[test]
    fn inline_html_subset_renders_as_styled_spans() {
        let parsed = parse_document(
            PathBuf::from("/tmp/inline.md"),
            "Press <kbd>Ctrl</kbd>+<kbd>C</kbd>, H<sub>2</sub>O, x<sup>2</sup>, <mark>hot</mark>, \
             <b>bold</b> <i>it</i> <em>em</em> <strong>st</strong> <code>a&lt;b</code> \
             <a href=\"guide.md\">go</a><br><span class=\"x\">plain</span> <blink>t</blink><!-- c -->"
                .into(),
        );
        let DocumentBlock::Paragraph(spans) = &parsed.blocks[0] else {
            panic!("paragraph");
        };
        let text = |value: &str| InlineSpan::Text(value.into());
        assert_eq!(
            spans,
            &vec![
                text("Press "),
                InlineSpan::Kbd(vec![text("Ctrl")]),
                text("+"),
                InlineSpan::Kbd(vec![text("C")]),
                text(", H"),
                InlineSpan::Subscript(vec![text("2")]),
                text("O, x"),
                InlineSpan::Superscript(vec![text("2")]),
                text(", "),
                InlineSpan::Mark(vec![text("hot")]),
                text(", "),
                InlineSpan::Strong(vec![text("bold")]),
                text(" "),
                InlineSpan::Emphasis(vec![text("it")]),
                text(" "),
                InlineSpan::Emphasis(vec![text("em")]),
                text(" "),
                InlineSpan::Strong(vec![text("st")]),
                text(" "),
                InlineSpan::Code("a<b".into()),
                text(" "),
                InlineSpan::Link {
                    label: vec![text("go")],
                    target: "guide.md".into(),
                },
                InlineSpan::HardBreak,
                text("plain"),
                text(" "),
                text("t"),
            ]
        );
        assert_eq!(
            parsed.plain_text(),
            "Press Ctrl+C, H₂O, x², hot, bold it em st a<b go\nplain t"
        );
    }

    #[test]
    fn unclosed_inline_html_closes_with_its_paragraph() {
        let parsed = parse_document(
            PathBuf::from("/tmp/unclosed.md"),
            "a <b>bold\n\nnext".into(),
        );
        assert_eq!(
            parsed.blocks,
            vec![
                DocumentBlock::Paragraph(vec![
                    InlineSpan::Text("a ".into()),
                    InlineSpan::Strong(vec![InlineSpan::Text("bold".into())]),
                ]),
                DocumentBlock::Paragraph(vec![InlineSpan::Text("next".into())]),
            ]
        );
    }

    #[test]
    fn html_block_images_and_details_render_through_the_html_subset() {
        let parsed = parse_document(
            PathBuf::from("/vault/readme.md"),
            "<p align=\"center\"><img src=\"logo.png\" alt=\"Logo\"></p>\n\n<details>\n<summary>More</summary>\n\nHidden body\n\n</details>\n"
                .into(),
        );
        assert_eq!(
            parsed.blocks,
            vec![
                DocumentBlock::Image {
                    alt: "Logo".into(),
                    source: "/vault/logo.png".into(),
                },
                DocumentBlock::Paragraph(vec![InlineSpan::Strong(vec![InlineSpan::Text(
                    "More".into()
                )])]),
                DocumentBlock::Paragraph(vec![InlineSpan::Text("Hidden body".into())]),
            ]
        );
    }

    #[test]
    fn a_lone_inline_html_image_becomes_an_image_block() {
        let parsed = parse_document(
            PathBuf::from("/tmp/image.md"),
            "Intro\n\n<img src=\"a.png\" alt=\"A\"> \n\nText <img src=\"b.png\" alt=\"B\"> more"
                .into(),
        );
        assert_eq!(
            parsed.blocks[1],
            DocumentBlock::Image {
                alt: "A".into(),
                source: "/tmp/a.png".into(),
            }
        );
        assert_eq!(parsed.blocks[2].plain_text(), "Text B more");
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
            DocumentBlock::Blockquote(vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                "plain quote".into()
            )])]),
        );
    }

    #[test]
    fn footnote_refs_and_backrefs_resolve_to_their_jump_targets() {
        let parsed = parse_document(
            PathBuf::from("/tmp/notes.md"),
            "# Title\n\nIntro.\n\n- A list claim.[^src]\n\nTail.\n\n[^src]: Source.\n\n    ```\n    code\n    ```\n".into(),
        );
        let section = parsed.blocks.len() - 1;
        assert!(matches!(
            parsed.blocks[section],
            DocumentBlock::FootnoteSection { .. }
        ));
        assert_eq!(parsed.anchor_block("fn-src"), Some(section));
        assert_eq!(parsed.anchor_block("fnref-src"), Some(2));
        assert_eq!(parsed.anchor_block("fn-missing"), None);
        assert_eq!(
            footnote_target("src"),
            "#fn-src",
            "reader links refs to this target"
        );
        assert_eq!(footnote_backref_target("src"), "#fnref-src");

        // A note that does not end in a paragraph gets its own backlink paragraph.
        let DocumentBlock::FootnoteSection { notes } = &parsed.blocks[section] else {
            unreachable!();
        };
        assert_eq!(
            notes[0].1.last(),
            Some(&DocumentBlock::Paragraph(vec![
                InlineSpan::FootnoteBackref {
                    label: "src".into()
                }
            ]))
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
                    vec![DocumentBlock::Paragraph(vec![
                        InlineSpan::Text("The evidence.".into()),
                        InlineSpan::Text(" ".into()),
                        InlineSpan::FootnoteBackref { label: "1".into() },
                    ])],
                )],
            }
        );
        assert_eq!(parsed.blocks.len(), 3, "footnote body must not leak");
    }

    #[test]
    fn dollar_math_parses_to_math_spans_without_eating_prices() {
        let parsed = parse_document(
            PathBuf::from("/tmp/math.md"),
            "Energy $E = mc^2$ costs $5 or $10.\nAn escaped \\$ stays, and `$code$` too.\n".into(),
        );

        let DocumentBlock::Paragraph(spans) = &parsed.blocks[0] else {
            panic!("expected a paragraph, got {:?}", parsed.blocks[0]);
        };
        assert!(spans.contains(&InlineSpan::Math {
            tex: "E = mc^2".into(),
            display: false,
        }));
        assert!(
            spans.contains(&InlineSpan::Code("$code$".into())),
            "{spans:?}"
        );
        let math_count = spans
            .iter()
            .filter(|span| matches!(span, InlineSpan::Math { .. }))
            .count();
        assert_eq!(math_count, 1, "prices must stay text: {spans:?}");
        assert!(parsed.blocks[0].plain_text().contains("costs $5 or $10."));
        assert!(parsed.blocks[0].plain_text().contains("escaped $ stays"));
    }

    #[test]
    fn display_math_paragraphs_become_math_blocks_and_inline_display_math_stays_inline() {
        let parsed = parse_document(
            PathBuf::from("/tmp/math.md"),
            "$$\n\\int_0^1 x\\,dx\n$$\n\nInline $$a^2$$ display.\n\n- item\n\n  $$b^2$$\n".into(),
        );

        assert_eq!(
            parsed.blocks[0],
            DocumentBlock::Math {
                tex: "\n\\int_0^1 x\\,dx\n".into(),
            }
        );
        assert_eq!(
            parsed.blocks[1],
            DocumentBlock::Paragraph(vec![
                InlineSpan::Text("Inline ".into()),
                InlineSpan::Math {
                    tex: "a^2".into(),
                    display: true,
                },
                InlineSpan::Text(" display.".into()),
            ])
        );
        let DocumentBlock::ListItem { children, .. } = &parsed.blocks[2] else {
            panic!("expected a list item, got {:?}", parsed.blocks[2]);
        };
        assert!(children.contains(&DocumentBlock::Math { tex: "b^2".into() }));
        assert_eq!(parsed.blocks[0].find_text(), "\n\\int_0^1 x\\,dx\n");
    }

    #[test]
    fn diagrams_fixture_parses_every_diagram_and_display_equation() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/diagrams.md");
        let parsed = parse_document(fixture.clone(), fs::read_to_string(&fixture).unwrap());

        let mermaid = parsed
            .blocks
            .iter()
            .filter(|block| matches!(block, DocumentBlock::MermaidCard { .. }))
            .count();
        let display_math = parsed
            .blocks
            .iter()
            .filter(|block| matches!(block, DocumentBlock::Math { .. }))
            .count();
        let inline_math = parsed
            .blocks
            .iter()
            .filter_map(|block| match block {
                DocumentBlock::Paragraph(spans) => Some(spans),
                _ => None,
            })
            .flat_map(|spans| spans.iter())
            .filter(|span| matches!(span, InlineSpan::Math { .. }))
            .count();

        let prices = parsed
            .blocks
            .iter()
            .find(|block| block.plain_text().contains("the book costs"))
            .expect("prices paragraph");
        let DocumentBlock::Paragraph(price_spans) = prices else {
            panic!("expected a paragraph, got {prices:?}");
        };
        assert!(
            prices
                .plain_text()
                .contains("costs $5 and the course costs $10, and an escaped $ sign"),
            "{price_spans:?}"
        );
        assert!(price_spans.contains(&InlineSpan::Math {
            tex: "a^2 + b^2 = c^2".into(),
            display: true,
        }));

        assert_eq!(mermaid, 13);
        assert_eq!(display_math, 5);
        assert!(inline_math >= 8, "found {inline_math} inline math spans");
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
    fn find_text_folds_soft_breaks_to_spaces() {
        let span_level = InlineSpan::SoftBreak;
        assert_eq!(span_level.plain_text(), "\n");
        assert_eq!(span_level.find_text(), " ");

        let block = DocumentBlock::Paragraph(vec![
            InlineSpan::Text("one".into()),
            InlineSpan::SoftBreak,
            InlineSpan::Text("two".into()),
            InlineSpan::HardBreak,
            InlineSpan::Text("three".into()),
        ]);
        assert_eq!(block.find_text(), "one two\nthree");
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
                alignments: vec![Alignment::None, Alignment::None],
            })]
        );
    }

    #[test]
    fn preserves_gfm_table_column_alignment() {
        let parsed = parse_document(
            PathBuf::from("/tmp/aligned.md"),
            "| L | C | R | N |\n| :-- | :-: | --: | --- |\n| a | b | c | d |\n".into(),
        );

        let DocumentBlock::Table(table) = &parsed.blocks[0] else {
            panic!("expected a table");
        };
        assert_eq!(
            table.alignments,
            vec![
                Alignment::Left,
                Alignment::Center,
                Alignment::Right,
                Alignment::None
            ]
        );
        assert_eq!(table.alignment(2), Alignment::Right);
        assert_eq!(table.alignment(9), Alignment::None);
    }

    #[test]
    fn keeps_blockquote_nested_blocks_inert_and_in_source_order() {
        let parsed = parse_document(
            PathBuf::from("/tmp/quote.md"),
            "> Intro\n>\n> - Item\n>\n> ```rust\n> let n = 1;\n> ```\n>\n> <aside>Raw</aside>\n>\n> > Nested\n\nAfter\n"
                .into(),
        );

        assert!(matches!(parsed.blocks[1], DocumentBlock::Paragraph(_)));
        assert_eq!(parsed.blocks.len(), 2);
        let DocumentBlock::Blockquote(children) = &parsed.blocks[0] else {
            panic!("expected a structured blockquote");
        };
        assert_eq!(
            children,
            &vec![
                DocumentBlock::Paragraph(vec![InlineSpan::Text("Intro".into())]),
                DocumentBlock::ListItem {
                    kind: ListKind::Unordered,
                    depth: 0,
                    children: vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                        "Item".into()
                    )])],
                },
                DocumentBlock::CodeBlock {
                    language: Some("rust".into()),
                    code: "let n = 1;\n".into(),
                    highlights: LineHighlights::default(),
                },
                DocumentBlock::Paragraph(vec![InlineSpan::Text("Raw".into())]),
                DocumentBlock::Blockquote(vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                    "Nested".into()
                )])]),
            ]
        );
    }

    #[test]
    fn blockquotes_keep_headings_and_quotes_inside_list_items_stay_in_the_item() {
        let parsed = parse_document(
            PathBuf::from("/tmp/quote.md"),
            "> ## Quoted heading\n> body\n\n- item\n\n  > inner quote\n\n  after\n".into(),
        );
        assert_eq!(
            parsed.blocks[0],
            DocumentBlock::Blockquote(vec![
                DocumentBlock::Heading {
                    level: 2,
                    content: vec![InlineSpan::Text("Quoted heading".into())],
                },
                DocumentBlock::Paragraph(vec![InlineSpan::Text("body".into())]),
            ])
        );
        assert!(
            parsed.headings.is_empty(),
            "quoted headings stay out of the outline"
        );
        assert_eq!(
            parsed.blocks[1],
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: vec![
                    DocumentBlock::Paragraph(vec![InlineSpan::Text("item".into())]),
                    DocumentBlock::Blockquote(vec![DocumentBlock::Paragraph(vec![
                        InlineSpan::Text("inner quote".into())
                    ])]),
                    DocumentBlock::Paragraph(vec![InlineSpan::Text("after".into())]),
                ],
            }
        );
    }

    #[test]
    fn code_fence_meta_parses_like_comark() {
        let info = parse_code_info("ts {1,3-5} [app.ts] title=x");
        assert_eq!(info.language.as_deref(), Some("ts"));
        assert_eq!(info.highlights, LineHighlights(vec![(1, 1), (3, 5)]));
        assert_eq!(info.filename.as_deref(), Some("app.ts"));
        assert_eq!(info.meta.as_deref(), Some("title=x"));

        let tight = parse_code_info("typescript[file]{2}meta");
        assert_eq!(tight.language.as_deref(), Some("typescript"));
        assert_eq!(tight.highlights, LineHighlights(vec![(2, 2)]));
        assert_eq!(tight.filename.as_deref(), Some("file"));
        assert_eq!(tight.meta.as_deref(), Some("meta"));

        // parseInt leniency, bad parts dropped, reversed ranges empty, `{}` stops parsing.
        let lenient = parse_code_info("rs {2a, x, 5-3, 7 - 8}");
        assert_eq!(lenient.highlights, LineHighlights(vec![(2, 2), (7, 8)]));
        assert!(lenient.highlights.contains(8));
        assert!(!lenient.highlights.contains(3));
        assert_eq!(
            parse_code_info("js {} rest").meta.as_deref(),
            Some("{} rest")
        );
        assert_eq!(parse_code_info("").language, None);

        let parsed = parse_document(
            PathBuf::from("/tmp/code.md"),
            "```rust {2}\nlet a = 1;\nlet b = 2;\n```\n".into(),
        );
        assert_eq!(
            parsed.blocks[0],
            DocumentBlock::CodeBlock {
                language: Some("rust".into()),
                code: "let a = 1;\nlet b = 2;\n".into(),
                highlights: LineHighlights(vec![(2, 2)]),
            }
        );
    }

    #[test]
    fn script_text_uses_unicode_forms_only_when_every_character_has_one() {
        assert_eq!(script_text("2", Script::Super), "²");
        assert_eq!(script_text("n+1", Script::Super), "ⁿ⁺¹");
        assert_eq!(script_text("2", Script::Sub), "₂");
        assert_eq!(script_text("max", Script::Sub), "ₘₐₓ");
        assert_eq!(script_text("th", Script::Super), "th");
        assert!(!script_is_unicode("th", Script::Super));
        assert_eq!(script_text(" ", Script::Super), " ");

        let block = DocumentBlock::Paragraph(vec![
            InlineSpan::Text("x".into()),
            InlineSpan::Superscript(vec![InlineSpan::Text("2".into())]),
        ]);
        assert_eq!(block.find_text(), "x²");
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
                        highlights: LineHighlights::default(),
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
