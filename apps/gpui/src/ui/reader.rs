use crate::{
    app::MdowApp,
    document::{
        DocumentBlock, InlineSpan, ListKind, ParsedDocument, TableBlock, is_supported_markdown,
        resolve_local_target,
    },
    theme::{Metrics, Theme},
    ui::primitives::icon,
};
use gpui::{
    AnyElement, Context, FocusHandle, Font, FontFeatures, FontStyle, FontWeight, Img,
    InteractiveElement, InteractiveText, IntoElement, ParentElement, ScrollHandle,
    StatefulInteractiveElement, Styled, StyledImage, StyledText, TextRun, UnderlineStyle, div,
    font, img, prelude::*, px, relative,
};
use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

pub const CODE_COPY_FEEDBACK_DURATION: Duration = Duration::from_secs(2);

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
            DocumentBlock::CodeBlock { .. } => Self::code_block(),
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
    pub link_target: Option<String>,
    pub link_node_id: Option<usize>,
}

impl InlineStyleRange {
    fn new(
        range: Range<usize>,
        emphasis: bool,
        strong: bool,
        code: bool,
        link_target: Option<String>,
        link_node_id: Option<usize>,
    ) -> Self {
        Self {
            range,
            emphasis,
            strong,
            code,
            link_target,
            link_node_id,
        }
    }
}

#[cfg(test)]
impl InlineStyleRange {
    fn emphasis(range: Range<usize>) -> Self {
        Self::new(range, true, false, false, None, None)
    }

    fn emphasis_strong(range: Range<usize>) -> Self {
        Self::new(range, true, true, false, None, None)
    }

    fn code(range: Range<usize>) -> Self {
        Self::new(range, false, false, true, None, None)
    }

