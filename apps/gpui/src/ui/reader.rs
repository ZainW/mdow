use crate::{
    app::MdowApp,
    document::{
        AlertKind, Alignment, DocumentBlock, FOOTNOTE_BACKREF, InlineSpan, LineHighlights,
        ListKind, ParsedDocument, Script, TableBlock, footnote_backref_target,
        footnote_ref_display, footnote_target, is_supported_document, resolve_local_target,
        script_text,
    },
    graphics::{
        DiagramPalette, GraphicCache, GraphicKey, GraphicState, MATH_SCALE, math_width_em,
        render_mermaid,
    },
    prefs::{READER_FONT_SIZE, READER_LINE_HEIGHT, ReaderStyle},
    syntax::{HighlightCache, HighlightLookup, HighlightedCode, PreparedDocument},
    theme::{ColorScheme, Metrics, Theme},
    ui::{
        graphic::{
            GraphicElement, InlineMathSlot, InlineMathText, MATH_PLACEHOLDER, hex_color, math_state,
        },
        primitives::icon,
    },
};
use gpui::{
    AnyElement, Context, FocusHandle, Font, FontFeatures, FontStyle, FontWeight, Img,
    InteractiveElement, InteractiveText, IntoElement, ListAlignment, ListOffset, ListState,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render,
    StatefulInteractiveElement, StrikethroughStyle, Styled, StyledImage, StyledText, TextRun,
    UnderlineStyle, WeakEntity, Window, canvas, div, font, img, list, point, prelude::*, px,
    relative,
};
use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

pub const CODE_COPY_FEEDBACK_DURATION: Duration = Duration::from_secs(2);
pub const CODE_HEADER_HEIGHT: f32 = 32.0;
pub const CODE_COPY_BUTTON_HEIGHT: f32 = 24.0;
pub const CODE_PADDING_BOTTOM: f32 = 14.0;

const READER_SCROLLBAR_TRACK_INSET: f32 = 4.0;
const READER_SCROLLBAR_MIN_THUMB_HEIGHT: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct ReaderScrollbarGeometry {
    thumb_top: f32,
    thumb_height: f32,
    thumb_travel: f32,
    max_offset: f32,
}

fn reader_scrollbar_geometry(
    viewport_height: f32,
    max_offset: f32,
    current_offset: f32,
) -> Option<ReaderScrollbarGeometry> {
    if !viewport_height.is_finite()
        || !max_offset.is_finite()
        || !current_offset.is_finite()
        || viewport_height <= 0.0
        || max_offset <= 0.0
    {
        return None;
    }

    let track_height = (viewport_height - READER_SCROLLBAR_TRACK_INSET * 2.0).max(0.0);
    if track_height <= 0.0 {
        return None;
    }

    let content_height = viewport_height + max_offset;
    let thumb_height = (track_height * viewport_height / content_height)
        .max(READER_SCROLLBAR_MIN_THUMB_HEIGHT.min(track_height))
        .min(track_height);
    let thumb_travel = (track_height - thumb_height).max(0.0);
    let progress = (-current_offset / max_offset).clamp(0.0, 1.0);

    Some(ReaderScrollbarGeometry {
        thumb_top: READER_SCROLLBAR_TRACK_INSET + thumb_travel * progress,
        thumb_height,
        thumb_travel,
        max_offset,
    })
}

