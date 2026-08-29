use crate::{
    app::MdowApp,
    document::{
        AlertKind, DocumentBlock, InlineSpan, ListKind, ParsedDocument, TableBlock,
        footnote_ref_display, is_supported_document, resolve_local_target,
    },
    prefs::{READER_FONT_SIZE, ReaderStyle},
    syntax::{HighlightedCode, PreparedDocument},
    theme::{ColorScheme, Metrics, Theme},
    ui::primitives::icon,
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
        let muted = level >= 4;
        let uppercase = level >= 6;
        Self {
            font_size: 15.5 * scale,
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

    pub fn code_block() -> Self {
        Self {
            radius: 10.0,
            padding: [14.0, 18.0],
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

    pub fn blockquote() -> Self {
        Self {
            padding: [6.2, 16.0],
            ..Self::body()
        }
    }

    fn body() -> Self {
        Self {
            font_size: 15.5,
            font_weight: 400,
            line_height: 1.65,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineStyleRange {
    pub range: Range<usize>,
    pub emphasis: bool,
    pub strong: bool,
    pub code: bool,
    pub strikethrough: bool,
    pub footnote: bool,
    pub link_target: Option<String>,
    pub link_node_id: Option<usize>,
}

impl InlineStyleRange {
    #[allow(clippy::too_many_arguments)]
    fn new(
        range: Range<usize>,
        emphasis: bool,
        strong: bool,
        code: bool,
        strikethrough: bool,
        footnote: bool,
        link_target: Option<String>,
        link_node_id: Option<usize>,
    ) -> Self {
        Self {
            range,
            emphasis,
            strong,
            code,
            strikethrough,
            footnote,
            link_target,
            link_node_id,
        }
    }
}

#[cfg(test)]
impl InlineStyleRange {
    fn emphasis(range: Range<usize>) -> Self {
        Self::new(range, true, false, false, false, false, None, None)
    }

    fn emphasis_strong(range: Range<usize>) -> Self {
        Self::new(range, true, true, false, false, false, None, None)
    }

    fn code(range: Range<usize>) -> Self {
        Self::new(range, false, false, true, false, false, None, None)
    }

    fn link(range: Range<usize>, target: &str) -> Self {
        Self::new(
            range,
            false,
            false,
            false,
            false,
            false,
            Some(target.to_owned()),
            Some(0),
        )
    }

    fn strikethrough(range: Range<usize>) -> Self {
        Self::new(range, false, false, false, true, false, None, None)
    }

    fn footnote(range: Range<usize>) -> Self {
        Self::new(range, false, false, false, false, true, None, None)
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
    link_target: Option<&'a str>,
    link_node_id: Option<usize>,
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
        DocumentBlock::Heading { content, .. }
        | DocumentBlock::Paragraph(content)
        | DocumentBlock::Blockquote(content) => {
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
            InlineSpan::Text(text) => append_inline_text(text, style, uppercase, layout),
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
            InlineSpan::FootnoteRef { label } => append_inline_text(
                &footnote_ref_display(label),
                InlineStyleContext {
                    footnote: true,
                    ..style
                },
                false,
                layout,
            ),
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
            InlineSpan::SoftBreak => append_inline_text(" ", style, false, layout),
            InlineSpan::HardBreak => append_inline_text("\n", style, false, layout),
        }
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
    if style.emphasis
        || style.strong
        || style.code
        || style.strikethrough
        || style.footnote
        || style.link_target.is_some()
    {
        layout.styles.push(InlineStyleRange::new(
            range.clone(),
            style.emphasis,
            style.strong,
            style.code,
            style.strikethrough,
            style.footnote,
            style.link_target.map(str::to_owned),
            style.link_node_id,
        ));
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
    Web(String),
    Local(PathBuf),
    Inert,
}

pub fn classify_link(document_path: &Path, target: &str) -> LinkRoute {
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
    if let Some(group) = list_group(block) {
        return BlockMargins {
            top: if previous.and_then(list_group) == Some(group) {
                15.5 * 0.35
            } else {
                15.5
            },
            bottom: if next.and_then(list_group) == Some(group) {
                3.875
            } else {
                15.5
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
        | DocumentBlock::MermaidCard { .. }
        | DocumentBlock::Table(_)
        | DocumentBlock::Alert { .. }
        | DocumentBlock::FootnoteSection { .. } => BlockMargins {
            top: 19.375,
            bottom: 19.375,
        },
        DocumentBlock::ThematicBreak => BlockMargins {
            top: 31.0,
            bottom: 31.0,
        },
        DocumentBlock::Paragraph(_)
        | DocumentBlock::Blockquote(_)
        | DocumentBlock::Image { .. }
        | DocumentBlock::RawText(_) => BlockMargins {
            top: 0.0,
            bottom: 15.5,
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
            before: if index == 0
                && matches!(
                    blocks.first(),
                    Some(DocumentBlock::Heading { level: 1, .. })
                ) {
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
            document,
            style,
            theme,
            list_state,
            scrollbar_drag_grab_y: None,
        }
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

pub(crate) fn reader_key_target(key: &str, current: f32, viewport: f32, max: f32) -> Option<f32> {
    let page = viewport * 0.9;
    match key {
        "home" => Some(0.0),
        "end" => Some(-max),
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
}

impl ReaderView<'_> {
    fn zoom(self, base: f32) -> f32 {
        base * (self.style.font_size / READER_FONT_SIZE)
    }
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
    };
    div()
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
        .when_some(style.max_width, |item, width| {
            item.max_w(px(width)).mx_auto()
        })
        .child(render_block(
            document,
            block,
            &[block_index],
            None,
            spacing[block_index],
            list_marker_is_visible(&document.blocks, block_index),
            view,
            cx,
        ))
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
                let paint = app.reader_paint_state(cx);
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
    let content = match block {
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
                    theme.foreground,
                    theme,
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
        DocumentBlock::Blockquote(content) => {
            let debug_selector = format!("reader-block-{block_suffix}");
            div()
                .id(("reader-block", block_index))
                .debug_selector(move || debug_selector)
                .flex()
                .w_full()
                .min_w_0()
                .border_l(px(3.0))
                .border_color(theme.border)
                .py(px(6.2))
                .text_color(theme.muted_foreground)
                .child(
                    div()
                        .min_w_0()
                        .flex_grow()
                        .px(px(BlockStyle::blockquote().padding[1]))
                        .child(render_inline(
                            content,
                            document_path,
                            LinkSurfaceKey::block(block_index),
                            400,
                            theme.muted_foreground,
                            theme,
                            link_state,
                            cx,
                        )),
                )
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
        DocumentBlock::CodeBlock { language, code } => render_code_block(
            language.as_deref(),
            code,
            document.code_block_at(block_path),
            block_path,
            document_path,
            view,
            cx,
        ),
        DocumentBlock::Table(table) => render_table(table, block_index, document_path, view, cx),
        DocumentBlock::Image { alt, source } => {
            render_image(alt, source, block_index, document_path, theme)
        }
        DocumentBlock::Alert { kind, children } => {
            render_alert(*kind, children, block_path, document, view, cx)
        }
        DocumentBlock::MermaidCard { source } => render_code_block(
            Some("mermaid"),
            source,
            None,
            block_path,
            document_path,
            view,
            cx,
        ),
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

#[allow(clippy::too_many_arguments)]
pub fn render_inline(
    spans: &[InlineSpan],
    document_path: &Path,
    surface: LinkSurfaceKey,
    base_weight: u16,
    base_color: gpui::Hsla,
    theme: Theme,
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
    tabular_numbers: bool,
    link_state: &ReaderLinkState<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
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
        tabular_numbers,
    );
    let styled_text = StyledText::new(layout.text.clone()).with_runs(runs);
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
            ));
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
        ));
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
        ));
    }
    runs
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
) -> TextRun {
    let mut run_font: Font = font(if code {
        Metrics::FONT_MONO
    } else {
        Metrics::FONT_SANS
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
            indentation_depth as f32 * 24.8 + if marker_visible { 0.0 } else { 3.875 }
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

fn alert_accent(kind: AlertKind, theme: Theme) -> gpui::Hsla {
    match kind {
        AlertKind::Note | AlertKind::Tip => theme.primary,
        AlertKind::Important | AlertKind::Warning => theme.accent,
        AlertKind::Caution => theme.destructive,
    }
}

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
        .border_l(px(3.0))
        .border_color(accent)
        .bg(accent.opacity(0.08))
        .rounded(px(6.0))
        .px(px(14.0))
        .py(px(10.0))
        .gap(px(6.0))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(12.0))
                .text_color(accent)
                .child(kind.label().to_owned()),
        )
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
        ))
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
    let mut list = div().flex().flex_col().gap(px(10.0)).w_full().min_w_0();
    for (note_index, (label, children)) in notes.iter().enumerate() {
        let mut note_path = block_path.to_vec();
        note_path.push(note_index);
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
                        .text_color(theme.muted_foreground)
                        .child(footnote_ref_display(label)),
                )
                .child(render_list_children(
                    children,
                    0,
                    &note_path,
                    document,
                    ReaderView {
                        find_block: None,
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
        .border_color(theme.border_subtle)
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.muted_foreground)
                .child("Notes"),
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

fn highlighted_text_runs(
    highlighted: &HighlightedCode,
    dark: bool,
    family: &'static str,
) -> Vec<TextRun> {
    let source = if dark {
        &highlighted.dark_runs
    } else {
        &highlighted.light_runs
    };
    source
        .iter()
        .map(|run| {
            let hex = ((run.color.red as u32) << 16)
                | ((run.color.green as u32) << 8)
                | run.color.blue as u32;
            let mut run_font = font(family);
            run_font.weight = FontWeight::NORMAL;
            run_font.style = if run.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            };
            TextRun {
                len: run.len,
                font: run_font,
                color: gpui::Hsla::from(gpui::rgb(hex)),
                background_color: None,
                underline: None,
                strikethrough: None,
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn render_code_block(
    language: Option<&str>,
    code: &str,
    highlighted: Option<&HighlightedCode>,
    block_path: &[usize],
    document_path: &Path,
    view: ReaderView<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let theme = view.theme;
    let copied_code = view.copied_code;
    let block_index = block_path_render_index(block_path);
    let block_suffix = block_path_suffix(block_path);
    let copied = code_copy_feedback_is_active(copied_code, block_index, Instant::now());
    let code_to_copy = code.to_owned();
    let display_language = highlighted
        .map(|value| value.normalized_language.as_deref())
        .unwrap_or(language);
    let highlighted_text = highlighted
        .map(|value| {
            StyledText::new(value.text.clone()).with_runs(highlighted_text_runs(
                value,
                theme.color_scheme == ColorScheme::Dark,
                view.style.code_family,
            ))
        })
        .unwrap_or_else(|| StyledText::new(code.to_owned()));
    let copy_debug_selector = format!("copy-code-{block_suffix}");
    let copy_button = div()
        .id(("copy-code", block_index))
        .debug_selector(move || copy_debug_selector)
        .tab_index(0)
        .focusable()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.0))
        .rounded(px(6.0))
        .bg(theme.muted.opacity(0.92))
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .hover(move |style| style.bg(theme.card).text_color(theme.foreground))
        .active(|style| style.opacity(0.78))
        .focus(move |style| style.border_1().border_color(theme.primary))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.copy_code(block_index, code_to_copy.clone(), cx);
        }))
        .child(icon(
            if copied {
                "icons/check.svg"
            } else {
                "icons/copy.svg"
            },
            if copied {
                theme.primary
            } else {
                theme.muted_foreground
            },
            14.0,
        ));
    let block_debug_selector = format!("reader-block-{block_suffix}");
    let copied_debug_selector = format!("copied-code-{block_suffix}");
    let code_debug_selector = format!("reader-code-{block_suffix}");
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || block_debug_selector)
        .relative()
        .w_full()
        .rounded(px(10.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        .shadow_sm()
        .overflow_hidden()
        .child(
            div()
                .absolute()
                .top(px(7.0))
                .right(px(8.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .when_some(display_language.map(str::to_owned), |row, language| {
                    row.child(
                        div()
                            .font_family(Metrics::FONT_MONO)
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(11.0))
                            .line_height(px(16.0))
                            .text_color(theme.muted_foreground)
                            .child(language.to_lowercase()),
                    )
                })
                .when(copied, |row| {
                    row.child(
                        div()
                            .id(("copied-code", block_index))
                            .debug_selector(move || copied_debug_selector)
                            .font_family(Metrics::FONT_SANS)
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(11.0))
                            .text_color(theme.primary)
                            .child("Copied"),
                    )
                })
                .child(copy_button),
        )
        .child(
            restrict_scroll_to_axis(div())
                .id((
                    "code-scroll",
                    document_scoped_element_id(document_path, "code-scroll", block_index),
                ))
                .debug_selector(move || code_debug_selector)
                .w_full()
                .overflow_x_scroll()
                .scrollbar_width(px(6.0))
                .px(px(18.0))
                .py(px(14.0))
                .font_family(view.style.code_family)
                .font_weight(FontWeight::NORMAL)
                .text_size(px(view.zoom(15.5 * 0.875)))
                .line_height(px(view.zoom(15.5 * 0.875) * 1.6))
                .whitespace_nowrap()
                .child(highlighted_text),
        )
        .into_any_element()
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
        .text_size(px(view.zoom(15.5 * 0.925)))
        .line_height(px(view.zoom(15.5 * 0.925) * 1.5));
    for column_index in 0..column_count {
        let content = table.headers.get(column_index).cloned().unwrap_or_default();
        let surface = LinkSurfaceKey::table_header(block_index, column_index);
        grid = grid.child(
            div()
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
                .text_size(px(view.zoom(15.5 * 0.925 * 0.8)))
                .line_height(px(view.zoom(15.5 * 0.925 * 1.3)))
                .text_color(theme.muted_foreground)
                .child(render_inline_layout(
                    inline_layout_with_transform(&content, true),
                    document_path,
                    surface,
                    600,
                    theme.muted_foreground,
                    theme,
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
    let content = if let Some(path) = resolve_image_target(document_path, source) {
        style_reader_image(img(Arc::<Path>::from(path)))
            .with_fallback(fallback)
            .into_any_element()
    } else {
        fallback()
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
        assert_eq!(BlockStyle::body().font_size, 15.5);
        assert_eq!(BlockStyle::body().line_height, 1.65);
        assert_eq!(BlockStyle::heading(1).font_size, 15.5 * 1.875);
        assert_eq!(BlockStyle::heading(2).margin_top_em, 1.8);
        assert_eq!(BlockStyle::blockquote().padding, [6.2, 16.0]);
        assert_eq!(BlockStyle::code_block().radius, 10.0);
        assert_eq!(BlockStyle::code_block().padding, [14.0, 18.0]);
        assert_eq!(BlockStyle::table_cell().padding, [10.0, 14.0]);
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
        let highlighted = highlight_code(Some("rust"), "fn main() {}\n");
        let light = highlighted_text_runs(&highlighted, false, Metrics::FONT_MONO);
        let dark = highlighted_text_runs(&highlighted, true, Metrics::FONT_MONO);

        assert_eq!(
            light.iter().map(|run| run.len).sum::<usize>(),
            highlighted.text.len()
        );
        assert_eq!(
            dark.iter().map(|run| run.len).sum::<usize>(),
            highlighted.text.len()
        );
        assert!(
            light
                .iter()
                .all(|run| run.font.family.as_ref() == Metrics::FONT_MONO)
        );
        assert_ne!(light[0].color, dark[0].color);
    }

    #[test]
    fn heading_styles_preserve_the_complete_six_level_hierarchy() {
        let expected = [
            (29.0625, 700, 1.2, -0.025, 2.0, 0.6, false, false),
            (23.25, 650, 1.25, -0.02, 1.8, 0.5, false, false),
            (17.825, 600, 1.3, -0.01, 1.5, 0.4, false, false),
            (15.5, 600, 1.4, 0.0, 1.3, 0.3, true, false),
            (14.725, 600, 1.4, 0.0, 1.2, 0.25, true, false),
            (13.5625, 600, 1.4, 0.03, 1.0, 0.2, true, true),
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

        assert_eq!(layout.text, "A quiet reader uses mdow at home today\nnext");
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
                InlineStyleRange::footnote(5..7),
            ],
        );

        let runs = text_runs(
            &layout,
            &[],
            None,
            None,
            400,
            theme.foreground,
            theme,
            false,
        );
        assert!(runs[0].strikethrough.is_some());
        assert!(runs[1].strikethrough.is_none());
        assert_eq!(runs[2].color, theme.muted_foreground);
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
            },
            DocumentBlock::Table(TableBlock {
                headers: vec![],
                rows: vec![],
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
            "only a first H1 resets its top margin"
        );
        assert_eq!(spacing[1].before, 19.375);
        assert_eq!(
            spacing[2].before, 19.375,
            "code/table margins collapse to max"
        );
        assert_eq!(spacing[3].before, 19.375);
        assert_eq!(
            spacing[4].before,
            15.5 * 0.35,
            "adjacent items use the CSS li + li margin"
        );
        assert_eq!(
            spacing[4].after, 15.5,
            "the list group retains a 1em outer margin"
        );

        let first_h2 = block_sequence_spacing(&[DocumentBlock::Heading {
            level: 2,
            content: vec![InlineSpan::Text("Section".into())],
        }]);
        assert_eq!(first_h2[0].before, BlockStyle::heading(2).font_size * 1.8);
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

        assert_eq!(spacing[1].before, 15.5);
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

        assert_eq!(block_sequence_spacing(&blocks)[1].before, 15.5 * 0.35);
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
        assert_eq!(nested_spacing[1].before, 15.5);
        assert_eq!(nested_spacing[2].before, 15.5);
        assert_eq!(block_sequence_spacing(&same_depth)[1].before, 15.5 * 0.35);
    }

    #[test]
    fn blockquote_style_has_equal_sixteen_pixel_inline_insets() {
        assert_eq!(BlockStyle::blockquote().padding, [6.2, 16.0]);
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
        assert_eq!(classify_link(document, "#details"), LinkRoute::Inert);
        assert_eq!(
            classify_link(document, "javascript:alert(1)"),
            LinkRoute::Inert
        );
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
    fn table_text_runs_enable_tabular_number_spacing() {
        let theme = Theme::for_appearance(gpui::WindowAppearance::Dark);
        let layout = inline_layout(&[InlineSpan::Text("123".into())]);

        let runs = text_runs(&layout, &[], None, None, 400, theme.foreground, theme, true);

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