    fn link(range: Range<usize>, target: &str) -> Self {
        Self::new(range, false, false, false, Some(target.to_owned()), Some(0))
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
    link_target: Option<&'a str>,
    link_node_id: Option<usize>,
}

pub fn inline_layout(spans: &[InlineSpan]) -> InlineLayout {
    inline_layout_with_transform(spans, false)
}

pub fn document_link_focus_targets(document: &ParsedDocument) -> Vec<LinkFocusTarget> {
    let mut targets = Vec::new();
    for (block_index, block) in document.blocks.iter().enumerate() {
        let mut surfaces: Vec<(LinkSurfaceKey, &[InlineSpan])> = Vec::new();
        match block {
            DocumentBlock::Heading { content, .. }
            | DocumentBlock::Paragraph(content)
            | DocumentBlock::ListItem { content, .. }
            | DocumentBlock::TaskItem { content, .. }
            | DocumentBlock::Blockquote(content) => {
                surfaces.push((LinkSurfaceKey::block(block_index), content));
            }
            DocumentBlock::Table(table) => {
                for (column_index, content) in table.headers.iter().enumerate() {
                    surfaces.push((
                        LinkSurfaceKey::table_header(block_index, column_index),
                        content,
                    ));
                }
                for (row_index, row) in table.rows.iter().enumerate() {
                    for (column_index, content) in row.iter().enumerate() {
                        surfaces.push((
                            LinkSurfaceKey::table_cell(block_index, row_index, column_index),
                            content,
                        ));
                    }
                }
            }
            DocumentBlock::CodeBlock { .. }
            | DocumentBlock::Image { .. }
            | DocumentBlock::ThematicBreak
            | DocumentBlock::RawText(_) => {}
        }

        for (surface, spans) in surfaces {
            let layout = inline_layout(spans);
            for (link_index, link) in layout
                .links
                .into_iter()
                .filter(|link| {
                    !matches!(
                        classify_link(&document.path, &link.target),
                        LinkRoute::Inert
                    )
                })
                .enumerate()
            {
                targets.push(LinkFocusTarget {
                    key: LinkFocusKey::new(surface, link_index),
                    target: link.target,
                });
            }
        }
    }
    targets
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
            InlineSpan::Code(code) => append_inline_text(
                code,
                InlineStyleContext {
                    code: true,
                    ..style
                },
                uppercase,
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
    if style.emphasis || style.strong || style.code || style.link_target.is_some() {
        layout.styles.push(InlineStyleRange::new(
            range.clone(),
            style.emphasis,
            style.strong,
            style.code,
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
    if is_supported_markdown(&path) {
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

fn is_list_block(block: &DocumentBlock) -> bool {
    matches!(
        block,
        DocumentBlock::ListItem { .. } | DocumentBlock::TaskItem { .. }
    )
}

fn block_margins(
    block: &DocumentBlock,
    previous: Option<&DocumentBlock>,
    next: Option<&DocumentBlock>,
) -> BlockMargins {
    if is_list_block(block) {
        return BlockMargins {
            top: if previous.is_some_and(is_list_block) {
                3.875
            } else {
                15.5
            },
            bottom: if next.is_some_and(is_list_block) {
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
        DocumentBlock::CodeBlock { .. } | DocumentBlock::Table(_) => BlockMargins {
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
            after: (index + 1 == margins.len())
                .then_some(margin.bottom)
                .unwrap_or(0.0),
        })
        .collect()
}

fn style_reader_image(image: Img) -> Img {
    image.max_w(relative(1.0)).rounded(px(8.0))
}

pub fn render_document(
    document: Arc<ParsedDocument>,
    wide_mode: bool,
    theme: Theme,
    copied_code: Option<(usize, Instant)>,
    link_state: &ReaderLinkState<'_>,
    scroll_handle: &ScrollHandle,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let mut column = div()
        .id("reader-column")
        .debug_selector(|| "reader-column".into())
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .px(px(Metrics::READER_INSET))
        .pt(px(Metrics::READER_TOP_PADDING))
        .pb(px(Metrics::READER_BOTTOM_PADDING))
        .font_family(Metrics::FONT_SANS)
        .font_weight(FontWeight::NORMAL)
        .text_size(px(15.5))
        .line_height(px(15.5 * 1.65))
        .text_color(theme.foreground)
        .when(!wide_mode, |column| {
            column.max_w(px(Metrics::READER_MAX_WIDTH)).mx_auto()
        });

    let spacing = block_sequence_spacing(&document.blocks);
    for (block_index, block) in document.blocks.iter().enumerate() {
        column = column.child(render_block(
            block,
            block_index,
            spacing[block_index],
            &document.path,
            theme,
            copied_code,
            link_state,
            cx,
        ));
    }

    div()
        .id("reader-scroll")
        .debug_selector(|| "reader-scroll".into())
        .flex()
        .flex_grow()
        .min_w_0()
        .min_h_0()
        .overflow_y_scroll()
        .scrollbar_width(px(6.0))
        .track_scroll(scroll_handle)
        .bg(theme.background)
        .child(column)
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_block(
    block: &DocumentBlock,
    block_index: usize,
    spacing: BlockSpacing,
    document_path: &Path,
    theme: Theme,
    copied_code: Option<(usize, Instant)>,
    link_state: &ReaderLinkState<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let content = match block {
        DocumentBlock::Heading { level, content } => {
            let style = BlockStyle::heading(*level);
            div()
                .id(("reader-block", block_index))
                .debug_selector(move || format!("reader-block-{block_index}"))
                .w_full()
                .min_w_0()
                .font_weight(FontWeight(style.font_weight as f32))
                .text_size(px(style.font_size))
                .line_height(px(style.font_size * style.line_height))
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
        DocumentBlock::Paragraph(content) => div()
            .id(("reader-block", block_index))
            .debug_selector(move || format!("reader-block-{block_index}"))
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
            .into_any_element(),
        DocumentBlock::ListItem {
            kind,
            depth,
            content,
        } => render_list_item(
            kind,
            *depth,
            content,
            block_index,
            document_path,
            theme,
            link_state,
            cx,
        ),
        DocumentBlock::TaskItem {
            checked,
            depth,
            content,
        } => render_task_item(
            *checked,
            *depth,
            content,
            block_index,
            document_path,
            theme,
            link_state,
            cx,
        ),
        DocumentBlock::Blockquote(content) => div()
            .id(("reader-block", block_index))
            .debug_selector(move || format!("reader-block-{block_index}"))
            .flex()
            .w_full()
            .min_w_0()
            .py(px(6.2))
            .text_color(theme.muted_foreground)
            .child(
                div()
                    .w(px(3.0))
                    .flex_none()
                    .rounded(px(1.5))
                    .bg(theme.border),
            )
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
            .into_any_element(),
        DocumentBlock::ThematicBreak => div()
            .id(("reader-block", block_index))
            .debug_selector(move || format!("reader-block-{block_index}"))
            .w_full()
            .child(div().h(px(1.0)).w_full().bg(theme.border))
            .into_any_element(),
        DocumentBlock::CodeBlock { language, code } => render_code_block(
            language.as_deref(),
            code,
            block_index,
            document_path,
            theme,
            copied_code,
            cx,
        ),
        DocumentBlock::Table(table) => {
            render_table(table, block_index, document_path, theme, link_state, cx)
        }
        DocumentBlock::Image { alt, source } => {
            render_image(alt, source, block_index, document_path, theme)
        }
        DocumentBlock::RawText(text) => div()
            .id(("reader-block", block_index))
            .debug_selector(move || format!("reader-block-{block_index}"))
            .w_full()
            .min_w_0()
            .child(StyledText::new(text.clone()))
            .into_any_element(),
    };

    div()
        .w_full()
        .min_w_0()
        .mt(px(spacing.before))
        .mb(px(spacing.after))
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
            } else {
                base_color
            },
            style.emphasis,
            style.code,
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
            tabular_numbers,
            theme,
        ));
    }
    runs
}

fn text_run(
    len: usize,
    weight: u16,
    color: gpui::Hsla,
    emphasis: bool,
    code: bool,
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
        strikethrough: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_list_item(
    kind: &ListKind,
    depth: usize,
    content: &[InlineSpan],
    block_index: usize,
    document_path: &Path,
    theme: Theme,
    link_state: &ReaderLinkState<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let marker = match kind {
        ListKind::Unordered => unordered_marker(depth).to_owned(),
        ListKind::Ordered { number } => format_ordered_marker(*number, depth),
    };
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || format!("reader-block-{block_index}"))
        .flex()
        .items_start()
        .w_full()
        .min_w_0()
        .gap(px(8.0))
        .ml(px(depth as f32 * 24.8))
        .child(
            div()
                .w(px(18.0))
                .flex_none()
                .text_right()
                .text_color(theme.muted_foreground)
                .child(marker),
        )
        .child(div().min_w_0().flex_grow().child(render_inline(
            content,
            document_path,
            LinkSurfaceKey::block(block_index),
            400,
            theme.foreground,
            theme,
            link_state,
            cx,
        )))
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn render_task_item(
    checked: bool,
    depth: usize,
    content: &[InlineSpan],
    block_index: usize,
    document_path: &Path,
    theme: Theme,
    link_state: &ReaderLinkState<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let checkbox = div()
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
        .debug_selector(move || format!("reader-block-{block_index}"))
        .flex()
        .items_start()
        .w_full()
        .min_w_0()
        .gap(px(8.0))
        .ml(px(depth as f32 * 24.8))
        .child(checkbox)
        .child(div().min_w_0().flex_grow().child(render_inline(
            content,
            document_path,
            LinkSurfaceKey::block(block_index),
            400,
            theme.foreground,
            theme,
            link_state,
            cx,
        )))
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

fn render_code_block(
    language: Option<&str>,
    code: &str,
    block_index: usize,
    document_path: &Path,
    theme: Theme,
    copied_code: Option<(usize, Instant)>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let copied = code_copy_feedback_is_active(copied_code, block_index, Instant::now());
    let code_to_copy = code.to_owned();
    let copy_button = div()
        .id(("copy-code", block_index))
        .debug_selector(move || format!("copy-code-{block_index}"))
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
    div()
        .id(("reader-block", block_index))
        .debug_selector(move || format!("reader-block-{block_index}"))
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
                .when_some(language.map(str::to_owned), |row, language| {
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
                            .debug_selector(move || format!("copied-code-{block_index}"))
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
                .w_full()
                .overflow_x_scroll()
                .scrollbar_width(px(6.0))
                .px(px(18.0))
                .py(px(14.0))
                .font_family(Metrics::FONT_MONO)
                .font_weight(FontWeight::NORMAL)
                .text_size(px(15.5 * 0.875))
                .line_height(px(15.5 * 0.875 * 1.6))
                .whitespace_nowrap()
                .child(code.to_owned()),
        )
        .into_any_element()
}

fn render_table(
    table: &TableBlock,
    block_index: usize,
    document_path: &Path,
    theme: Theme,
    link_state: &ReaderLinkState<'_>,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let column_count = table
        .headers
        .len()
        .max(table.rows.iter().map(Vec::len).max().unwrap_or(0))
        .max(1);
    let mut grid = div()
        .grid()
        .grid_cols(column_count as u16)
        .min_w(px(column_count as f32 * 140.0))
        .font_family(Metrics::FONT_SANS)
        .text_size(px(15.5 * 0.925))
        .line_height(px(15.5 * 0.925 * 1.5));
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
                .text_size(px(15.5 * 0.925 * 0.8))
                .line_height(px(15.5 * 0.925 * 1.3))
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
    use crate::document::InlineSpan;
    use std::time::{Duration, Instant};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    #[test]
    fn reader_styles_match_markdown_css() {
        assert_eq!(BlockStyle::heading(1).font_size, 15.5 * 1.875);
        assert_eq!(BlockStyle::heading(1).line_height, 1.2);
        assert_eq!(BlockStyle::heading(2).font_weight, 650);
        assert_eq!(BlockStyle::code_block().radius, 10.0);
        assert_eq!(BlockStyle::code_block().padding, [14.0, 18.0]);
        assert_eq!(BlockStyle::table_cell().padding, [10.0, 14.0]);
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
                content: vec![InlineSpan::Text("one".into())],
            },
            DocumentBlock::ListItem {
                kind: ListKind::Unordered,
                depth: 0,
                content: vec![InlineSpan::Text("two".into())],
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
            spacing[4].before, 3.875,
            "items keep compact internal rhythm"
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
        fs::create_dir(image.parent().unwrap()).unwrap();
        fs::write(&image, b"not decoded by this pure path test").unwrap();

        assert_eq!(
            resolve_image_target(&document, "images/hero.PNG"),
            Some(image),
        );
        assert_eq!(resolve_image_target(&document, "images/missing.png"), None);
        assert_eq!(resolve_image_target(&document, "images/readme.txt"), None);
        assert_eq!(
            resolve_image_target(&document, "https://mdow.dev/hero.png"),
            None
        );
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