fn reader_scrollbar_offset_for_pointer(
    pointer_y: f32,
    grab_y: f32,
    geometry: ReaderScrollbarGeometry,
) -> f32 {
    if geometry.thumb_travel <= 0.0 {
        return 0.0;
    }

    let thumb_top =
        (pointer_y - grab_y - READER_SCROLLBAR_TRACK_INSET).clamp(0.0, geometry.thumb_travel);
    -geometry.max_offset * thumb_top / geometry.thumb_travel
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockStyle {
    pub font_size: f32,
    pub font_weight: u16,
    pub line_height: f32,
    pub letter_spacing_em: f32,
    pub margin_top_em: f32,
    pub margin_bottom_em: f32,
    pub muted: bool,
    pub uppercase: bool,
    pub radius: f32,
    pub padding: [f32; 2],
}

impl BlockStyle {
    pub fn for_block(block: &DocumentBlock) -> Self {
        match block {
            DocumentBlock::Heading { level, .. } => Self::heading(*level),
            DocumentBlock::CodeBlock { .. } | DocumentBlock::MermaidCard { .. } => {
                Self::code_block()
            }
            DocumentBlock::Table(_) => Self::table_cell(),
            _ => Self::body(),
        }
    }

    /// Electron's markdown.css heading scale (em of the 15.5px body).
    pub fn heading(level: u8) -> Self {
        let (scale, font_weight, line_height, letter_spacing_em, margin_top_em, margin_bottom_em) =
            match level {
                1 => (1.875, 700, 1.2, -0.025, 2.0, 0.6),
                2 => (1.5, 650, 1.25, -0.02, 1.8, 0.5),
                3 => (1.15, 600, 1.3, -0.01, 1.5, 0.4),
                4 => (1.0, 600, 1.4, 0.0, 1.3, 0.3),
                5 => (0.95, 600, 1.4, 0.0, 1.2, 0.25),
                _ => (0.875, 600, 1.4, 0.03, 1.0, 0.2),
            };
        // Only h6 is muted (and uppercase), as in Electron since v1.10.
        let muted = level >= 6;
        let uppercase = level >= 6;
        Self {
            font_size: READER_FONT_SIZE * scale,
            font_weight,
            line_height,
            letter_spacing_em,
            margin_top_em,
            margin_bottom_em,
            muted,
            uppercase,
            ..Self::body()
        }
    }

    /// The redesigned code surface: 8px radius; the code sits below a 32px header with
    /// 12px 16px 14px padding (top/x here, bottom is [`CODE_PADDING_BOTTOM`]).
    pub fn code_block() -> Self {
        Self {
            radius: 8.0,
            padding: [12.0, 16.0],
            line_height: 1.6,
            ..Self::body()
        }
    }

    pub fn table_cell() -> Self {
        Self {
            padding: [10.0, 14.0],
            ..Self::body()
        }
    }

    /// `padding: 0.4em 1em`.
    pub fn blockquote() -> Self {
        Self {
            padding: [READER_FONT_SIZE * 0.4, READER_FONT_SIZE],
            ..Self::body()
        }
    }

    fn body() -> Self {
        Self {
            font_size: READER_FONT_SIZE,
            font_weight: 400,
            line_height: READER_LINE_HEIGHT,
            letter_spacing_em: 0.0,
            margin_top_em: 0.0,
            margin_bottom_em: 1.0,
            muted: false,
            uppercase: false,
            radius: 0.0,
            padding: [0.0, 0.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineLayout {
    pub text: String,
    pub styles: Vec<InlineStyleRange>,
    pub links: Vec<InlineLink>,
    /// Formulas, in text order. Each range first holds the `$...$` source (styled as code, the
    /// fallback when the formula cannot be typeset) until [`reserve_inline_math`] swaps it for
    /// placeholder characters as wide as the typeset formula.
    pub math: Vec<InlineMath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineMath {
    pub range: Range<usize>,
    pub tex: String,
    pub display: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineStyleRange {
    pub range: Range<usize>,
    pub emphasis: bool,
    pub strong: bool,
    pub code: bool,
    pub strikethrough: bool,
    pub footnote: bool,
    pub script: Option<Script>,
    pub mark: bool,
    pub kbd: bool,
    pub link_target: Option<String>,
    pub link_node_id: Option<usize>,
    /// Placeholder characters reserving room for a typeset formula; painted transparent.
    pub math: bool,
}

impl InlineStyleRange {
    fn new(range: Range<usize>, style: InlineStyleContext<'_>) -> Self {
        Self {
            range,
            emphasis: style.emphasis,
            strong: style.strong,
            code: style.code,
            strikethrough: style.strikethrough,
            footnote: style.footnote,
            script: style.script,
            mark: style.mark,
            kbd: style.kbd,
            link_target: style.link_target.map(str::to_owned),
            link_node_id: style.link_node_id,
            math: false,
        }
    }
}

#[cfg(test)]
impl InlineStyleRange {
    fn emphasis(range: Range<usize>) -> Self {
        Self {
            range,
            emphasis: true,
            ..Self::default()
        }
    }

    fn emphasis_strong(range: Range<usize>) -> Self {
        Self {
            range,
            emphasis: true,
            strong: true,
            ..Self::default()
        }
    }

    fn code(range: Range<usize>) -> Self {
        Self {
            range,
            code: true,
            ..Self::default()
        }
    }

    fn link(range: Range<usize>, target: &str) -> Self {
        Self {
            range,
            link_target: Some(target.to_owned()),
            link_node_id: Some(0),
            ..Self::default()
        }
    }

    fn strikethrough(range: Range<usize>) -> Self {
        Self {
            range,
            strikethrough: true,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineLink {
    pub range: Range<usize>,
    pub target: String,
    pub node_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkSurfaceKey {
    Block {
        block_index: usize,
    },
    TableHeader {
        block_index: usize,
        column_index: usize,
    },
    TableCell {
        block_index: usize,
        row_index: usize,
        column_index: usize,
    },
}

impl LinkSurfaceKey {
    pub const fn block(block_index: usize) -> Self {
        Self::Block { block_index }
    }

    const fn table_header(block_index: usize, column_index: usize) -> Self {
        Self::TableHeader {
            block_index,
            column_index,
        }
    }

    const fn table_cell(block_index: usize, row_index: usize, column_index: usize) -> Self {
        Self::TableCell {
            block_index,
            row_index,
            column_index,
        }
    }

    fn debug_selector(self) -> String {
        match self {
            Self::Block { block_index } => format!("reader-inline-{block_index}-0"),
            Self::TableHeader {
                block_index,
                column_index,
            } => format!("reader-inline-{block_index}-header-{column_index}"),
            Self::TableCell {
                block_index,
                row_index,
                column_index,
            } => format!("reader-inline-{block_index}-cell-{row_index}-{column_index}"),
        }
    }

    fn focus_debug_selector(self, link_index: usize) -> String {
        match self {
            Self::Block { block_index } => {
                format!("reader-link-focus-{block_index}-{link_index}")
            }
            Self::TableHeader {
                block_index,
                column_index,
            } => format!("reader-link-focus-{block_index}-header-{column_index}-{link_index}"),
            Self::TableCell {
                block_index,
                row_index,
                column_index,
            } => format!(
                "reader-link-focus-{block_index}-cell-{row_index}-{column_index}-{link_index}"
            ),
        }
    }

    fn identity_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + 3 * size_of::<usize>());
        match self {
            Self::Block { block_index } => {
                bytes.push(0);
                bytes.extend_from_slice(&block_index.to_le_bytes());
            }
            Self::TableHeader {
                block_index,
                column_index,
            } => {
                bytes.push(1);
                bytes.extend_from_slice(&block_index.to_le_bytes());
                bytes.extend_from_slice(&column_index.to_le_bytes());
            }
            Self::TableCell {
                block_index,
                row_index,
                column_index,
            } => {
                bytes.push(2);
                bytes.extend_from_slice(&block_index.to_le_bytes());
                bytes.extend_from_slice(&row_index.to_le_bytes());
                bytes.extend_from_slice(&column_index.to_le_bytes());
            }
        }
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinkFocusKey {
    pub surface: LinkSurfaceKey,
    pub link_index: usize,
}

impl LinkFocusKey {
    pub const fn new(surface: LinkSurfaceKey, link_index: usize) -> Self {
        Self {
            surface,
            link_index,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFocusTarget {
    pub key: LinkFocusKey,
    pub target: String,
}

pub struct ReaderLinkState<'a> {
    pub hovered: Option<LinkFocusKey>,
    pub focused: Option<LinkFocusKey>,
    pub focus_handles: &'a HashMap<LinkFocusKey, FocusHandle>,
}

#[derive(Clone, Copy, Default)]
struct InlineStyleContext<'a> {
    emphasis: bool,
    strong: bool,
    code: bool,
    strikethrough: bool,
    footnote: bool,
    script: Option<Script>,
    mark: bool,
    kbd: bool,
    link_target: Option<&'a str>,
    link_node_id: Option<usize>,
}

impl InlineStyleContext<'_> {
    fn is_plain(&self) -> bool {
        !(self.emphasis
            || self.strong
            || self.code
            || self.strikethrough
            || self.footnote
            || self.mark
            || self.kbd
            || self.script.is_some()
            || self.link_target.is_some())
    }
}

pub fn inline_layout(spans: &[InlineSpan]) -> InlineLayout {
    inline_layout_with_transform(spans, false)
}

pub fn document_link_focus_targets(document: &ParsedDocument) -> Vec<LinkFocusTarget> {
    let mut targets = Vec::new();
    collect_link_focus_targets(
        &document.blocks,
        &mut Vec::new(),
        &document.path,
        &mut targets,
    );
    targets
}

pub fn block_link_focus_targets(
    document: &ParsedDocument,
    block_index: usize,
) -> Vec<LinkFocusTarget> {
    let Some(block) = document.blocks.get(block_index) else {
        return Vec::new();
    };
    let mut targets = Vec::new();
    let mut parent_path = vec![block_index];
    collect_current_block_link_targets(block, &mut parent_path, &document.path, &mut targets);
    targets
}

fn collect_link_focus_targets(
    blocks: &[DocumentBlock],
    parent_path: &mut Vec<usize>,
    document_path: &Path,
    targets: &mut Vec<LinkFocusTarget>,
) {
    for (child_index, block) in blocks.iter().enumerate() {
        parent_path.push(child_index);
        collect_current_block_link_targets(block, parent_path, document_path, targets);
        parent_path.pop();
    }
}

fn collect_current_block_link_targets(
    block: &DocumentBlock,
    parent_path: &mut Vec<usize>,
    document_path: &Path,
    targets: &mut Vec<LinkFocusTarget>,
) {
    let block_index = block_path_render_index(parent_path);
    match block {
        DocumentBlock::Heading { content, .. } | DocumentBlock::Paragraph(content) => {
            append_link_focus_targets(
                content,
                LinkSurfaceKey::block(block_index),
                document_path,
                targets,
            );
        }
        DocumentBlock::Table(table) => {
            for (column_index, content) in table.headers.iter().enumerate() {
                append_link_focus_targets(
                    content,
                    LinkSurfaceKey::table_header(block_index, column_index),
                    document_path,
                    targets,
                );
            }
            for (row_index, row) in table.rows.iter().enumerate() {
                for (column_index, content) in row.iter().enumerate() {
                    append_link_focus_targets(
                        content,
                        LinkSurfaceKey::table_cell(block_index, row_index, column_index),
                        document_path,
                        targets,
                    );
                }
            }
        }
        DocumentBlock::ListItem { children, .. }
        | DocumentBlock::TaskItem { children, .. }
        | DocumentBlock::Blockquote(children)
        | DocumentBlock::Alert { children, .. } => {
            collect_link_focus_targets(children, parent_path, document_path, targets);
        }
        DocumentBlock::FootnoteSection { notes } => {
            for (note_index, (_, children)) in notes.iter().enumerate() {
                parent_path.push(note_index);
                collect_link_focus_targets(children, parent_path, document_path, targets);
                parent_path.pop();
            }
        }
        DocumentBlock::CodeBlock { .. }
        | DocumentBlock::MermaidCard { .. }
        | DocumentBlock::Math { .. }
        | DocumentBlock::Image { .. }
        | DocumentBlock::ThematicBreak
        | DocumentBlock::RawText(_) => {}
    }
}

fn append_link_focus_targets(
    spans: &[InlineSpan],
    surface: LinkSurfaceKey,
    document_path: &Path,
    targets: &mut Vec<LinkFocusTarget>,
) {
    for (link_index, link) in inline_layout(spans)
        .links
        .into_iter()
        .filter(|link| !matches!(classify_link(document_path, &link.target), LinkRoute::Inert))
        .enumerate()
    {
        targets.push(LinkFocusTarget {
            key: LinkFocusKey::new(surface, link_index),
            target: link.target,
        });
    }
}

fn block_path_render_index(block_path: &[usize]) -> usize {
    if let [block_index] = block_path {
        return *block_index;
    }

    let mut hash = 0xcbf29ce484222325_u64;
    for index in block_path {
        for byte in index.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    (hash as usize) | (1_usize << (usize::BITS - 1))
}

fn block_path_suffix(block_path: &[usize]) -> String {
    block_path
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("-")
}

fn inline_layout_with_transform(spans: &[InlineSpan], uppercase: bool) -> InlineLayout {
    let mut layout = InlineLayout {
        text: String::new(),
        styles: Vec::new(),
        links: Vec::new(),
        math: Vec::new(),
    };
    let mut next_link_node_id = 0;
    append_inline_spans(
        spans,
        InlineStyleContext::default(),
        uppercase,
        &mut next_link_node_id,
        &mut layout,
    );
    layout
}

fn append_inline_spans<'a>(
    spans: &'a [InlineSpan],
    style: InlineStyleContext<'a>,
    uppercase: bool,
    next_link_node_id: &mut usize,
    layout: &mut InlineLayout,
) {
    for span in spans {
        match span {
            InlineSpan::Text(text) => match style.script {
                // Keep painted bytes identical to `InlineSpan::find_text`.
                Some(script) => {
                    append_inline_text(&script_text(text, script), style, false, layout)
                }
                None => append_inline_text(text, style, uppercase, layout),
            },
            InlineSpan::Superscript(content) | InlineSpan::Subscript(content) => {
                append_inline_spans(
                    content,
                    InlineStyleContext {
                        script: Some(if matches!(span, InlineSpan::Superscript(_)) {
                            Script::Super
                        } else {
                            Script::Sub
                        }),
                        ..style
                    },
                    uppercase,
                    next_link_node_id,
                    layout,
                )
            }
            InlineSpan::Mark(content) => append_inline_spans(
                content,
                InlineStyleContext {
                    mark: true,
                    ..style
                },
                uppercase,
                next_link_node_id,
                layout,
            ),
            InlineSpan::Kbd(content) => append_inline_spans(
                content,
                InlineStyleContext { kbd: true, ..style },
                uppercase,
                next_link_node_id,
                layout,
            ),
            InlineSpan::FootnoteBackref { label } => {
                let target = footnote_backref_target(label);
                let link_node_id = *next_link_node_id;
                *next_link_node_id += 1;
                append_inline_text(
                    FOOTNOTE_BACKREF,
                    InlineStyleContext {
                        footnote: true,
                        link_target: Some(&target),
                        link_node_id: Some(link_node_id),
                        ..style
                    },
                    false,
                    layout,
                );
            }
            InlineSpan::Emphasis(content) => append_inline_spans(
                content,
                InlineStyleContext {
                    emphasis: true,
                    ..style
                },
                uppercase,
                next_link_node_id,
                layout,
            ),
            InlineSpan::Strong(content) => append_inline_spans(
                content,
                InlineStyleContext {
                    strong: true,
                    ..style
                },
                uppercase,
                next_link_node_id,
                layout,
            ),
            InlineSpan::Strikethrough(content) => append_inline_spans(
                content,
                InlineStyleContext {
                    strikethrough: true,
                    ..style
                },
                uppercase,
                next_link_node_id,
                layout,
            ),
            InlineSpan::Code(code) => append_inline_text(
                code,
                InlineStyleContext {
                    code: true,
                    ..style
                },
                uppercase,
                layout,
            ),
            // Footnote references are links to their note (`#fn-label`).
            InlineSpan::FootnoteRef { label } => {
                let target = footnote_target(label);
                let link_node_id = *next_link_node_id;
                *next_link_node_id += 1;
                append_inline_text(
                    &footnote_ref_display(label),
                    InlineStyleContext {
                        footnote: true,
                        link_target: Some(&target),
                        link_node_id: Some(link_node_id),
                        ..style
                    },
                    false,
                    layout,
                );
            }
            InlineSpan::Link { label, target } => {
                let link_node_id = *next_link_node_id;
                *next_link_node_id += 1;
                append_inline_spans(
                    label,
                    InlineStyleContext {
                        link_target: Some(target),
                        link_node_id: Some(link_node_id),
                        ..style
                    },
                    uppercase,
                    next_link_node_id,
                    layout,
                )
            }
            InlineSpan::Math { tex, display } => {
                let start = layout.text.len();
                append_inline_text(
                    &math_source(tex, *display),
                    InlineStyleContext {
                        code: true,
                        ..style
                    },
                    false,
                    layout,
                );
                layout.math.push(InlineMath {
                    range: start..layout.text.len(),
                    tex: tex.clone(),
                    display: *display,
                });
            }
            // Electron renders soft breaks outside code as <br> (breaksOutsideCode).
            InlineSpan::SoftBreak => append_inline_text("\n", style, false, layout),
            InlineSpan::HardBreak => append_inline_text("\n", style, false, layout),
        }
    }
}

/// Swaps each formula's source text for [`MATH_PLACEHOLDER`] characters at least as wide as the
/// typeset formula, so native text layout (and wrapping) leaves room to paint it. `width_em`
/// returns a formula's width in ems of the text, or `None` when it does not typeset (its source
/// then stays as code-styled fallback text). `placeholder_em` is one placeholder's advance in ems.
/// Returns the formulas that received placeholders, with their new ranges.
fn reserve_inline_math(
    layout: &mut InlineLayout,
    placeholder_em: f32,
    width_em: impl Fn(&str, bool) -> Option<f32>,
) -> Vec<InlineMath> {
    if layout.math.is_empty() || placeholder_em <= 0.0 {
        return Vec::new();
    }
    let mut edits = Vec::<(Range<usize>, usize)>::new();
    let mut reserved = Vec::new();
    let mut text = String::with_capacity(layout.text.len());
    let mut copied = 0;
    for math in &layout.math {
        let Some(width) = width_em(&math.tex, math.display) else {
            continue;
        };
        let count = ((width / placeholder_em).ceil() as usize).max(1);
        text.push_str(&layout.text[copied..math.range.start]);
        let start = text.len();
        text.extend(std::iter::repeat_n(MATH_PLACEHOLDER, count));
        edits.push((math.range.clone(), text.len() - start));
        reserved.push(InlineMath {
            range: start..text.len(),
            tex: math.tex.clone(),
            display: math.display,
        });
        copied = math.range.end;
    }
    if edits.is_empty() {
        return Vec::new();
    }
    text.push_str(&layout.text[copied..]);

    let shift = |index: usize| {
        edits
            .iter()
            .filter(|(old, _)| old.end <= index)
            .fold(index, |index, (old, new_len)| index - old.len() + new_len)
    };
    for style in &mut layout.styles {
        if edits.iter().any(|(old, _)| *old == style.range) {
            style.code = false;
            style.math = true;
        }
        style.range = shift(style.range.start)..shift(style.range.end);
    }
    for link in &mut layout.links {
        link.range = shift(link.range.start)..shift(link.range.end);
    }
    layout.text = text;
    layout.math = reserved.clone();
    reserved
}

/// The font inline math placeholders are laid out in. Ligatures and kerning are off so every
/// placeholder advances by exactly the measured width.
fn math_placeholder_font(reader: ReaderStyle) -> Font {
    let mut placeholder = font(reader.content_family);
    placeholder.features = FontFeatures(Arc::new(vec![
        ("liga".into(), 0),
        ("calt".into(), 0),
        ("kern".into(), 0),
    ]));
    placeholder
}

/// The TeX source with its delimiters, shown when math cannot be typeset.
fn math_source(tex: &str, display: bool) -> String {
    if display {
        format!("$${tex}$$")
    } else {
        format!("${tex}$")
    }
}

fn append_inline_text(
    text: &str,
    style: InlineStyleContext<'_>,
    uppercase: bool,
    layout: &mut InlineLayout,
) {
    if text.is_empty() {
        return;
    }
    let start = layout.text.len();
    if uppercase {
        layout.text.push_str(&text.to_uppercase());
    } else {
        layout.text.push_str(text);
    }
    let range = start..layout.text.len();
    if !style.is_plain() {
        layout
            .styles
            .push(InlineStyleRange::new(range.clone(), style));
    }
    if let Some(target) = style.link_target {
        if let Some(link) = layout.links.last_mut()
            && Some(link.node_id) == style.link_node_id
            && link.range.end == range.start
        {
            link.range.end = range.end;
        } else {
            layout.links.push(InlineLink {
                range,
                target: target.to_owned(),
                node_id: style
                    .link_node_id
                    .expect("link text always carries its source-node identity"),
            });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkRoute {
    Markdown(PathBuf),
    Anchor(String),
    Web(String),
    Local(PathBuf),
    Inert,
}

pub fn classify_link(document_path: &Path, target: &str) -> LinkRoute {
    if let Some(fragment) = target.strip_prefix('#') {
        return LinkRoute::Anchor(fragment.to_owned());
    }
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return LinkRoute::Web(target.to_owned());
    }
    let Some(path) = resolve_local_target(document_path, target) else {
        return LinkRoute::Inert;
    };
    if is_supported_document(&path) {
        LinkRoute::Markdown(path)
    } else {
        LinkRoute::Local(path)
    }
}

/// Where an image's pixels come from. Electron's CSP (`img-src 'self' mdow-local: data: blob:`)
/// renders local files and `data:` URIs but blocks every remote http(s) image, so Native shows
/// the alt-text placeholder for remote images instead of fetching them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageTarget {
    Local(PathBuf),
    Data(gpui::ImageFormat, Vec<u8>),
    /// http(s): blocked, like Electron.
    Remote,
    Unavailable,
}

/// Decoded `data:` images larger than this show the placeholder.
pub const MAX_DATA_IMAGE_BYTES: usize = 16 * 1024 * 1024;

pub fn classify_image(document_path: &Path, source: &str) -> ImageTarget {
    let source = source.trim();
    let lower = source.get(..8).unwrap_or(source).to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || source.starts_with("//") {
        return ImageTarget::Remote;
    }
    if lower.starts_with("data:") {
        return decode_data_image(source).map_or(ImageTarget::Unavailable, |(format, bytes)| {
            ImageTarget::Data(format, bytes)
        });
    }
    resolve_image_target(document_path, source).map_or(ImageTarget::Unavailable, ImageTarget::Local)
}

fn decode_data_image(source: &str) -> Option<(gpui::ImageFormat, Vec<u8>)> {
    let (header, payload) = source.get(5..)?.split_once(',')?;
    let mut parts = header.split(';');
    let mime = parts.next()?.trim().to_ascii_lowercase();
    let base64 = parts.any(|part| part.trim().eq_ignore_ascii_case("base64"));
    let format = gpui::ImageFormat::from_mime_type(&mime)?;
    // Base64 inflates by 4/3; reject oversized payloads before decoding them.
    if payload.len() / 4 * 3 > MAX_DATA_IMAGE_BYTES {
        return None;
    }
    let bytes = if base64 {
        decode_base64(payload)?
    } else {
        percent_decode_bytes(payload)?
    };
    (!bytes.is_empty() && bytes.len() <= MAX_DATA_IMAGE_BYTES).then_some((format, bytes))
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u32> {
        match byte {
            b'A'..=b'Z' => Some(u32::from(byte - b'A')),
            b'a'..=b'z' => Some(u32::from(byte - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(byte - b'0') + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut output = Vec::with_capacity(input.len() / 4 * 3);
    let mut buffer = 0_u32;
    let mut bits = 0;
    for byte in input.bytes() {
        if byte.is_ascii_whitespace() || byte == b'%' {
            continue;
        }
        if byte == b'=' {
            break;
        }
        buffer = (buffer << 6) | value(byte)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(output)
}

fn percent_decode_bytes(input: &str) -> Option<Vec<u8>> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = input.get(index + 1..index + 3)?;
            output.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    Some(output)
}

/// Decoded `data:` images, keyed by their source, so repaints do not decode base64 again.
fn data_image(source: &str) -> Option<Arc<gpui::Image>> {
    use std::hash::{Hash, Hasher};
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<u64, Arc<gpui::Image>>>> =
        std::sync::OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    let key = hasher.finish();
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(image) = cache.get(&key) {
        return Some(image.clone());
    }
    let (format, bytes) = decode_data_image(source.trim())?;
    if cache.len() >= 64 {
        cache.clear();
    }
    let image = Arc::new(gpui::Image::from_bytes(format, bytes));
    cache.insert(key, image.clone());
    Some(image)
}

pub fn resolve_image_target(document_path: &Path, source: &str) -> Option<PathBuf> {
    let path = resolve_local_target(document_path, source)?;
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    (path.is_file() && gpui::Img::extensions().contains(&extension.as_str())).then_some(path)
}

pub fn code_copy_feedback_is_active(
    copied_code: Option<(usize, Instant)>,
    block_index: usize,
    now: Instant,
) -> bool {
    copied_code.is_some_and(|(index, copied_at)| {
        index == block_index
            && now.saturating_duration_since(copied_at) < CODE_COPY_FEEDBACK_DURATION
    })
}

pub fn clear_expired_code_copy_feedback(
    copied_code: &mut Option<(usize, Instant)>,
    block_index: usize,
    now: Instant,
) -> bool {
    if copied_code.is_some_and(|(index, copied_at)| {
        index == block_index
            && now.saturating_duration_since(copied_at) >= CODE_COPY_FEEDBACK_DURATION
    }) {
        *copied_code = None;
        true
    } else {
        false
    }
}

fn restrict_scroll_to_axis<E: Styled>(mut element: E) -> E {
    element.style().restrict_scroll_to_axis = Some(true);
    element
}

fn document_scoped_element_id(document_path: &Path, role: &str, block_index: usize) -> u64 {
    document_scoped_identity_id(document_path, role, &block_index.to_le_bytes())
}

fn document_scoped_identity_id(document_path: &Path, role: &str, identity: &[u8]) -> u64 {
    // FNV-1a gives us a deterministic, inexpensive identity without retaining the full path in
    // GPUI's element-id tree.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in role
        .as_bytes()
        .iter()
        .chain(document_path.as_os_str().as_encoded_bytes())
        .chain(identity)
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn link_surface_element_id(document_path: &Path, role: &str, surface: LinkSurfaceKey) -> u64 {
    document_scoped_identity_id(document_path, role, &surface.identity_bytes())
}

fn link_focus_element_id(document_path: &Path, role: &str, key: LinkFocusKey) -> u64 {
    let mut identity = key.surface.identity_bytes();
    identity.extend_from_slice(&key.link_index.to_le_bytes());
    document_scoped_identity_id(document_path, role, &identity)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockSpacing {
    pub before: f32,
    pub after: f32,
}

#[derive(Debug, Clone, Copy)]
struct BlockMargins {
    top: f32,
    bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListGroup {
    Unordered(usize),
    Ordered(usize),
}

fn list_group(block: &DocumentBlock) -> Option<ListGroup> {
    match block {
        DocumentBlock::ListItem {
            kind: ListKind::Ordered { .. },
            depth,
            ..
        } => Some(ListGroup::Ordered(*depth)),
        DocumentBlock::ListItem {
            kind: ListKind::Unordered,
            depth,
            ..
        }
        | DocumentBlock::TaskItem { depth, .. } => Some(ListGroup::Unordered(*depth)),
        _ => None,
    }
}

fn list_marker_is_visible(blocks: &[DocumentBlock], block_index: usize) -> bool {
    if !matches!(
        blocks.get(block_index),
        Some(DocumentBlock::ListItem {
            kind: ListKind::Unordered,
            ..
        })
    ) {
        return true;
    }

    let group = list_group(&blocks[block_index]).expect("unordered list group");
    let group_start = (0..block_index)
        .rev()
        .take_while(|index| list_group(&blocks[*index]) == Some(group))
        .last()
        .unwrap_or(block_index);
    let group_end = (block_index + 1..blocks.len())
        .take_while(|index| list_group(&blocks[*index]) == Some(group))
        .last()
        .map_or(block_index + 1, |index| index + 1);

    !blocks[group_start..group_end]
        .iter()
        .any(|block| matches!(block, DocumentBlock::TaskItem { .. }))
}

fn block_margins(
    block: &DocumentBlock,
    previous: Option<&DocumentBlock>,
    next: Option<&DocumentBlock>,
) -> BlockMargins {
    // Electron's markdown.css margins, in em of the 15.5px body.
    const EM: f32 = READER_FONT_SIZE;
    if let Some(group) = list_group(block) {
        return BlockMargins {
            top: if previous.and_then(list_group) == Some(group) {
                EM * 0.35
            } else {
                EM
            },
            bottom: if next.and_then(list_group) == Some(group) {
                EM * 0.25
            } else {
                EM
            },
        };
    }

    match block {
        DocumentBlock::Heading { level, .. } => {
            let style = BlockStyle::heading(*level);
            BlockMargins {
                top: style.font_size * style.margin_top_em,
                bottom: style.font_size * style.margin_bottom_em,
            }
        }
        DocumentBlock::CodeBlock { .. }
        | DocumentBlock::Table(_)
        | DocumentBlock::Alert { .. }
        | DocumentBlock::FootnoteSection { .. } => BlockMargins {
            top: EM * 1.25,
            bottom: EM * 1.25,
        },
        // The Electron reader gives diagrams a 1.5em margin.
        DocumentBlock::MermaidCard { .. } => BlockMargins {
            top: EM * 1.5,
            bottom: EM * 1.5,
        },
        DocumentBlock::ThematicBreak => BlockMargins {
            top: EM * 2.0,
            bottom: EM * 2.0,
        },
        // KaTeX display math keeps a 1em margin above and below.
        DocumentBlock::Math { .. } => BlockMargins {
            top: EM,
            bottom: EM,
        },
        DocumentBlock::Paragraph(_)
        | DocumentBlock::Blockquote(_)
        | DocumentBlock::Image { .. }
        | DocumentBlock::RawText(_) => BlockMargins {
            top: 0.0,
            bottom: EM,
        },
        DocumentBlock::ListItem { .. } | DocumentBlock::TaskItem { .. } => unreachable!(),
    }
}

pub fn block_sequence_spacing(blocks: &[DocumentBlock]) -> Vec<BlockSpacing> {
    let margins = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| {
            block_margins(
                block,
                index
                    .checked_sub(1)
                    .and_then(|previous| blocks.get(previous)),
                blocks.get(index + 1),
            )
        })
        .collect::<Vec<_>>();

    margins
        .iter()
        .enumerate()
        .map(|(index, margin)| BlockSpacing {
            before: if index == 0 && matches!(blocks.first(), Some(DocumentBlock::Heading { .. })) {
                0.0
            } else if let Some(previous) = index.checked_sub(1).and_then(|i| margins.get(i)) {
                previous.bottom.max(margin.top)
            } else {
                margin.top
            },
            after: if index + 1 == margins.len() {
                margin.bottom
            } else {
                0.0
            },
        })
        .collect()
}

fn style_reader_image(image: Img) -> Img {
    image.max_w(relative(1.0)).rounded(px(8.0))
}

const READER_LIST_OVERDRAW: f32 = 720.0;

pub(crate) struct ReaderPane {
    app: WeakEntity<MdowApp>,
    document: Arc<PreparedDocument>,
    style: ReaderStyle,
    theme: Theme,
    list_state: ListState,
    heading_blocks: Vec<usize>,
    scrollbar_drag_grab_y: Option<f32>,
}

impl ReaderPane {
    pub(crate) fn new(
        app: WeakEntity<MdowApp>,
        document: Arc<PreparedDocument>,
        style: ReaderStyle,
        theme: Theme,
    ) -> Self {
        let list_state = ListState::new(
            document.blocks.len(),
            ListAlignment::Top,
            px(READER_LIST_OVERDRAW),
        );
        Self {
            app,
            heading_blocks: document.heading_blocks(),
            document,
            style,
            theme,
            list_state,
            scrollbar_drag_grab_y: None,
        }
    }

    /// The outline heading at or above the top of the viewport: the first heading in the top
    /// block when it has any, otherwise the last heading before it.
    pub(crate) fn active_heading(&self) -> Option<usize> {
        active_heading_for(
            &self.heading_blocks,
            self.list_state.logical_scroll_top().item_ix,
        )
    }

    #[cfg(test)]
    pub(crate) fn list_state(&self) -> ListState {
        self.list_state.clone()
    }

    pub(crate) fn hosts_document(&self, document: &Arc<PreparedDocument>) -> bool {
        Arc::ptr_eq(&self.document, document)
    }

    pub(crate) fn sync(
        &mut self,
        document: Arc<PreparedDocument>,
        style: ReaderStyle,
        theme: Theme,
        cx: &mut Context<Self>,
    ) {
        let mut notify = false;
        if !Arc::ptr_eq(&self.document, &document) {
            let offset = self.list_state.logical_scroll_top();
            self.document = document;
            self.heading_blocks = self.document.heading_blocks();
            self.list_state.reset(self.document.blocks.len());
            if offset.item_ix < self.document.blocks.len() {
                self.list_state.scroll_to(offset);
            }
            notify = true;
        }
        if self.style != style {
            let offset = self.list_state.logical_scroll_top();
            self.style = style;
            self.list_state.reset(self.document.blocks.len());
            if offset.item_ix < self.document.blocks.len() {
                self.list_state.scroll_to(offset);
            }
            notify = true;
        }
        if self.theme != theme {
            self.theme = theme;
            notify = true;
        }
        if notify {
            cx.notify();
        }
    }

    pub(crate) fn scroll_to_block(&self, block: usize) {
        if block < self.list_state.item_count() {
            self.list_state.scroll_to(ListOffset {
                item_ix: block,
                offset_in_item: px(0.0),
            });
        }
    }

    pub(crate) fn scroll_by_key(&self, key: &str) -> bool {
        let viewport = f32::from(self.list_state.viewport_bounds().size.height);
        let max = f32::from(self.list_state.max_offset_for_scrollbar().height);
        let current = f32::from(self.list_state.scroll_px_offset_for_scrollbar().y);
        let Some(target) = reader_key_target(key, current, viewport, max) else {
            return false;
        };
        self.list_state
            .set_offset_from_scrollbar(point(px(0.0), px(target)));
        true
    }

    fn begin_scrollbar_drag(&mut self, grab_y: f32) {
        self.scrollbar_drag_grab_y = Some(grab_y);
        self.list_state.scrollbar_drag_started();
    }

    fn end_scrollbar_drag(&mut self) {
        self.scrollbar_drag_grab_y = None;
        self.list_state.scrollbar_drag_ended();
    }
}

fn active_heading_for(heading_blocks: &[usize], top_block: usize) -> Option<usize> {
    let seen = heading_blocks.partition_point(|&block| block <= top_block);
    let block = *heading_blocks.get(seen.checked_sub(1)?)?;
    Some(heading_blocks.partition_point(|&earlier| earlier < block))
}

const READER_LINE_STEP: f32 = 40.0;

pub(crate) fn reader_key_target(key: &str, current: f32, viewport: f32, max: f32) -> Option<f32> {
    let page = viewport * 0.9;
    match key {
        "home" => Some(0.0),
        "end" => Some(-max),
        "up" => Some((current + READER_LINE_STEP).min(0.0)),
        "down" => Some((current - READER_LINE_STEP).max(-max)),
        "pageup" => Some((current + page).min(0.0)),
        "pagedown" => Some((current - page).max(-max)),
        _ => None,
    }
}

fn render_reader_scrollbar(
    document_path: &Path,
    list_state: &ListState,
    theme: Theme,
    cx: &Context<ReaderPane>,
) -> Option<AnyElement> {
    let geometry = reader_scrollbar_geometry(
        f32::from(list_state.viewport_bounds().size.height),
        f32::from(list_state.max_offset_for_scrollbar().height),
        f32::from(list_state.scroll_px_offset_for_scrollbar().y),
    )?;
    let entity = cx.entity();
    let event_handle = list_state.clone();
    let thumb_color = theme.muted_foreground.opacity(match theme.color_scheme {
        ColorScheme::Light => 0.25,
        ColorScheme::Dark => 0.20,
    });
    let thumb_hover_color = theme.muted_foreground.opacity(match theme.color_scheme {
        ColorScheme::Light => 0.45,
        ColorScheme::Dark => 0.40,
    });

    Some(
        div()
            .id((
                "reader-scrollbar-track",
                document_scoped_element_id(document_path, "reader-scrollbar-track", 0),
            ))
            .debug_selector(|| "reader-scrollbar-track".into())
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(6.0))
            .cursor_pointer()
            .child(
                canvas(
                    |_, _, _| (),
                    move |track_bounds, _, window, _| {
                        window.on_mouse_event({
                            let entity = entity.clone();
                            let handle = event_handle.clone();
                            move |event: &MouseDownEvent, _, _, cx| {
                                if event.button != MouseButton::Left
                                    || !track_bounds.contains(&event.position)
                                {
                                    return;
                                }

                                let pointer_y = f32::from(event.position.y - track_bounds.origin.y);
                                if pointer_y >= geometry.thumb_top
                                    && pointer_y <= geometry.thumb_top + geometry.thumb_height
                                {
                                    let grab_y = pointer_y - geometry.thumb_top;
                                    entity.update(cx, |this, _| {
                                        this.begin_scrollbar_drag(grab_y);
                                    });
                                } else {
                                    let target = reader_scrollbar_offset_for_pointer(
                                        pointer_y,
                                        geometry.thumb_height / 2.0,
                                        geometry,
                                    );
                                    handle.set_offset_from_scrollbar(point(px(0.0), px(target)));
                                    entity.update(cx, |this, _| {
                                        this.end_scrollbar_drag();
                                    });
                                    cx.notify(entity.entity_id());
                                }
                            }
                        });
                        window.on_mouse_event({
                            let entity = entity.clone();
                            move |event: &MouseUpEvent, _, _, cx| {
                                if event.button == MouseButton::Left {
                                    entity.update(cx, |this, _| {
                                        this.end_scrollbar_drag();
                                    });
                                }
                            }
                        });
                        window.on_mouse_event({
                            let entity = entity.clone();
                            let handle = event_handle.clone();
                            move |event: &MouseMoveEvent, _, _, cx| {
                                if !event.dragging() {
                                    return;
                                }
                                let Some(grab_y) = entity.read(cx).scrollbar_drag_grab_y else {
                                    return;
                                };
                                let pointer_y = f32::from(event.position.y - track_bounds.origin.y);
                                let target = reader_scrollbar_offset_for_pointer(
                                    pointer_y, grab_y, geometry,
                                );
                                handle.set_offset_from_scrollbar(point(px(0.0), px(target)));
                                cx.notify(entity.entity_id());
                            }
                        });
                    },
                )
                .size_full(),
            )
            .child(
                div()
                    .id((
                        "reader-scrollbar-thumb",
                        document_scoped_element_id(document_path, "reader-scrollbar-thumb", 0),
                    ))
                    .debug_selector(|| "reader-scrollbar-thumb".into())
                    .absolute()
                    .top(px(geometry.thumb_top))
                    .right_0()
                    .h(px(geometry.thumb_height))
                    .w_full()
                    .rounded(px(999.0))
                    .bg(thumb_color)
                    .hover(move |thumb| thumb.bg(thumb_hover_color)),
            )
            .into_any_element(),
    )
}

#[derive(Clone, Copy)]
struct ReaderView<'a> {
    style: ReaderStyle,
    theme: Theme,
    copied_code: Option<(usize, Instant)>,
    link_state: &'a ReaderLinkState<'a>,
    find_block: Option<usize>,
    /// Body text inside blockquotes uses the muted foreground.
    muted: bool,
}

impl ReaderView<'_> {
    fn zoom(self, base: f32) -> f32 {
        base * (self.style.font_size / READER_FONT_SIZE)
    }

    fn text_color(self) -> gpui::Hsla {
        if self.muted {
            self.theme.muted_foreground
        } else {
            self.theme.foreground
        }
    }
}

/// `color-mix(in oklch, var(--muted-foreground) 45%, transparent)`.
fn blockquote_border(theme: Theme) -> gpui::Hsla {
    theme.muted_foreground.opacity(0.45)
}

#[allow(clippy::too_many_arguments)]
fn render_reader_item(
    document: &PreparedDocument,
    block_index: usize,
    style: ReaderStyle,
    theme: Theme,
    copied_code: Option<(usize, Instant)>,
    link_state: &ReaderLinkState<'_>,
    find_block: Option<usize>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let Some(block) = document.blocks.get(block_index) else {
        return div().into_any_element();
    };
    let last = document.blocks.len().saturating_sub(1);
    let spacing = block_sequence_spacing(&document.blocks);
    let view = ReaderView {
        style,
        theme,
        copied_code,
        link_state,
        find_block,
        muted: false,
    };
    // List items have no flex parent, so center the column in a full-width row.
    let column = div()
        .id(("reader-column", block_index))
        .debug_selector(|| "reader-column".into())
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .px(px(Metrics::READER_INSET))
        .when(block_index == 0, |item| {
            item.pt(px(Metrics::READER_TOP_PADDING))
        })
        .when(block_index == last, |item| {
            item.pb(px(Metrics::READER_BOTTOM_PADDING))
        })
        .font_family(style.content_family)
        .font_weight(FontWeight::NORMAL)
        .text_size(px(style.font_size))
        .line_height(px(style.font_size * style.line_height))
        .text_color(theme.foreground)
        .when_some(style.max_width, |item, width| item.max_w(px(width)))
        .child(render_block(
            document,
            block,
            &[block_index],
            None,
            spacing[block_index],
            list_marker_is_visible(&document.blocks, block_index),
            view,
            cx,
        ));
    div()
        .flex()
        .justify_center()
        .w_full()
        .min_w_0()
        .child(column)
        .into_any_element()
}

impl Render for ReaderPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app = self.app.clone();
        let document = self.document.clone();
        let style = self.style;
        let theme = self.theme;
        let list_state = self.list_state.clone();
        let viewport = list(list_state.clone(), move |block_index, _, cx| {
            app.update(cx, |app, cx| {
                let handles = app.ensure_block_link_focus_handles(&document, block_index, cx);
                let paint = app.reader_paint_state(&document.path, cx);
                let link_state = ReaderLinkState {
                    hovered: paint.hovered_link,
                    focused: paint.focused_link,
                    focus_handles: &handles,
                };
                render_reader_item(
                    &document,
                    block_index,
                    style,
                    theme,
                    paint.copied_code,
                    &link_state,
                    paint.find_block,
                    cx,
                )
            })
            .unwrap_or_else(|_| div().into_any_element())
        })
        .w_full()
        .h_full();
        let scrollbar = render_reader_scrollbar(&self.document.path, &self.list_state, theme, cx);

        div()
            .id("reader-scroll")
            .debug_selector(|| "reader-scroll".into())
            .relative()
            .flex()
            .flex_col()
            .flex_grow()
            .min_w_0()
            .min_h_0()
            .bg(theme.background)
            .child(viewport)
            .when_some(scrollbar, |reader, scrollbar| reader.child(scrollbar))
    }
}

#[allow(clippy::too_many_arguments)]
fn render_block(
    document: &PreparedDocument,
    block: &DocumentBlock,
    block_path: &[usize],
    parent_list_depth: Option<usize>,
    spacing: BlockSpacing,
    list_marker_visible: bool,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let link_state = view.link_state;
    let find_block = view.find_block;
    let block_index = block_path_render_index(block_path);
    let block_suffix = block_path_suffix(block_path);
    let document_path = document.path.as_path();
    let content =
        match block {
            DocumentBlock::Heading { level, content } => {
                let style = BlockStyle::heading(*level);
                let font_size = view.zoom(style.font_size);
                let debug_selector = format!("reader-block-{block_suffix}");
                div()
                    .id(("reader-block", block_index))
                    .debug_selector(move || debug_selector)
                    .w_full()
                    .min_w_0()
                    .font_weight(FontWeight(style.font_weight as f32))
                    .text_size(px(font_size))
                    .line_height(px(font_size * style.line_height))
                    .text_color(if style.muted {
                        theme.muted_foreground
                    } else {
                        theme.foreground
                    })
                    .child(render_inline_layout(
                        inline_layout_with_transform(content, style.uppercase),
                        document_path,
                        LinkSurfaceKey::block(block_index),
                        style.font_weight,
                        if style.muted {
                            theme.muted_foreground
                        } else {
                            theme.foreground
                        },
                        theme,
                        view.style,
                        false,
                        link_state,
                        cx,
                    ))
                    .into_any_element()
            }
            DocumentBlock::Paragraph(content) => {
                let debug_selector = format!("reader-block-{block_suffix}");
                div()
                    .id(("reader-block", block_index))
                    .debug_selector(move || debug_selector)
                    .w_full()
                    .min_w_0()
                    .child(render_inline(
                        content,
                        document_path,
                        LinkSurfaceKey::block(block_index),
                        400,
                        view.text_color(),
                        theme,
                        view.style,
                        link_state,
                        cx,
                    ))
                    .into_any_element()
            }
            DocumentBlock::ListItem {
                kind,
                depth,
                children,
            } => render_list_item(
                kind,
                *depth,
                children,
                list_marker_visible,
                block_path,
                parent_list_depth,
                document,
                view,
                cx,
            ),
            DocumentBlock::TaskItem {
                checked,
                depth,
                children,
            } => render_task_item(
                *checked,
                *depth,
                children,
                block_path,
                parent_list_depth,
                document,
                view,
                cx,
            ),
            DocumentBlock::Blockquote(children) => {
                let debug_selector = format!("reader-block-{block_suffix}");
                let padding = BlockStyle::blockquote().padding;
                div()
                    .id(("reader-block", block_index))
                    .debug_selector(move || debug_selector)
                    .flex()
                    .w_full()
                    .min_w_0()
                    .border_l(px(3.0))
                    .border_color(blockquote_border(theme))
                    .py(px(padding[0]))
                    .text_color(theme.muted_foreground)
                    .child(div().min_w_0().flex_grow().px(px(padding[1])).child(
                        render_list_children(
                            children,
                            0,
                            block_path,
                            document,
                            ReaderView {
                                find_block: None,
                                muted: true,
                                ..view
                            },
                            cx,
                        ),
                    ))
                    .into_any_element()
            }
            DocumentBlock::ThematicBreak => {
                let debug_selector = format!("reader-block-{block_suffix}");
                div()
                    .id(("reader-block", block_index))
                    .debug_selector(move || debug_selector)
                    .w_full()
                    .child(div().h(px(1.0)).w_full().bg(theme.border))
                    .into_any_element()
            }
            DocumentBlock::CodeBlock {
                language,
                code,
                highlights,
            } => render_code_block(
                language.as_deref(),
                code,
                highlights,
                true,
                block_path,
                document_path,
                view,
                cx,
            ),
            DocumentBlock::Table(table) => {
                render_table(table, block_index, document_path, view, cx)
            }
            DocumentBlock::Image { alt, source } => {
                render_image(alt, source, block_index, document_path, theme)
            }
            DocumentBlock::Alert { kind, children } => {
                render_alert(*kind, children, block_path, document, view, cx)
            }
            DocumentBlock::MermaidCard { source } => {
                render_mermaid_block(source, block_path, document_path, view, cx)
            }
            DocumentBlock::Math { tex } => render_math_block(tex, block_path, document_path, view),
            DocumentBlock::FootnoteSection { notes } => {
                render_footnote_section(notes, block_path, document, view, cx)
            }
            DocumentBlock::RawText(text) => {
                let debug_selector = format!("reader-block-{block_suffix}");
                div()
                    .id(("reader-block", block_index))
                    .debug_selector(move || debug_selector)
                    .w_full()
                    .min_w_0()
                    .child(StyledText::new(text.clone()))
                    .into_any_element()
            }
        };

    let find_hit = block_path.len() == 1 && find_block == Some(block_path[0]);
    div()
        .w_full()
        .min_w_0()
        .mt(px(spacing.before))
        .mb(px(spacing.after))
        .when(find_hit, |block| {
            block.bg(theme.accent.opacity(0.14)).rounded(px(4.0))
        })
        .child(content)
        .into_any_element()
}

/// Display math, centered, scrolling sideways when wider than the column. TeX that does not
/// typeset shows its source in the destructive color, as KaTeX does with `throwOnError: false`.
fn render_math_block(
    tex: &str,
    block_path: &[usize],
    document_path: &Path,
    view: ReaderView<'_>,
) -> AnyElement {
    let theme = view.theme;
    let block_index = block_path_render_index(block_path);
    let debug_selector = format!("reader-block-{}", block_path_suffix(block_path));
    let content = match math_state(tex, true, theme.foreground, view.style.font_size) {
        GraphicState::Ready(graphic) => div()
            .flex()
            .w_full()
            .child(
                div()
                    .flex_none()
                    .mx_auto()
                    .py(px(4.0))
                    .child(GraphicElement::new(
                        graphic.image,
                        graphic.width,
                        graphic.height,
                        false,
                    )),
            )
            .into_any_element(),
        GraphicState::Pending | GraphicState::Failed(_) => div()
            .font_family(view.style.code_family)
            .text_size(px(view.zoom(14.0)))
            .text_color(theme.destructive)
            .whitespace_nowrap()
            .child(math_source(tex.trim(), true))
            .into_any_element(),
    };
    div()
        .id((
            "reader-math",
            document_scoped_element_id(document_path, "reader-math", block_index),
        ))
        .debug_selector(move || debug_selector)
        .w_full()
        .overflow_x_scroll()
        .map(restrict_scroll_to_axis)
        .child(content)
        .into_any_element()
}

/// Mermaid diagram in a code-block card (same radius, border, surface and 32px header with the
/// label left and Copy right), rendered off the UI thread the first time it scrolls near the
/// viewport and shrunk to the card like the Electron reader's `max-width: 100%`. Diagrams that
/// fail to parse show why above their source.
fn render_mermaid_block(
    source: &str,
    block_path: &[usize],
    document_path: &Path,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let block_index = block_path_render_index(block_path);
    let debug_selector = format!("reader-block-{}", block_path_suffix(block_path));
    let scale = view.zoom(1.0);
    // Edge labels sit on the card surface, so their backing matches it.
    let palette = DiagramPalette::new(
        theme.color_scheme == ColorScheme::Dark,
        hex_color(theme.code_surface),
    );
    let body = match diagram_state(source, palette, scale, cx) {
        GraphicState::Ready(graphic) => div()
            .flex()
            .justify_center()
            .w_full()
            .p(px(DIAGRAM_CARD_PADDING))
            .child(GraphicElement::new(
                graphic.image,
                graphic.width * scale,
                graphic.height * scale,
                true,
            )),
        GraphicState::Pending => div().w_full().h(px(view.zoom(DIAGRAM_PENDING_HEIGHT))),
        GraphicState::Failed(error) => {
            return div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .w_full()
                .min_w_0()
                .child(
                    div()
                        .font_family(Metrics::FONT_SANS)
                        .text_size(px(view.zoom(13.0)))
                        .text_color(theme.muted_foreground)
                        .child(format!("This diagram could not be drawn: {error}")),
                )
                .child(render_code_block(
                    Some("mermaid"),
                    source,
                    &LineHighlights::default(),
                    false,
                    block_path,
                    document_path,
                    view,
                    cx,
                ))
                .into_any_element();
        }
    };
    let copied = code_copy_feedback_is_active(view.copied_code, block_index, Instant::now());
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || debug_selector)
        .w_full()
        .min_w_0()
        .rounded(px(BlockStyle::code_block().radius))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.code_surface)
        .overflow_hidden()
        .child(code_card_header(
            Some("mermaid".into()),
            code_copy_button(source.to_owned(), block_path, copied, theme, cx),
            theme,
        ))
        .child(body)
        .into_any_element()
}

/// Inner padding between a diagram and its card border.
const DIAGRAM_CARD_PADDING: f32 = 16.0;

/// Space a diagram holds while it renders, so the column does not jump twice.
const DIAGRAM_PENDING_HEIGHT: f32 = 160.0;

/// The cached diagram for `source`, starting a background render on the first request. The app
/// re-renders when the diagram is ready.
fn diagram_state(
    source: &str,
    palette: DiagramPalette,
    scale: f32,
    cx: &Context<MdowApp>,
) -> GraphicState {
    let key = GraphicKey::Diagram {
        source: source.to_owned(),
        dark: palette.dark,
        scale: (scale * 100.0).round() as u32,
    };
    if let Some(state) = GraphicCache::global().get_or_begin(&key) {
        return state;
    }
    let source = source.to_owned();
    cx.spawn(async move |app, cx| {
        let rendered = cx
            .background_executor()
            .spawn(async move { render_mermaid(&source, &palette, scale) })
            .await;
        let state = match rendered {
            Ok(graphic) => GraphicState::Ready(graphic),
            Err(error) => GraphicState::Failed(error.into()),
        };
        GraphicCache::global().insert(key, state);
        app.update(cx, |_, cx| cx.notify()).ok();
    })
    .detach();
    GraphicState::Pending
}

#[allow(clippy::too_many_arguments)]
pub fn render_inline(
    spans: &[InlineSpan],
    document_path: &Path,
    surface: LinkSurfaceKey,
    base_weight: u16,
    base_color: gpui::Hsla,
    theme: Theme,
    style: ReaderStyle,
    link_state: &ReaderLinkState<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    render_inline_layout(
        inline_layout(spans),
        document_path,
        surface,
        base_weight,
        base_color,
        theme,
        style,
        false,
        link_state,
        cx,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_inline_layout(
    layout: InlineLayout,
    document_path: &Path,
    surface: LinkSurfaceKey,
    base_weight: u16,
    base_color: gpui::Hsla,
    theme: Theme,
    style: ReaderStyle,
    tabular_numbers: bool,
    link_state: &ReaderLinkState<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let mut layout = layout;
    let placeholder_font = math_placeholder_font(style);
    let reserved_math = if layout.math.is_empty() {
        Vec::new()
    } else {
        let text_system = cx.text_system();
        let placeholder_em = text_system
            .advance(
                text_system.resolve_font(&placeholder_font),
                px(16.0),
                MATH_PLACEHOLDER,
            )
            .map_or(0.0, |advance| f32::from(advance.width) / 16.0);
        reserve_inline_math(&mut layout, placeholder_em, |tex, display| {
            math_width_em(tex, display).map(|width| width * MATH_SCALE)
        })
    };
    let active_links = layout
        .links
        .iter()
        .filter(|link| !matches!(classify_link(document_path, &link.target), LinkRoute::Inert))
        .cloned()
        .collect::<Vec<_>>();
    let hovered_link_index = link_state
        .hovered
        .filter(|key| key.surface == surface)
        .map(|key| key.link_index);
    let focused_link_index = link_state
        .focused
        .filter(|key| key.surface == surface)
        .map(|key| key.link_index);
    let runs = text_runs(
        &layout,
        &active_links,
        hovered_link_index,
        focused_link_index,
        base_weight,
        base_color,
        theme,
        style,
        tabular_numbers,
    );
    let styled_text = StyledText::new(layout.text.clone()).with_runs(runs);
    let text_layout = styled_text.layout().clone();
    let math_slots = reserved_math
        .into_iter()
        .map(|math| InlineMathSlot {
            color: if active_links
                .iter()
                .any(|link| link.range.contains(&math.range.start))
            {
                theme.primary
            } else {
                base_color
            },
            range: math.range,
            tex: math.tex,
            display: math.display,
        })
        .collect::<Vec<_>>();
    let document_path = document_path.to_owned();
    let click_links = active_links.clone();
    let text: AnyElement = if click_links.is_empty() {
        styled_text.into_any_element()
    } else {
        let click_ranges = click_links
            .iter()
            .map(|link| link.range.clone())
            .collect::<Vec<_>>();
        let click_document_path = document_path.clone();
        let click_targets = click_links
            .iter()
            .map(|link| link.target.clone())
            .collect::<Vec<_>>();
        let hover_links = click_links.clone();
        let weak_app = cx.weak_entity();
        InteractiveText::new(
            (
                "reader-inline-text",
                link_surface_element_id(&document_path, "reader-inline-text", surface),
            ),
            styled_text,
        )
        .on_click(
            click_ranges,
            cx.processor(move |this, link_index: usize, _, cx| {
                if let Some(target) = click_targets.get(link_index).map(String::as_str) {
                    this.activate_link(&click_document_path, target, cx);
                }
            }),
        )
        .on_hover(move |character_index, _, _, cx| {
            let next = character_index
                .and_then(|character_index| {
                    hover_links
                        .iter()
                        .position(|link| link.range.contains(&character_index))
                })
                .map(|link_index| LinkFocusKey::new(surface, link_index));
            weak_app
                .update(cx, |this, cx| this.set_hovered_link(next, cx))
                .ok();
        })
        .into_any_element()
    };
    let text = if math_slots.is_empty() {
        text
    } else {
        InlineMathText::new(text, text_layout, math_slots, placeholder_font).into_any_element()
    };

    let keyboard_links = active_links
        .iter()
        .enumerate()
        .filter_map(|(link_index, link)| {
            link_state
                .focus_handles
                .get(&LinkFocusKey::new(surface, link_index))
                .cloned()
                .map(|handle| (link_index, link.target.clone(), handle))
        })
        .collect::<Vec<_>>();
    let weak_app = cx.weak_entity();
    let mut surface_element = div()
        .id((
            "reader-inline",
            link_surface_element_id(&document_path, "reader-inline", surface),
        ))
        .debug_selector(move || surface.debug_selector())
        .w_full()
        .min_w_0()
        .relative()
        .whitespace_normal()
        .on_hover(move |hovered, _, cx| {
            if !*hovered {
                weak_app
                    .update(cx, |this, cx| {
                        this.clear_hovered_link_for_surface(surface, cx)
                    })
                    .ok();
            }
        })
        // Keep every inline style in one StyledText/InteractiveText layout so wrapping remains
        // native text wrapping rather than flex-fragment wrapping.
        .child(text);
    for (link_index, target, focus_handle) in keyboard_links {
        let keyboard_document_path = document_path.clone();
        let focus_key = LinkFocusKey::new(surface, link_index);
        let focus_proxy_id =
            link_focus_element_id(&keyboard_document_path, "reader-link-focus", focus_key);
        let focus_debug_selector = surface.focus_debug_selector(link_index);
        surface_element = surface_element.child(
            div()
                .id(("reader-link-focus", focus_proxy_id))
                .debug_selector(move || focus_debug_selector.clone())
                .absolute()
                .top_0()
                .left_0()
                .size(px(0.0))
                .tab_index(0)
                .tab_group()
                .track_focus(&focus_handle)
                .on_key_up(cx.listener(move |this, event: &gpui::KeyUpEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                        this.activate_link(&keyboard_document_path, &target, cx);
                        cx.stop_propagation();
                    }
                })),
        );
    }
    surface_element.into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn text_runs(
    layout: &InlineLayout,
    active_links: &[InlineLink],
    hovered_link: Option<usize>,
    focused_link: Option<usize>,
    base_weight: u16,
    base_color: gpui::Hsla,
    theme: Theme,
    reader: ReaderStyle,
    tabular_numbers: bool,
) -> Vec<TextRun> {
    let mut runs = Vec::new();
    let mut cursor = 0;
    for style in &layout.styles {
        if cursor < style.range.start {
            runs.push(text_run(
                style.range.start - cursor,
                base_weight,
                base_color,
                false,
                false,
                false,
                false,
                tabular_numbers,
                theme,
                reader,
            ));
        }
        if style.math {
            runs.push(TextRun {
                len: style.range.len(),
                font: math_placeholder_font(reader),
                color: gpui::transparent_black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            });
            cursor = style.range.end;
            continue;
        }
        let link_index = style.link_target.as_ref().and_then(|_| {
            active_links
                .iter()
                .position(|link| link.range.contains(&style.range.start))
        });
        runs.push(text_run(
            style.range.len(),
            if style.strong { 700 } else { base_weight },
            if link_index.is_some() {
                theme.primary
            } else if style.footnote {
                theme.muted_foreground
            } else {
                base_color
            },
            style.emphasis,
            style.code,
            style.strikethrough,
            link_index.is_some_and(|link_index| {
                hovered_link == Some(link_index) || focused_link == Some(link_index)
            }),
            tabular_numbers,
            theme,
            reader,
        ));
        if let Some(run) = runs.last_mut() {
            apply_inline_extras(run, style, theme, reader);
        }
        cursor = style.range.end;
    }
    if cursor < layout.text.len() {
        runs.push(text_run(
            layout.text.len() - cursor,
            base_weight,
            base_color,
            false,
            false,
            false,
            false,
            tabular_numbers,
            theme,
            reader,
        ));
    }
    runs
}

/// Styles the inline HTML subset and super/subscripts on top of a base run.
fn apply_inline_extras(
    run: &mut TextRun,
    style: &InlineStyleRange,
    theme: Theme,
    reader: ReaderStyle,
) {
    if let Some(script) = style.script {
        // Unicode script characters are already raised/lowered; the OpenType feature covers
        // the rest in fonts that have it (see `document::Script`).
        let tag = match script {
            Script::Super => "sups",
            Script::Sub => "subs",
        };
        let mut features = run.font.features.tag_value_list().to_vec();
        features.push((tag.into(), 1));
        run.font.features = FontFeatures(Arc::new(features));
    }
    if style.mark {
        run.background_color = Some(theme.alert_warning.opacity(match theme.color_scheme {
            ColorScheme::Light => 0.22,
            ColorScheme::Dark => 0.30,
        }));
    }
    if style.kbd {
        // A keycap: the code face at medium weight on the border tone, a step darker than the
        // inline-code well so keys and code read differently.
        run.font.family = reader.code_family.into();
        run.font.weight = FontWeight::MEDIUM;
        run.background_color = Some(theme.border_subtle);
    }
}

#[allow(clippy::too_many_arguments)]
fn text_run(
    len: usize,
    weight: u16,
    color: gpui::Hsla,
    emphasis: bool,
    code: bool,
    strikethrough: bool,
    underline: bool,
    tabular_numbers: bool,
    theme: Theme,
    reader: ReaderStyle,
) -> TextRun {
    let mut run_font: Font = font(if code {
        reader.code_family
    } else {
        reader.content_family
    });
    run_font.weight = FontWeight(weight as f32);
    run_font.style = if emphasis {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };
    if tabular_numbers {
        run_font.features = FontFeatures(Arc::new(vec![("tnum".into(), 1)]));
    }
    TextRun {
        len,
        font: run_font,
        color,
        background_color: code.then_some(theme.muted),
        underline: underline.then_some(UnderlineStyle {
            thickness: px(1.0),
            color: Some(theme.primary),
            wavy: false,
        }),
        strikethrough: strikethrough.then_some(StrikethroughStyle {
            thickness: px(1.0),
            color: None,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_list_item(
    kind: &ListKind,
    depth: usize,
    children: &[DocumentBlock],
    marker_visible: bool,
    block_path: &[usize],
    parent_list_depth: Option<usize>,
    document: &PreparedDocument,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let block_index = block_path_render_index(block_path);
    let block_suffix = block_path_suffix(block_path);
    let block_debug_selector = format!("reader-block-{block_suffix}");
    let marker_debug_selector = format!("reader-list-marker-{block_suffix}");
    let marker = match kind {
        ListKind::Unordered => unordered_marker(depth).to_owned(),
        ListKind::Ordered { number } => format_ordered_marker(*number, depth),
    };
    let indentation_depth =
        parent_list_depth.map_or(depth, |parent_depth| depth.saturating_sub(parent_depth));
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || block_debug_selector)
        .flex()
        .items_start()
        .w_full()
        .min_w_0()
        .gap(px(if marker_visible { 8.0 } else { 0.0 }))
        .ml(px(
            indentation_depth as f32 * 24.8 + if marker_visible { 0.0 } else { 4.0 }
        ))
        .when(marker_visible, |row| {
            row.child(
                div()
                    .debug_selector(move || marker_debug_selector)
                    .w(px(18.0))
                    .flex_none()
                    .text_right()
                    .text_color(theme.muted_foreground)
                    .child(marker),
            )
        })
        .child(render_list_children(
            children, depth, block_path, document, view, cx,
        ))
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_task_item(
    checked: bool,
    depth: usize,
    children: &[DocumentBlock],
    block_path: &[usize],
    parent_list_depth: Option<usize>,
    document: &PreparedDocument,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let block_index = block_path_render_index(block_path);
    let block_suffix = block_path_suffix(block_path);
    let block_debug_selector = format!("reader-block-{block_suffix}");
    let marker_debug_selector = format!("reader-list-marker-{block_suffix}");
    let indentation_depth =
        parent_list_depth.map_or(depth, |parent_depth| depth.saturating_sub(parent_depth));
    let checkbox = div()
        .debug_selector(move || marker_debug_selector)
        .flex()
        .items_center()
        .justify_center()
        .size(px(14.0))
        .mt(px(5.0))
        .flex_none()
        .rounded(px(3.0))
        .border_1()
        .border_color(if checked { theme.primary } else { theme.border })
        .bg(if checked {
            theme.primary
        } else {
            theme.background
        })
        .when(checked, |box_element| {
            box_element.child(icon("icons/check.svg", theme.background, 10.0))
        });
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || block_debug_selector)
        .flex()
        .items_start()
        .w_full()
        .min_w_0()
        .gap(px(8.0))
        .ml(px(indentation_depth as f32 * 24.8))
        .child(checkbox)
        .child(render_list_children(
            children, depth, block_path, document, view, cx,
        ))
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_list_children(
    children: &[DocumentBlock],
    list_depth: usize,
    block_path: &[usize],
    document: &PreparedDocument,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let mut spacing = block_sequence_spacing(children);
    if let Some(first) = spacing.first_mut() {
        first.before = 0.0;
    }
    if let Some(last) = spacing.last_mut() {
        last.after = 0.0;
    }

    let mut column = div().flex().flex_col().min_w_0().flex_grow();
    for (child_index, child) in children.iter().enumerate() {
        let mut child_path = block_path.to_vec();
        child_path.push(child_index);
        let child_debug_selector = format!("reader-list-child-{}", block_path_suffix(&child_path));
        column = column.child(
            div()
                .debug_selector(move || child_debug_selector)
                .w_full()
                .min_w_0()
                .child(render_block(
                    document,
                    child,
                    &child_path,
                    Some(list_depth),
                    spacing[child_index],
                    list_marker_is_visible(children, child_index),
                    view,
                    cx,
                )),
        );
    }
    column.into_any_element()
}

/// Electron's five `--md-alert-*` colors.
pub fn alert_accent(kind: AlertKind, theme: Theme) -> gpui::Hsla {
    match kind {
        AlertKind::Note => theme.alert_note,
        AlertKind::Tip => theme.alert_tip,
        AlertKind::Important => theme.alert_important,
        AlertKind::Warning => theme.alert_warning,
        AlertKind::Caution => theme.alert_caution,
    }
}

/// Lucide icons: info, lightbulb, message-square-warning, triangle-alert, octagon-alert.
pub fn alert_icon(kind: AlertKind) -> &'static str {
    match kind {
        AlertKind::Note => "icons/info.svg",
        AlertKind::Tip => "icons/lightbulb.svg",
        AlertKind::Important => "icons/message-square-warning.svg",
        AlertKind::Warning => "icons/triangle-alert.svg",
        AlertKind::Caution => "icons/octagon-alert.svg",
    }
}

const ALERT_ICON_SIZE: f32 = 15.0;
const ALERT_ICON_GAP: f32 = 7.0;
const ALERT_BORDER_ALPHA: f32 = 0.30;
const ALERT_FILL_ALPHA: f32 = 0.07;

#[allow(clippy::too_many_arguments)]
fn render_alert(
    kind: AlertKind,
    children: &[DocumentBlock],
    block_path: &[usize],
    document: &PreparedDocument,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let block_index = block_path_render_index(block_path);
    let block_suffix = block_path_suffix(block_path);
    let accent = alert_accent(kind, theme);
    let debug_selector = format!("reader-block-{block_suffix}");
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || debug_selector)
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .border_1()
        .border_color(accent.opacity(ALERT_BORDER_ALPHA))
        .bg(accent.opacity(ALERT_FILL_ALPHA))
        .rounded(px(8.0))
        .px(px(14.0))
        .py(px(10.0))
        .gap(px(2.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(ALERT_ICON_GAP))
                .font_family(Metrics::FONT_SANS)
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(13.0))
                .line_height(px(20.0))
                .text_color(accent)
                .child(icon(alert_icon(kind), accent, ALERT_ICON_SIZE))
                .child(kind.label().to_owned()),
        )
        .child(
            // The body is reader-sized and aligns with the title text, not the icon.
            div()
                .pl(px(ALERT_ICON_SIZE + ALERT_ICON_GAP))
                .min_w_0()
                .child(render_list_children(
                    children,
                    0,
                    block_path,
                    document,
                    ReaderView {
                        find_block: None,
                        ..view
                    },
                    cx,
                )),
        )
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_footnote_section(
    notes: &[(String, Vec<DocumentBlock>)],
    block_path: &[usize],
    document: &PreparedDocument,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let block_index = block_path_render_index(block_path);
    let block_suffix = block_path_suffix(block_path);
    let debug_selector = format!("reader-block-{block_suffix}");
    // Notes read one step quieter than the body: 0.9em, muted, numbered like an ordered list.
    let note_size = view.zoom(READER_FONT_SIZE * 0.9);
    let mut list = div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .w_full()
        .min_w_0()
        .text_size(px(note_size))
        .line_height(px(note_size * READER_LINE_HEIGHT));
    for (note_index, (label, children)) in notes.iter().enumerate() {
        let mut note_path = block_path.to_vec();
        note_path.push(note_index);
        let marker = if label.bytes().all(|byte| byte.is_ascii_digit()) {
            format!("{label}.")
        } else {
            label.clone()
        };
        list = list.child(
            div()
                .flex()
                .items_start()
                .gap(px(8.0))
                .w_full()
                .min_w_0()
                .child(
                    div()
                        .flex_none()
                        .min_w(px(18.0))
                        .text_right()
                        .text_color(theme.muted_foreground)
                        .child(marker),
                )
                .child(render_list_children(
                    children,
                    0,
                    &note_path,
                    document,
                    ReaderView {
                        find_block: None,
                        muted: true,
                        ..view
                    },
                    cx,
                )),
        );
    }
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || debug_selector)
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .pt(px(16.0))
        .border_t_1()
        .border_color(theme.border)
        .gap(px(8.0))
        .child(
            div()
                .font_family(Metrics::FONT_SANS)
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.muted_foreground)
                .child("Footnotes"),
        )
        .child(list)
        .into_any_element()
}

fn format_ordered_marker(number: u64, depth: usize) -> String {
    match depth {
        0 => format!("{number}."),
        1 => format!("{}.", to_lower_alpha(number)),
        _ => format!("{}.", to_lower_roman(number)),
    }
}

fn unordered_marker(depth: usize) -> &'static str {
    match depth {
        0 => "•",
        1 => "◦",
        _ => "▪",
    }
}

fn to_lower_alpha(mut number: u64) -> String {
    if number == 0 {
        return "0".into();
    }
    let mut output = Vec::new();
    while number > 0 {
        number -= 1;
        output.push((b'a' + (number % 26) as u8) as char);
        number /= 26;
    }
    output.into_iter().rev().collect()
}

fn to_lower_roman(mut number: u64) -> String {
    let mut output = String::new();
    for (value, digits) in [
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
    ] {
        while number >= value {
            number -= value;
            output.push_str(digits);
        }
    }
    output
}

/// The code a block paints: the fence body without its final newline, which would otherwise
/// lay out as an empty last line below the code.
pub fn code_display_text(code: &str) -> &str {
    code.strip_suffix('\n')
        .map(|code| code.strip_suffix('\r').unwrap_or(code))
        .unwrap_or(code)
}

fn syntax_hsla(color: crate::syntax::SyntaxColor) -> gpui::Hsla {
    gpui::Hsla::from(gpui::rgb(
        (u32::from(color.red) << 16) | (u32::from(color.green) << 8) | u32::from(color.blue),
    ))
}

fn code_font(family: &'static str, italic: bool) -> Font {
    let mut run_font = font(family);
    run_font.weight = FontWeight::NORMAL;
    run_font.style = if italic {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };
    run_font
}

/// Highlighted runs clipped to `display_len` bytes (the painted text drops the final newline).
fn highlighted_text_runs(
    highlighted: &HighlightedCode,
    display_len: usize,
    family: &'static str,
) -> Vec<TextRun> {
    let mut remaining = display_len;
    let mut runs = Vec::new();
    for run in &highlighted.runs {
        if remaining == 0 {
            break;
        }
        let len = run.len.min(remaining);
        remaining -= len;
        runs.push(TextRun {
            len,
            font: code_font(family, run.italic),
            color: syntax_hsla(run.color),
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }
    runs
}

/// Plain code before (or without) highlighting: the same face, weight, and default syntax color
/// the highlighter uses, so swapping in highlighted runs does not shift or flash the text.
fn plain_code_runs(display_len: usize, scheme: ColorScheme, family: &'static str) -> Vec<TextRun> {
    (display_len > 0)
        .then(|| TextRun {
            len: display_len,
            font: code_font(family, false),
            color: syntax_hsla(crate::syntax::default_code_color(scheme)),
            background_color: None,
            underline: None,
            strikethrough: None,
        })
        .into_iter()
        .collect()
}

/// Asks the shared cache for this block's tokens. On the first miss the block is highlighted on a
/// background thread and the reader repaints when it lands; until then it paints plain text.
fn lazy_highlight(
    language: Option<&str>,
    code: &str,
    scheme: ColorScheme,
    cx: &Context<MdowApp>,
) -> Option<Arc<HighlightedCode>> {
    match HighlightCache::global().lookup(language, code, scheme) {
        HighlightLookup::Ready(highlighted) => Some(highlighted),
        HighlightLookup::Claimed(key) => {
            let language = language.unwrap_or_default().to_owned();
            let code = code.to_owned();
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .spawn(async move {
                        HighlightCache::global().highlight_claimed(key, &language, &code);
                    })
                    .await;
                this.update(cx, |_, cx| cx.notify()).ok();
            })
            .detach();
            None
        }
        HighlightLookup::Pending | HighlightLookup::Unsupported => None,
    }
}

/// The subtle full-width band behind a highlighted line.
fn code_line_band(theme: Theme) -> gpui::Hsla {
    theme.foreground.opacity(match theme.color_scheme {
        ColorScheme::Light => 0.06,
        ColorScheme::Dark => 0.07,
    })
}

/// The Copy button in a code card header; `copied` shows the confirmation state.
fn code_copy_button(
    code_to_copy: String,
    block_path: &[usize],
    copied: bool,
    theme: Theme,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let block_index = block_path_render_index(block_path);
    let block_suffix = block_path_suffix(block_path);
    let copy_debug_selector = format!("copy-code-{block_suffix}");
    let copied_debug_selector = format!("copied-code-{block_suffix}");
    let copy_color = if copied {
        theme.alert_tip
    } else {
        theme.muted_foreground
    };
    div()
        .id(("copy-code", block_index))
        .debug_selector(move || copy_debug_selector)
        .tab_index(0)
        .focusable()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(CODE_COPY_BUTTON_HEIGHT))
        .px(px(8.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(gpui::transparent_black())
        .font_family(Metrics::FONT_SANS)
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(11.0))
        .line_height(px(16.0))
        .text_color(copy_color)
        .cursor_pointer()
        .hover(move |style| {
            style
                .bg(theme.foreground.opacity(0.06))
                .text_color(if copied { copy_color } else { theme.foreground })
        })
        .active(|style| style.opacity(0.78))
        .focus(move |style| style.border_color(theme.primary))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.copy_code(block_index, code_to_copy.clone(), cx);
        }))
        .child(icon(
            if copied {
                "icons/check.svg"
            } else {
                "icons/copy.svg"
            },
            copy_color,
            13.0,
        ))
        .child(if copied {
            div()
                .id(("copied-code", block_index))
                .debug_selector(move || copied_debug_selector)
                .child("Copied")
                .into_any_element()
        } else {
            div().child("Copy").into_any_element()
        })
        .into_any_element()
}

/// The 32px header shared by code blocks and diagrams: a mono label left, actions right.
fn code_card_header(label: Option<String>, action: AnyElement, theme: Theme) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(CODE_HEADER_HEIGHT))
        .pl(px(14.0))
        .pr(px(6.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .font_family(Metrics::FONT_MONO)
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(11.0))
                .line_height(px(16.0))
                .text_color(theme.muted_foreground)
                .children(label),
        )
        .child(action)
}

#[allow(clippy::too_many_arguments)]
fn render_code_block(
    language: Option<&str>,
    code: &str,
    highlights: &LineHighlights,
    highlight: bool,
    block_path: &[usize],
    document_path: &Path,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let block_index = block_path_render_index(block_path);
    let block_suffix = block_path_suffix(block_path);
    let copied = code_copy_feedback_is_active(view.copied_code, block_index, Instant::now());
    let code_to_copy = code.to_owned();
    let display = code_display_text(code);
    let family = view.style.code_family;
    let highlighted = highlight
        .then(|| lazy_highlight(language, code, theme.color_scheme, cx))
        .flatten();
    let runs = match &highlighted {
        Some(highlighted) => highlighted_text_runs(highlighted, display.len(), family),
        None => plain_code_runs(display.len(), theme.color_scheme, family),
    };
    let code_text = StyledText::new(display.to_owned()).with_runs(runs);
    let code_size = view.zoom(READER_FONT_SIZE * 0.875);
    let code_line_height = code_size * BlockStyle::code_block().line_height;
    let [padding_top, padding_x] = BlockStyle::code_block().padding;

    // Blocks without a fence language keep the header (with only the Copy button) so every code
    // block shares one geometry and the button never moves; a made-up "text" label would claim
    // a language the author never wrote.
    let header = code_card_header(
        language.map(str::to_lowercase),
        code_copy_button(code_to_copy, block_path, copied, theme, cx),
        theme,
    );

    let band = code_line_band(theme);
    let line_count = display.split('\n').count();
    let bands = (1..=line_count)
        .filter(|line| highlights.contains(*line))
        .map(|line| {
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(padding_top + (line - 1) as f32 * code_line_height))
                .h(px(code_line_height))
                .bg(band)
        })
        .collect::<Vec<_>>();

    let block_debug_selector = format!("reader-block-{block_suffix}");
    let code_debug_selector = format!("reader-code-{block_suffix}");
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || block_debug_selector)
        .w_full()
        .min_w_0()
        .rounded(px(BlockStyle::code_block().radius))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.code_surface)
        .overflow_hidden()
        .child(header)
        .child(
            restrict_scroll_to_axis(div())
                .id((
                    "code-scroll",
                    document_scoped_element_id(document_path, "code-scroll", block_index),
                ))
                .debug_selector(move || code_debug_selector)
                .flex()
                .w_full()
                .overflow_x_scroll()
                .scrollbar_width(px(6.0))
                .child(
                    // At least as wide as the block and as wide as the longest line, so
                    // highlight bands span the whole scrollable width.
                    div()
                        .relative()
                        .flex_none()
                        .min_w_full()
                        .pt(px(padding_top))
                        .pb(px(CODE_PADDING_BOTTOM))
                        .px(px(padding_x))
                        .font_family(family)
                        .font_weight(FontWeight::NORMAL)
                        .text_size(px(code_size))
                        .line_height(px(code_line_height))
                        .whitespace_nowrap()
                        .children(bands)
                        .child(code_text),
                ),
        )
        .into_any_element()
}

fn align_table_cell<E: Styled>(cell: E, alignment: Alignment) -> E {
    match alignment {
        Alignment::None => cell,
        Alignment::Left => cell.text_left(),
        Alignment::Center => cell.text_center(),
        Alignment::Right => cell.text_right(),
    }
}

fn render_table(
    table: &TableBlock,
    block_index: usize,
    document_path: &Path,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let link_state = view.link_state;
    let column_count = table
        .headers
        .len()
        .max(table.rows.iter().map(Vec::len).max().unwrap_or(0))
        .max(1);
    let mut grid = div()
        .grid()
        .grid_cols(column_count as u16)
        .min_w(px(column_count as f32 * 140.0))
        .font_family(view.style.content_family)
        .text_size(px(view.zoom(16.0 * 0.925)))
        .line_height(px(view.zoom(16.0 * 0.925) * 1.5));
    for column_index in 0..column_count {
        let content = table.headers.get(column_index).cloned().unwrap_or_default();
        let surface = LinkSurfaceKey::table_header(block_index, column_index);
        grid = grid.child(
            div()
                .map(|cell| align_table_cell(cell, table.alignment(column_index)))
                .min_w_0()
                .px(px(14.0))
                .py(px(10.0))
                .bg(theme.muted)
                .border_b_1()
                .border_color(theme.border)
                .when(column_index + 1 < column_count, |cell| {
                    cell.border_r_1().border_color(theme.border_subtle)
                })
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(view.zoom(14.0)))
                .line_height(px(view.zoom(16.0 * 0.925 * 1.3)))
                .text_color(theme.muted_foreground)
                .child(render_inline_layout(
                    inline_layout(&content),
                    document_path,
                    surface,
                    600,
                    theme.muted_foreground,
                    theme,
                    view.style,
                    true,
                    link_state,
                    cx,
                )),
        );
    }
    for (row_index, row) in table.rows.iter().enumerate() {
        for column_index in 0..column_count {
            let content = row.get(column_index).cloned().unwrap_or_default();
            let surface = LinkSurfaceKey::table_cell(block_index, row_index, column_index);
            let last_row = row_index + 1 == table.rows.len();
            grid = grid.child(
                div()
                    .map(|cell| align_table_cell(cell, table.alignment(column_index)))
                    .min_w_0()
                    .px(px(14.0))
                    .py(px(10.0))
                    .when(!last_row, |cell| {
                        cell.border_b_1().border_color(theme.border_subtle)
                    })
                    .when(column_index + 1 < column_count, |cell| {
                        cell.border_r_1().border_color(theme.border_subtle)
                    })
                    .child(render_inline(
                        &content,
                        document_path,
                        surface,
                        400,
                        theme.foreground,
                        theme,
                        view.style,
                        link_state,
                        cx,
                    )),
            );
        }
    }
    div()
        .id((
            "table-scroll",
            document_scoped_element_id(document_path, "table-scroll", block_index),
        ))
        .debug_selector(move || format!("reader-block-{block_index}"))
        .w_full()
        .rounded(px(8.0))
        .border_1()
        .border_color(theme.border)
        .overflow_hidden()
        .overflow_x_scroll()
        .map(restrict_scroll_to_axis)
        .scrollbar_width(px(6.0))
        .child(grid)
        .into_any_element()
}

fn render_image(
    alt: &str,
    source: &str,
    block_index: usize,
    document_path: &Path,
    theme: Theme,
) -> AnyElement {
    let alt_owned = alt.to_owned();
    let fallback = move || image_fallback(alt_owned.clone(), theme, block_index);
    let is_data = source
        .trim_start()
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"));
    let image = if is_data {
        data_image(source).map(|image| img(image))
    } else {
        resolve_image_target(document_path, source).map(|path| img(Arc::<Path>::from(path)))
    };
    let content = match image {
        Some(image) => style_reader_image(image)
            .with_loading({
                let fallback = fallback.clone();
                move || fallback()
            })
            .with_fallback(fallback)
            .into_any_element(),
        // Remote images are blocked like Electron's CSP; unresolvable ones say so.
        None => fallback(),
    };
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || format!("reader-block-{block_index}"))
        .w_full()
        .child(content)
        .into_any_element()
}

fn image_fallback(alt: String, theme: Theme, block_index: usize) -> AnyElement {
    div()
        .id(("image-fallback", block_index))
        .debug_selector(move || format!("image-fallback-{block_index}"))
        .flex()
        .items_center()
        .justify_center()
        .w_full()
        .min_h(px(96.0))
        .px(px(18.0))
        .py(px(16.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        .font_family(Metrics::FONT_SANS)
        .text_size(px(13.0))
        .text_color(theme.muted_foreground)
        .child(if alt.is_empty() {
            "Image unavailable".to_owned()
        } else {
            alt
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{InlineSpan, parse_document};
    use crate::prefs::{CodeFont, ContentFont, PrefEdit, Prefs};
    use crate::syntax::highlight_code;
    use std::time::{Duration, Instant};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    fn paragraph_children(text: &str) -> Vec<DocumentBlock> {
        vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
            text.into(),
        )])]
    }

    #[test]
    fn reader_surface_metrics_match_markdown_css() {
        // Electron typography.ts: MARKDOWN_FONT_SIZE 15.5, MARKDOWN_LINE_HEIGHT 1.65.
        assert_eq!(BlockStyle::body().font_size, 15.5);
        assert_eq!(BlockStyle::body().line_height, 1.65);
        assert_eq!(BlockStyle::heading(1).font_size, 15.5 * 1.875);
        assert_eq!(BlockStyle::heading(2).margin_top_em, 1.8);
        assert_eq!(BlockStyle::blockquote().padding, [15.5 * 0.4, 15.5]);
        assert_eq!(BlockStyle::code_block().radius, 8.0);
        assert_eq!(BlockStyle::code_block().padding, [12.0, 16.0]);
        assert_eq!(CODE_PADDING_BOTTOM, 14.0);
        assert_eq!(CODE_HEADER_HEIGHT, 32.0);
        assert_eq!(CODE_COPY_BUTTON_HEIGHT, 24.0);
        assert_eq!(BlockStyle::table_cell().padding, [10.0, 14.0]);
    }

    #[test]
    fn code_blocks_paint_without_a_trailing_empty_line() {
        assert_eq!(code_display_text("let a = 1;\n"), "let a = 1;");
        assert_eq!(code_display_text("let a = 1;\r\n"), "let a = 1;");
        assert_eq!(code_display_text("a\n\n"), "a\n");
        assert_eq!(code_display_text("no newline"), "no newline");

        let highlighted = highlight_code(Some("rust"), "fn main() {}\n", ColorScheme::Dark);
        let display = code_display_text(&highlighted.text);
        let runs = highlighted_text_runs(&highlighted, display.len(), Metrics::FONT_MONO);
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), display.len());
    }

    #[test]
    fn plain_code_matches_highlighted_metrics_to_avoid_a_flash() {
        let plain = plain_code_runs(5, ColorScheme::Light, Metrics::FONT_MONO);
        let highlighted = highlight_code(Some("rust"), "let a", ColorScheme::Light);
        let runs = highlighted_text_runs(&highlighted, 5, Metrics::FONT_MONO);
        assert_eq!(plain[0].font.family, runs[0].font.family);
        assert_eq!(plain[0].font.weight, runs[0].font.weight);
        assert_eq!(
            plain_code_runs(0, ColorScheme::Light, Metrics::FONT_MONO).len(),
            0
        );
    }

    #[test]
    fn alerts_use_five_distinct_electron_colors_and_lucide_icons() {
        for theme in [
            Theme::for_appearance(gpui::WindowAppearance::Light),
            Theme::for_appearance(gpui::WindowAppearance::Dark),
        ] {
            let kinds = [
                AlertKind::Note,
                AlertKind::Tip,
                AlertKind::Important,
                AlertKind::Warning,
                AlertKind::Caution,
            ];
            let colors = kinds.map(|kind| alert_accent(kind, theme));
            for (index, color) in colors.iter().enumerate() {
                assert!(
                    colors[index + 1..].iter().all(|other| other != color),
                    "alert colors must be distinct"
                );
            }
            let icons = kinds.map(alert_icon);
            assert_eq!(
                icons,
                [
                    "icons/info.svg",
                    "icons/lightbulb.svg",
                    "icons/message-square-warning.svg",
                    "icons/triangle-alert.svg",
                    "icons/octagon-alert.svg",
                ]
            );
            for icon in icons {
                assert!(
                    crate::assets::required_assets().any(|asset| asset == icon),
                    "{icon} must be a required asset"
                );
            }
        }
        let light = Theme::for_appearance(gpui::WindowAppearance::Light);
        let dark = Theme::for_appearance(gpui::WindowAppearance::Dark);
        // oklch(0.55 0.17 255) = #1570d1; oklch(0.68 0.14 255) = #589aed.
        let hex = |color: gpui::Hsla| {
            let rgba = gpui::Rgba::from(color);
            let byte = |channel: f32| (channel * 255.0).round() as u8;
            format!(
                "#{:02x}{:02x}{:02x}",
                byte(rgba.r),
                byte(rgba.g),
                byte(rgba.b)
            )
        };
        assert_eq!(hex(light.alert_note), "#1570d1");
        assert_eq!(hex(dark.alert_note), "#589aed");
        assert_eq!(hex(light.alert_caution), "#cc3336");
        assert_eq!(hex(dark.alert_tip), "#56ae6c");
        assert_eq!(hex(light.alert_warning), "#ad7300");
    }

    #[test]
    fn reader_scrollbar_geometry_tracks_viewport_extent_and_offset() {
        let top = reader_scrollbar_geometry(600.0, 1_400.0, 0.0).expect("overflow thumb");
        let middle = reader_scrollbar_geometry(600.0, 1_400.0, -700.0).expect("overflow thumb");
        let bottom = reader_scrollbar_geometry(600.0, 1_400.0, -1_400.0).expect("overflow thumb");

        assert!((top.thumb_height - 177.6).abs() < 0.001);
        assert_eq!(top.thumb_top, 4.0);
        assert!((middle.thumb_top - 211.2).abs() < 0.001);
        assert!((bottom.thumb_top - 418.4).abs() < 0.001);
        assert!(reader_scrollbar_geometry(600.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn reader_scrollbar_pointer_targets_clamp_to_the_scroll_extent() {
        let geometry = reader_scrollbar_geometry(600.0, 1_400.0, 0.0).expect("overflow thumb");

        assert_eq!(
            reader_scrollbar_offset_for_pointer(-100.0, 20.0, geometry),
            0.0
        );
        assert!((reader_scrollbar_offset_for_pointer(300.0, 88.8, geometry) + 700.0).abs() < 0.001);
        assert_eq!(
            reader_scrollbar_offset_for_pointer(900.0, 20.0, geometry),
            -1_400.0,
        );
    }

    #[test]
    fn highlighted_runs_keep_lengths_fonts_and_theme_colors() {
        let code = "fn main() {}\n";
        let light_code = highlight_code(Some("rust"), code, ColorScheme::Light);
        let dark_code = highlight_code(Some("rust"), code, ColorScheme::Dark);
        let light = highlighted_text_runs(&light_code, code.len(), Metrics::FONT_MONO);
        let dark = highlighted_text_runs(&dark_code, code.len(), Metrics::FONT_MONO);

        assert_eq!(light.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert_eq!(dark.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert!(
            light
                .iter()
                .all(|run| run.font.family.as_ref() == Metrics::FONT_MONO)
        );
        assert_ne!(light[0].color, dark[0].color);
    }

    #[test]
    fn heading_styles_preserve_the_complete_six_level_hierarchy() {
        // Electron markdown.css: h4/h5 use the foreground; only h6 is muted and uppercase.
        let expected = [
            (15.5 * 1.875, 700, 1.2, -0.025, 2.0, 0.6, false, false),
            (15.5 * 1.5, 650, 1.25, -0.02, 1.8, 0.5, false, false),
            (15.5 * 1.15, 600, 1.3, -0.01, 1.5, 0.4, false, false),
            (15.5, 600, 1.4, 0.0, 1.3, 0.3, false, false),
            (15.5 * 0.95, 600, 1.4, 0.0, 1.2, 0.25, false, false),
            (15.5 * 0.875, 600, 1.4, 0.03, 1.0, 0.2, true, true),
        ];

        for (level, expected) in (1_u8..=6).zip(expected) {
            let style = BlockStyle::heading(level);
            assert!((style.font_size - expected.0).abs() < 0.0001);
            assert_eq!(
                (
                    style.font_weight,
                    style.line_height,
                    style.letter_spacing_em,
                    style.margin_top_em,
                    style.margin_bottom_em,
                    style.muted,
                    style.uppercase,
                ),
                (
                    expected.1, expected.2, expected.3, expected.4, expected.5, expected.6,
                    expected.7,
                ),
            );
        }
    }

    #[test]
    fn inline_layout_keeps_plain_text_and_nested_style_ranges_on_one_surface() {
        let layout = inline_layout(&[
            InlineSpan::Text("A ".into()),
            InlineSpan::Emphasis(vec![
                InlineSpan::Text("quiet".into()),
                InlineSpan::Strong(vec![InlineSpan::Text(" reader".into())]),
            ]),
            InlineSpan::Text(" uses ".into()),
            InlineSpan::Code("mdow".into()),
            InlineSpan::Text(" at ".into()),
            InlineSpan::Link {
                label: vec![InlineSpan::Text("home".into())],
                target: "guide.md".into(),
            },
            InlineSpan::SoftBreak,
            InlineSpan::Text("today".into()),
            InlineSpan::HardBreak,
            InlineSpan::Text("next".into()),
        ]);

        assert_eq!(layout.text, "A quiet reader uses mdow at home\ntoday\nnext");
        assert_eq!(
            layout.styles,
            vec![
                InlineStyleRange::emphasis(2..7),
                InlineStyleRange::emphasis_strong(7..14),
                InlineStyleRange::code(20..24),
                InlineStyleRange::link(28..32, "guide.md"),
            ],
        );
        assert_eq!(
            layout.links,
            vec![InlineLink {
                range: 28..32,
                target: "guide.md".into(),
                node_id: 0,
            }],
        );
    }

    #[test]
    fn inline_math_reserves_placeholders_and_shifts_later_ranges() {
        let spans = vec![
            InlineSpan::Text("a ".into()),
            InlineSpan::Math {
                tex: "x".into(),
                display: false,
            },
            InlineSpan::Text(" ".into()),
            InlineSpan::Link {
                label: vec![InlineSpan::Text("b".into())],
                target: "guide.md".into(),
            },
            InlineSpan::Text(" ".into()),
            InlineSpan::Math {
                tex: "bad".into(),
                display: false,
            },
        ];
        let mut layout = inline_layout(&spans);
        assert_eq!(layout.text, "a $x$ b $bad$");
        assert_eq!(layout.math.len(), 2);

        let reserved = reserve_inline_math(&mut layout, 0.25, |tex, _| (tex == "x").then_some(0.9));

        assert_eq!(layout.text, "a .... b $bad$");
        assert_eq!(
            reserved,
            vec![InlineMath {
                range: 2..6,
                tex: "x".into(),
                display: false,
            }]
        );
        assert_eq!(layout.links[0].range, 7..8);
        let placeholder = layout
            .styles
            .iter()
            .find(|style| style.range == (2..6))
            .unwrap();
        assert!(placeholder.math && !placeholder.code);
        let fallback = layout
            .styles
            .iter()
            .find(|style| style.range == (9..14))
            .unwrap();
        assert!(
            fallback.code && !fallback.math,
            "untypeset math stays code text"
        );
    }

    #[test]
    fn inline_math_inside_a_link_extends_the_link_range() {
        let mut layout = inline_layout(&[InlineSpan::Link {
            label: vec![
                InlineSpan::Text("see ".into()),
                InlineSpan::Math {
                    tex: r"\lambda".into(),
                    display: false,
                },
            ],
            target: "#display-math".into(),
        }]);

        reserve_inline_math(&mut layout, 0.5, |_, _| Some(1.0));

        assert_eq!(layout.text, "see ..");
        assert_eq!(layout.links[0].range, 0..6);
    }

    #[test]
    fn soft_breaks_paint_as_line_breaks_that_find_text_matches_byte_for_byte() {
        let spans = vec![
            InlineSpan::Text("hello".into()),
            InlineSpan::SoftBreak,
            InlineSpan::Emphasis(vec![InlineSpan::Text("wide".into())]),
            InlineSpan::SoftBreak,
            InlineSpan::Text("world".into()),
        ];
        let painted = inline_layout(&spans).text;
        let searchable = DocumentBlock::Paragraph(spans).find_text();

        assert_eq!(painted, "hello\nwide\nworld");
        assert_eq!(searchable, "hello wide world");
        assert_eq!(painted.len(), searchable.len());
    }

    #[test]
    fn strikethrough_and_footnote_refs_style_the_painted_layout() {
        let theme = Theme::for_appearance(gpui::WindowAppearance::Dark);
        let layout = inline_layout(&[
            InlineSpan::Strikethrough(vec![InlineSpan::Text("gone".into())]),
            InlineSpan::Text(" ".into()),
            InlineSpan::FootnoteRef { label: "1".into() },
        ]);

        assert_eq!(layout.text, "gone ¹");
        assert_eq!(
            layout.styles,
            vec![
                InlineStyleRange::strikethrough(0..4),
                InlineStyleRange {
                    range: 5..7,
                    footnote: true,
                    link_target: Some("#fn-1".into()),
                    link_node_id: Some(0),
                    ..InlineStyleRange::default()
                },
            ],
        );
        assert_eq!(
            layout.links,
            vec![InlineLink {
                range: 5..7,
                target: "#fn-1".into(),
                node_id: 0,
            }],
            "footnote refs are clickable links to their note"
        );

        let runs = text_runs(
            &layout,
            &layout.links,
            None,
            None,
            400,
            theme.foreground,
            theme,
            Prefs::default().reader_style(),
            false,
        );
        assert!(runs[0].strikethrough.is_some());
        assert!(runs[1].strikethrough.is_none());
        assert_eq!(runs[2].color, theme.primary);
    }

    #[test]
    fn footnote_links_route_to_anchor_jumps_in_both_directions() {
        let document = parse_document(
            PathBuf::from("/tmp/notes.md"),
            "Claim.[^a]\n\n[^a]: Note.\n".into(),
        );
        let targets = document_link_focus_targets(&document)
            .into_iter()
            .map(|target| target.target)
            .collect::<Vec<_>>();
        assert_eq!(targets, vec!["#fn-a", "#fnref-a"]);
        for target in &targets {
            let LinkRoute::Anchor(fragment) = classify_link(&document.path, target) else {
                panic!("{target} should be an in-document anchor");
            };
            assert!(document.anchor_block(&fragment).is_some(), "{target}");
        }
        assert_eq!(document.anchor_block("fn-a"), Some(1));
        assert_eq!(document.anchor_block("fnref-a"), Some(0));
    }

    #[test]
    fn inline_html_subset_styles_runs_and_keeps_find_text_aligned() {
        let theme = Theme::for_appearance(gpui::WindowAppearance::Light);
        let spans = vec![
            InlineSpan::Kbd(vec![InlineSpan::Text("K".into())]),
            InlineSpan::Text(" x".into()),
            InlineSpan::Superscript(vec![InlineSpan::Text("2".into())]),
            InlineSpan::Text(" ".into()),
            InlineSpan::Superscript(vec![InlineSpan::Text("th".into())]),
            InlineSpan::Text(" ".into()),
            InlineSpan::Mark(vec![InlineSpan::Text("hot".into())]),
        ];
        let layout = inline_layout(&spans);
        assert_eq!(layout.text, "K x² th hot");
        assert_eq!(layout.text, DocumentBlock::Paragraph(spans).find_text());

        let reader = Prefs::default().reader_style();
        let runs = text_runs(
            &layout,
            &[],
            None,
            None,
            400,
            theme.foreground,
            theme,
            reader,
            false,
        );
        let kbd = &runs[0];
        assert_eq!(kbd.font.family.as_ref(), reader.code_family);
        assert_eq!(kbd.background_color, Some(theme.border_subtle));
        let superscript = &runs[2];
        assert!(
            superscript
                .font
                .features
                .tag_value_list()
                .contains(&("sups".to_owned(), 1))
        );
        let mark = runs.last().unwrap();
        assert!(mark.background_color.is_some());
    }

    #[test]
    fn adjacent_same_target_links_keep_distinct_source_identity() {
        let layout = inline_layout(&[
            InlineSpan::Link {
                label: vec![InlineSpan::Text("one".into())],
                target: "same.md".into(),
            },
            InlineSpan::Link {
                label: vec![InlineSpan::Text("two".into())],
                target: "same.md".into(),
            },
        ]);

        assert_eq!(layout.text, "onetwo");
        assert_eq!(layout.links.len(), 2);
        assert_eq!(layout.links[0].range, 0..3);
        assert_eq!(layout.links[1].range, 3..6);
        assert_ne!(layout.links[0].node_id, layout.links[1].node_id);
    }

    #[test]
    fn active_heading_is_the_first_heading_at_or_above_the_top_block() {
        let heading_blocks = [2, 5, 5, 9];

        assert_eq!(active_heading_for(&heading_blocks, 0), None);
        assert_eq!(active_heading_for(&heading_blocks, 2), Some(0));
        assert_eq!(active_heading_for(&heading_blocks, 4), Some(0));
        assert_eq!(active_heading_for(&heading_blocks, 5), Some(1));
        assert_eq!(active_heading_for(&heading_blocks, 8), Some(1));
        assert_eq!(active_heading_for(&heading_blocks, 40), Some(3));
        assert_eq!(active_heading_for(&[], 3), None);
    }

    #[test]
    fn table_cells_apply_their_column_alignment() {
        let text_align = |alignment| {
            align_table_cell(div(), alignment)
                .style()
                .text
                .as_ref()
                .and_then(|text| text.text_align)
        };

        assert_eq!(text_align(Alignment::None), None);
        assert_eq!(text_align(Alignment::Left), Some(gpui::TextAlign::Left));
        assert_eq!(text_align(Alignment::Center), Some(gpui::TextAlign::Center));
        assert_eq!(text_align(Alignment::Right), Some(gpui::TextAlign::Right));
    }

    #[test]
    fn nested_horizontal_scrollers_restrict_plain_wheel_events_to_the_vertical_axis() {
        let mut scroller = restrict_scroll_to_axis(div());

        assert_eq!(scroller.style().restrict_scroll_to_axis, Some(true));
    }

    #[test]
    fn horizontal_scroll_ids_include_document_identity() {
        assert_ne!(
            document_scoped_element_id(Path::new("/tmp/one.md"), "code-scroll", 4),
            document_scoped_element_id(Path::new("/tmp/two.md"), "code-scroll", 4),
        );
        assert_eq!(
            document_scoped_element_id(Path::new("/tmp/one.md"), "code-scroll", 4),
            document_scoped_element_id(Path::new("/tmp/one.md"), "code-scroll", 4),
        );
    }

    #[test]
    fn nested_list_markers_stop_cycling_and_ordered_alpha_extends_past_z() {
        assert_eq!(unordered_marker(0), "•");
        assert_eq!(unordered_marker(1), "◦");
        assert_eq!(unordered_marker(2), "▪");
        assert_eq!(unordered_marker(5), "▪");
        assert_eq!(format_ordered_marker(27, 1), "aa.");
        assert_eq!(format_ordered_marker(52, 1), "az.");
        assert_eq!(format_ordered_marker(53, 1), "ba.");
        assert_eq!(format_ordered_marker(9, 2), "ix.");
        assert_eq!(format_ordered_marker(9, 8), "ix.");
        assert_eq!(format_ordered_marker(4_000, 2), "mmmm.");
    }

    #[test]
    fn block_sequence_spacing_collapses_adjacent_margins_and_groups_lists() {
        let blocks = vec![
            DocumentBlock::Heading {
                level: 1,
                content: vec![InlineSpan::Text("Title".into())],
            },
            DocumentBlock::CodeBlock {
                language: None,
                code: "one".into(),
                highlights: LineHighlights::default(),
            },
            DocumentBlock::Table(TableBlock {
                headers: vec![],
                rows: vec![],
                alignments: vec![],
            }),
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("one"),
            },
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("two"),
            },
        ];

        let spacing = block_sequence_spacing(&blocks);

        assert_eq!(
            spacing[0].before, 0.0,
            "a leading heading has no redundant top margin"
        );
        assert_eq!(spacing[1].before, READER_FONT_SIZE * 1.25);
        assert_eq!(
            spacing[2].before,
            READER_FONT_SIZE * 1.25,
            "code/table margins collapse to max"
        );
        assert_eq!(spacing[3].before, READER_FONT_SIZE * 1.25);
        assert_eq!(
            spacing[4].before,
            READER_FONT_SIZE * 0.35,
            "adjacent items use the CSS li + li margin"
        );
        assert_eq!(
            spacing[4].after, READER_FONT_SIZE,
            "the list group retains a 1em outer margin"
        );

        let first_h2 = block_sequence_spacing(&[DocumentBlock::Heading {
            level: 2,
            content: vec![InlineSpan::Text("Section".into())],
        }]);
        assert_eq!(first_h2[0].before, 0.0);
    }

    #[test]
    fn block_sequence_spacing_separates_unordered_and_ordered_list_groups() {
        let blocks = vec![
            DocumentBlock::TaskItem {
                checked: true,
                depth: 0,
                children: paragraph_children("task"),
            },
            DocumentBlock::ListItem {
                kind: ListKind::Ordered { number: 1 },
                depth: 0,
                children: paragraph_children("ordered"),
            },
        ];

        let spacing = block_sequence_spacing(&blocks);

        assert_eq!(spacing[1].before, READER_FONT_SIZE);
    }

    #[test]
    fn adjacent_list_items_use_the_css_li_plus_li_margin() {
        let blocks = vec![
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("first"),
            },
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("second"),
            },
        ];

        assert_eq!(
            block_sequence_spacing(&blocks)[1].before,
            READER_FONT_SIZE * 0.35
        );
    }

    #[test]
    fn mixed_task_list_groups_suppress_unordered_markers() {
        let mixed_group = vec![
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("plain"),
            },
            DocumentBlock::TaskItem {
                checked: false,
                depth: 0,
                children: paragraph_children("task"),
            },
        ];
        let plain_group = vec![DocumentBlock::ListItem {
            kind: ListKind::Unordered,
            depth: 0,
            children: paragraph_children("plain"),
        }];

        assert!(!list_marker_is_visible(&mixed_group, 0));
        assert!(list_marker_is_visible(&plain_group, 0));
    }

    #[test]
    fn task_marker_suppression_is_scoped_to_the_same_depth() {
        let parent_then_nested_task = vec![
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("parent"),
            },
            DocumentBlock::TaskItem {
                checked: false,
                depth: 1,
                children: paragraph_children("nested task"),
            },
        ];
        let same_depth_task = vec![
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("plain"),
            },
            DocumentBlock::TaskItem {
                checked: false,
                depth: 0,
                children: paragraph_children("peer task"),
            },
        ];

        assert!(list_marker_is_visible(&parent_then_nested_task, 0));
        assert!(!list_marker_is_visible(&same_depth_task, 0));
    }

    #[test]
    fn parent_and_nested_list_items_do_not_share_adjacent_item_spacing() {
        let nested_boundaries = vec![
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("parent"),
            },
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 1,
                children: paragraph_children("nested"),
            },
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("parent peer"),
            },
        ];
        let same_depth = vec![
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("first"),
            },
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                children: paragraph_children("second"),
            },
        ];

        let nested_spacing = block_sequence_spacing(&nested_boundaries);
        assert_eq!(nested_spacing[1].before, READER_FONT_SIZE);
        assert_eq!(nested_spacing[2].before, READER_FONT_SIZE);
        assert_eq!(
            block_sequence_spacing(&same_depth)[1].before,
            READER_FONT_SIZE * 0.35
        );
    }

    #[test]
    fn image_style_preserves_intrinsic_width_with_a_full_width_cap() {
        let mut image = style_reader_image(img(Arc::<Path>::from(PathBuf::from("tiny.png"))));

        assert!(image.style().size.width.is_none());
        assert_eq!(
            image.style().max_size.width,
            Some(gpui::relative(1.0).into())
        );
    }

    #[test]
    fn link_routes_distinguish_mdow_documents_web_urls_and_local_files() {
        let document = Path::new("/vault/guides/start.md");

        assert_eq!(
            classify_link(document, "next.MDX#details"),
            LinkRoute::Markdown(PathBuf::from("/vault/guides/next.MDX")),
        );
        assert_eq!(
            classify_link(document, "../images/hero.png"),
            LinkRoute::Local(PathBuf::from("/vault/images/hero.png")),
        );
        assert_eq!(
            classify_link(document, "chapter%20one.md?mode=reader#details"),
            LinkRoute::Markdown(PathBuf::from("/vault/guides/chapter one.md")),
        );
        assert_eq!(
            classify_link(document, "https://mdow.dev/docs"),
            LinkRoute::Web("https://mdow.dev/docs".into()),
        );
        assert_eq!(
            classify_link(document, "#details"),
            LinkRoute::Anchor("details".into())
        );
        assert_eq!(
            classify_link(document, "javascript:alert(1)"),
            LinkRoute::Inert
        );
    }

    #[test]
    fn non_web_uri_schemes_stay_inert_like_the_electron_shell() {
        // Electron's main process only hands http(s) URLs to shell.openExternal
        // (isAllowedExternalUrl), so other schemes are deliberately not opened.
        let document = Path::new("/vault/guides/start.md");

        for target in [
            "mailto:hello@mdow.dev",
            "tel:+15555550100",
            "file:///etc/hosts",
            "javascript:alert(1)",
            "data:text/html,<p>hi</p>",
            "vscode://file/tmp/a.md",
        ] {
            assert_eq!(
                classify_link(document, target),
                LinkRoute::Inert,
                "{target}"
            );
        }
    }

    #[test]
    fn image_sources_classify_like_electrons_csp() {
        let directory = tempfile::tempdir().unwrap();
        let document = directory.path().join("guide.md");
        let image = directory.path().join("hero.png");
        fs::write(&image, b"png bytes").unwrap();

        assert_eq!(
            classify_image(&document, "hero.png"),
            ImageTarget::Local(image)
        );
        for remote in [
            "https://mdow.dev/hero.png",
            "HTTP://mdow.dev/hero.png",
            "//cdn.example/x.png",
        ] {
            assert_eq!(
                classify_image(&document, remote),
                ImageTarget::Remote,
                "{remote}"
            );
        }
        assert_eq!(
            classify_image(&document, "data:image/png;base64,aGVsbG8="),
            ImageTarget::Data(gpui::ImageFormat::Png, b"hello".to_vec())
        );
        assert_eq!(
            classify_image(&document, "data:image/svg+xml,%3Csvg%2F%3E"),
            ImageTarget::Data(gpui::ImageFormat::Svg, b"<svg/>".to_vec())
        );
        assert_eq!(
            classify_image(&document, "data:text/html;base64,aGVsbG8="),
            ImageTarget::Unavailable,
            "non-image data URIs never render"
        );
        assert_eq!(
            classify_image(&document, "data:image/png;base64,!!!"),
            ImageTarget::Unavailable
        );
        assert_eq!(
            classify_image(&document, "missing.png"),
            ImageTarget::Unavailable
        );
        assert!(data_image("data:image/png;base64,aGVsbG8=").is_some());
        assert!(
            decode_data_image(&format!(
                "data:image/png;base64,{}",
                "A".repeat(MAX_DATA_IMAGE_BYTES / 3 * 4 + 8)
            ))
            .is_none(),
            "oversized data URIs are rejected"
        );
    }

    #[test]
    fn highlighted_code_lines_follow_the_fence_meta() {
        let highlights = LineHighlights(vec![(1, 1), (3, 4)]);
        let lines = (1..=5)
            .filter(|line| highlights.contains(*line))
            .collect::<Vec<_>>();
        assert_eq!(lines, vec![1, 3, 4]);
        let light = Theme::for_appearance(gpui::WindowAppearance::Light);
        assert!(code_line_band(light).a > 0.0 && code_line_band(light).a < 0.1);
    }

    #[test]
    fn blockquote_border_mixes_the_muted_foreground_like_electron() {
        let theme = Theme::for_appearance(gpui::WindowAppearance::Light);
        let border = blockquote_border(theme);
        assert_eq!(border.h, theme.muted_foreground.h);
        assert!((border.a - 0.45).abs() < f32::EPSILON);
    }

    #[test]
    fn image_resolution_uses_local_supported_files_and_falls_back_for_failures() {
        let directory = tempfile::tempdir().unwrap();
        let document = directory.path().join("guide.md");
        let image = directory.path().join("images/hero.PNG");
        let encoded_image = directory.path().join("images/hero shot.PNG");
        fs::create_dir(image.parent().unwrap()).unwrap();
        fs::write(&image, b"not decoded by this pure path test").unwrap();
        fs::write(&encoded_image, b"not decoded by this pure path test").unwrap();

        assert_eq!(
            resolve_image_target(&document, "images/hero.PNG"),
            Some(image),
        );
        assert_eq!(
            resolve_image_target(&document, "images/hero%20shot.PNG?raw=1#preview"),
            Some(encoded_image),
        );
        assert_eq!(resolve_image_target(&document, "images/missing.png"), None);
        assert_eq!(resolve_image_target(&document, "images/readme.txt"), None);
        assert_eq!(
            resolve_image_target(&document, "https://mdow.dev/hero.png"),
            None
        );
    }

    #[test]
    fn showcase_local_link_and_image_resolve_to_real_fixture_files() {
        let fixture_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let document = fixture_directory.join("showcase.md");
        let guide = fixture_directory.join("guide.md");
        let image = fixture_directory.join("images/preview.png");

        assert_eq!(
            classify_link(&document, "./guide.md"),
            LinkRoute::Markdown(guide.clone()),
        );
        assert!(guide.is_file());
        assert_eq!(
            resolve_image_target(&document, "./images/preview.png"),
            Some(image.clone()),
        );
        assert!(image.is_file());
    }

    #[test]
    fn code_copy_feedback_is_scoped_to_one_block_and_expires_after_two_seconds() {
        let copied_at = Instant::now();
        let mut copied_code = Some((3, copied_at));

        assert!(code_copy_feedback_is_active(
            copied_code,
            3,
            copied_at + Duration::from_millis(1_999),
        ));
        assert!(!code_copy_feedback_is_active(
            copied_code,
            2,
            copied_at + Duration::from_millis(1_999),
        ));
        assert!(clear_expired_code_copy_feedback(
            &mut copied_code,
            3,
            copied_at + Duration::from_secs(2),
        ));
        assert_eq!(copied_code, None);
        assert!(!clear_expired_code_copy_feedback(
            &mut copied_code,
            3,
            copied_at + Duration::from_secs(3),
        ));
    }

    #[test]
    fn text_runs_use_the_selected_content_and_code_families() {
        let theme = Theme::for_appearance(gpui::WindowAppearance::Dark);
        let mut prefs = Prefs::default();
        prefs.apply(PrefEdit::ContentFont(ContentFont::Georgia));
        prefs.apply(PrefEdit::CodeFont(CodeFont::SfMono));
        let layout = inline_layout(&[
            InlineSpan::Text("hello ".into()),
            InlineSpan::Code("world".into()),
        ]);

        let runs = text_runs(
            &layout,
            &[],
            None,
            None,
            400,
            theme.foreground,
            theme,
            prefs.reader_style(),
            false,
        );

        assert_eq!(runs[0].font.family.as_ref(), ContentFont::Georgia.family());
        assert_eq!(runs[1].font.family.as_ref(), CodeFont::SfMono.family());
    }

    #[test]
    fn table_text_runs_enable_tabular_number_spacing() {
        let theme = Theme::for_appearance(gpui::WindowAppearance::Dark);
        let layout = inline_layout(&[InlineSpan::Text("123".into())]);

        let runs = text_runs(
            &layout,
            &[],
            None,
            None,
            400,
            theme.foreground,
            theme,
            Prefs::default().reader_style(),
            true,
        );

        assert_eq!(
            runs[0].font.features.tag_value_list(),
            &[("tnum".to_owned(), 1)],
        );
    }

    #[test]
    fn focused_link_range_is_underlined_without_splitting_the_text_surface() {
        let theme = Theme::for_appearance(gpui::WindowAppearance::Dark);
        let layout = inline_layout(&[
            InlineSpan::Link {
                label: vec![InlineSpan::Text("one".into())],
                target: "one.md".into(),
            },
            InlineSpan::Text(" and ".into()),
            InlineSpan::Link {
                label: vec![InlineSpan::Text("two".into())],
                target: "two.md".into(),
            },
        ]);

        let runs = text_runs(
            &layout,
            &layout.links,
            None,
            Some(1),
            400,
            theme.foreground,
            theme,
            Prefs::default().reader_style(),
            false,
        );

        assert!(runs[0].underline.is_none());
        assert!(runs[2].underline.is_some());
    }

    #[test]
    fn document_link_focus_targets_include_each_active_source_link() {
        let document = ParsedDocument {
            path: PathBuf::from("/tmp/links.md"),
            title: "Links".into(),
            frontmatter_title: None,
            source: String::new(),
            blocks: vec![DocumentBlock::Paragraph(vec![
                InlineSpan::Link {
                    label: vec![InlineSpan::Text("one".into())],
                    target: "one.md".into(),
                },
                InlineSpan::Text(" ".into()),
                InlineSpan::Link {
                    label: vec![InlineSpan::Text("two".into())],
                    target: "two.md".into(),
                },
            ])],
            headings: vec![],
        };

        assert_eq!(
            document_link_focus_targets(&document),
            vec![
                LinkFocusTarget {
                    key: LinkFocusKey::new(LinkSurfaceKey::block(0), 0),
                    target: "one.md".into(),
                },
                LinkFocusTarget {
                    key: LinkFocusKey::new(LinkSurfaceKey::block(0), 1),
                    target: "two.md".into(),
                },
            ],
        );
    }

    #[test]
    fn list_child_links_keep_source_order_and_distinct_focus_surfaces() {
        let document = parse_document(
            PathBuf::from("/tmp/list-links.md"),
            "- [before](before.md)\n\n  ```rust\n  let n = 1;\n  ```\n\n  [after](after.md)\n"
                .into(),
        );

        let targets = document_link_focus_targets(&document);

        assert_eq!(
            targets
                .iter()
                .map(|target| target.target.as_str())
                .collect::<Vec<_>>(),
            vec!["before.md", "after.md"],
        );
        assert_ne!(targets[0].key.surface, targets[1].key.surface);
    }

    #[test]
    fn large_table_and_following_block_have_distinct_link_focus_keys() {
        let link = || {
            vec![InlineSpan::Link {
                label: vec![InlineSpan::Text("link".into())],
                target: "target.md".into(),
            }]
        };
        let document = ParsedDocument {
            path: PathBuf::from("/tmp/large-table.md"),
            title: "Large table".into(),
            frontmatter_title: None,
            source: String::new(),
            blocks: vec![
                DocumentBlock::Table(TableBlock {
                    headers: (0..32).map(|_| link()).collect(),
                    rows: (0..31).map(|_| (0..32).map(|_| link()).collect()).collect(),
                    alignments: Vec::new(),
                }),
                DocumentBlock::Paragraph(link()),
            ],
            headings: vec![],
        };

        let targets = document_link_focus_targets(&document);
        let distinct_keys = targets
            .iter()
            .map(|target| target.key)
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(targets.len(), 1_025);
        assert_eq!(distinct_keys.len(), targets.len());
    }
}
