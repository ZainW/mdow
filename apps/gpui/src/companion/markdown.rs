//! A compact renderer for companion answers on the native markdown pipeline: the same
//! `parse_document` blocks and `inline_layout` spans as the reader, the same syntax
//! highlighter, laid out at UI size for a narrow panel.

use crate::{
    document::{DocumentBlock, ListKind, ParsedDocument, parse_document},
    graphics::GraphicState,
    syntax::{HighlightedCode, SyntaxColor, highlight_code},
    theme::{ColorScheme, Metrics, Theme},
    ui::graphic::{GraphicElement, math_state},
    ui::{
        primitives::icon,
        reader::{alert_accent, alert_icon, code_display_text, inline_layout},
    },
};
use gpui::{
    AnyElement, App, Font, FontStyle, FontWeight, InteractiveText, SharedString,
    StrikethroughStyle, StyledText, TextRun, UnderlineStyle, Window, div, font, prelude::*, px,
};
use std::{collections::HashMap, path::PathBuf, rc::Rc, sync::Arc};

pub const FONT_SIZE: f32 = 13.0;
pub const LINE_HEIGHT: f32 = 1.55;
const CODE_FONT_SIZE: f32 = 12.0;

pub type LinkHandler = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// Parses an answer (or a partial, still-streaming answer) into reader blocks.
pub fn parse(text: &str) -> Arc<ParsedDocument> {
    Arc::new(parse_document(
        PathBuf::from("companion.md"),
        text.to_owned(),
    ))
}

/// Highlighted code blocks keyed by language and source, per color scheme.
#[derive(Default)]
pub struct HighlightCache {
    entries: HashMap<(ColorScheme, Option<String>, String), Arc<HighlightedCode>>,
}

impl HighlightCache {
    pub fn get_or_highlight(
        &mut self,
        language: Option<&str>,
        code: &str,
        scheme: ColorScheme,
    ) -> Arc<HighlightedCode> {
        let key = (scheme, language.map(str::to_owned), code.to_owned());
        self.entries
            .entry(key)
            .or_insert_with(|| Arc::new(highlight_code(language, code, scheme)))
            .clone()
    }

    fn lookup(
        &self,
        language: Option<&str>,
        code: &str,
        scheme: ColorScheme,
    ) -> Option<Arc<HighlightedCode>> {
        self.entries
            .get(&(scheme, language.map(str::to_owned), code.to_owned()))
            .cloned()
    }

    /// Highlights every code block in a finished answer so the next paint can use them.
    pub fn prepare(&mut self, document: &ParsedDocument, scheme: ColorScheme) {
        fn walk(blocks: &[DocumentBlock], cache: &mut HighlightCache, scheme: ColorScheme) {
            for block in blocks {
                match block {
                    DocumentBlock::CodeBlock { language, code, .. } => {
                        cache.get_or_highlight(
                            language.as_deref(),
                            code_display_text(code),
                            scheme,
                        );
                    }
                    DocumentBlock::ListItem { children, .. }
                    | DocumentBlock::TaskItem { children, .. }
                    | DocumentBlock::Blockquote(children)
                    | DocumentBlock::Alert { children, .. } => walk(children, cache, scheme),
                    _ => {}
                }
            }
        }
        walk(&document.blocks, self, scheme);
    }
}

#[derive(Clone)]
pub struct MarkdownView<'a> {
    pub theme: Theme,
    pub id: SharedString,
    pub highlights: &'a HighlightCache,
    pub on_link: LinkHandler,
    pub muted: bool,
}

fn syntax_color(color: SyntaxColor) -> gpui::Hsla {
    gpui::Hsla::from(gpui::rgb(
        (u32::from(color.red) << 16) | (u32::from(color.green) << 8) | u32::from(color.blue),
    ))
}

fn run_font(code: bool, weight: FontWeight, italic: bool) -> Font {
    let mut run = font(if code {
        Metrics::FONT_MONO
    } else {
        Metrics::FONT_SANS
    });
    run.weight = weight;
    run.style = if italic {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };
    run
}

impl MarkdownView<'_> {
    fn text_color(&self) -> gpui::Hsla {
        if self.muted {
            self.theme.muted_foreground
        } else {
            self.theme.foreground
        }
    }

    pub fn render(&self, document: &ParsedDocument) -> AnyElement {
        let mut column = div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .w_full()
            .min_w_0()
            .font_family(Metrics::FONT_SANS)
            .text_size(px(FONT_SIZE))
            .line_height(px(FONT_SIZE * LINE_HEIGHT))
            .text_color(self.text_color());
        for (index, block) in document.blocks.iter().enumerate() {
            column = column.child(self.block(block, &[index], None));
        }
        column.into_any_element()
    }

    fn element_id(&self, path: &[usize], suffix: &str) -> SharedString {
        let path = path
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("-");
        format!("{}-{suffix}-{path}", self.id).into()
    }

    fn inline(
        &self,
        spans: &[crate::document::InlineSpan],
        weight: FontWeight,
        color: gpui::Hsla,
        path: &[usize],
    ) -> AnyElement {
        let layout = inline_layout(spans);
        if layout.text.is_empty() {
            return div().into_any_element();
        }
        let theme = self.theme;
        let mut runs = Vec::new();
        let mut cursor = 0;
        let base = |len: usize| TextRun {
            len,
            font: run_font(false, weight, false),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        for style in &layout.styles {
            if cursor < style.range.start {
                runs.push(base(style.range.start - cursor));
            }
            let link = style.link_target.is_some();
            runs.push(TextRun {
                len: style.range.len(),
                font: run_font(
                    style.code || style.kbd,
                    if style.strong {
                        FontWeight::BOLD
                    } else {
                        weight
                    },
                    style.emphasis,
                ),
                color: if link { theme.primary } else { color },
                background_color: (style.code || style.kbd || style.mark).then_some(
                    if style.mark {
                        theme.accent.opacity(0.22)
                    } else {
                        theme.muted
                    },
                ),
                underline: link.then_some(UnderlineStyle {
                    thickness: px(1.0),
                    color: Some(theme.primary.opacity(0.4)),
                    wavy: false,
                }),
                strikethrough: style.strikethrough.then_some(StrikethroughStyle {
                    thickness: px(1.0),
                    color: None,
                }),
            });
            cursor = style.range.end;
        }
        if cursor < layout.text.len() {
            runs.push(base(layout.text.len() - cursor));
        }
        let styled = StyledText::new(layout.text.clone()).with_runs(runs);
        if layout.links.is_empty() {
            return div().w_full().min_w_0().child(styled).into_any_element();
        }
        let ranges = layout
            .links
            .iter()
            .map(|link| link.range.clone())
            .collect::<Vec<_>>();
        let targets = layout
            .links
            .iter()
            .map(|link| link.target.clone())
            .collect::<Vec<_>>();
        let on_link = self.on_link.clone();
        div()
            .w_full()
            .min_w_0()
            .child(
                InteractiveText::new(self.element_id(path, "inline"), styled).on_click(
                    ranges,
                    move |index, window, cx| {
                        if let Some(target) = targets.get(index) {
                            on_link(target, window, cx);
                        }
                    },
                ),
            )
            .into_any_element()
    }

    fn children(
        &self,
        blocks: &[DocumentBlock],
        path: &[usize],
        list_depth: Option<usize>,
    ) -> AnyElement {
        let mut column = div().flex().flex_col().gap(px(6.0)).min_w_0().flex_1();
        for (index, block) in blocks.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(index);
            column = column.child(self.block(block, &child_path, list_depth));
        }
        column.into_any_element()
    }

    fn block(
        &self,
        block: &DocumentBlock,
        path: &[usize],
        list_depth: Option<usize>,
    ) -> AnyElement {
        let theme = self.theme;
        match block {
            DocumentBlock::Heading { level, content } => {
                let size = match level {
                    1 => 16.0,
                    2 => 15.0,
                    3 => 14.0,
                    _ => FONT_SIZE,
                };
                div()
                    .mt(px(4.0))
                    .text_size(px(size))
                    .line_height(px(size * 1.35))
                    .child(self.inline(content, FontWeight::SEMIBOLD, theme.foreground, path))
                    .into_any_element()
            }
            DocumentBlock::Paragraph(spans) => {
                self.inline(spans, FontWeight::NORMAL, self.text_color(), path)
            }
            DocumentBlock::ListItem {
                kind,
                depth,
                children,
            } => {
                let marker = match kind {
                    ListKind::Unordered => match depth {
                        0 => "•".to_owned(),
                        1 => "◦".to_owned(),
                        _ => "▪".to_owned(),
                    },
                    ListKind::Ordered { number } => format!("{number}."),
                };
                let indent = depth.saturating_sub(list_depth.map_or(0, |parent| parent + 1));
                div()
                    .flex()
                    .items_start()
                    .gap(px(6.0))
                    .ml(px(indent as f32 * 16.0))
                    .min_w_0()
                    .child(
                        div()
                            .w(px(16.0))
                            .flex_none()
                            .text_right()
                            .text_color(theme.muted_foreground)
                            .child(marker),
                    )
                    .child(self.children(children, path, Some(*depth)))
                    .into_any_element()
            }
            DocumentBlock::TaskItem {
                checked,
                depth,
                children,
            } => {
                let indent = depth.saturating_sub(list_depth.map_or(0, |parent| parent + 1));
                let checked = *checked;
                div()
                    .flex()
                    .items_start()
                    .gap(px(6.0))
                    .ml(px(indent as f32 * 16.0))
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .mt(px(3.5))
                            .size(px(13.0))
                            .flex_none()
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(if checked { theme.primary } else { theme.border })
                            .bg(if checked {
                                theme.primary
                            } else {
                                theme.background
                            })
                            .when(checked, |mark| {
                                mark.child(icon("icons/check.svg", theme.background, 9.0))
                            }),
                    )
                    .child(self.children(children, path, Some(*depth)))
                    .into_any_element()
            }
            DocumentBlock::Blockquote(children) => {
                let quoted = MarkdownView {
                    muted: true,
                    ..self.clone()
                };
                div()
                    .pl(px(10.0))
                    .border_l_2()
                    .border_color(theme.muted_foreground.opacity(0.45))
                    .child(quoted.children(children, path, list_depth))
                    .into_any_element()
            }
            DocumentBlock::Alert { kind, children } => {
                let accent = alert_accent(*kind, theme);
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .pl(px(10.0))
                    .border_l_2()
                    .border_color(accent)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(accent)
                            .child(icon(alert_icon(*kind), accent, 13.0))
                            .child(kind.label()),
                    )
                    .child(self.children(children, path, list_depth))
                    .into_any_element()
            }
            DocumentBlock::CodeBlock { language, code, .. } => {
                self.code_block(language.as_deref(), code_display_text(code), path)
            }
            DocumentBlock::MermaidCard { source } => {
                self.code_block(Some("mermaid"), code_display_text(source), path)
            }
            DocumentBlock::Math { tex } => self.math_block(tex, path),
            DocumentBlock::Table(table) => {
                let mut grid = div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .overflow_hidden();
                let row =
                    |cells: &[Vec<crate::document::InlineSpan>], header: bool, row_index: usize| {
                        let mut line = div()
                            .flex()
                            .w_full()
                            .when(header, |line| line.bg(theme.muted))
                            .when(row_index > 0, |line| {
                                line.border_t_1().border_color(theme.border_subtle)
                            });
                        for (column, cell) in cells.iter().enumerate() {
                            line = line.child(
                                div()
                                    .flex_1()
                                    .flex_basis(px(0.0))
                                    .min_w_0()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .child(self.inline(
                                        cell,
                                        if header {
                                            FontWeight::SEMIBOLD
                                        } else {
                                            FontWeight::NORMAL
                                        },
                                        theme.foreground,
                                        &[path, &[row_index, column]].concat(),
                                    )),
                            );
                        }
                        line
                    };
                grid = grid.child(row(&table.headers, true, 0));
                for (index, cells) in table.rows.iter().enumerate() {
                    grid = grid.child(row(cells, false, index + 1));
                }
                grid.into_any_element()
            }
            DocumentBlock::ThematicBreak => div()
                .h(px(1.0))
                .my(px(2.0))
                .bg(theme.border_subtle)
                .into_any_element(),
            DocumentBlock::Image { alt, source } => div()
                .text_color(theme.muted_foreground)
                .child(format!(
                    "[image: {}]",
                    if alt.is_empty() { source } else { alt }
                ))
                .into_any_element(),
            DocumentBlock::RawText(text) => div()
                .text_color(self.text_color())
                .child(text.clone())
                .into_any_element(),
            DocumentBlock::FootnoteSection { notes } => {
                let mut column = div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .pt(px(6.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .text_size(px(12.0));
                for (index, (label, blocks)) in notes.iter().enumerate() {
                    column = column.child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("{label}.")),
                            )
                            .child(self.children(blocks, &[path, &[index]].concat(), None)),
                    );
                }
                column.into_any_element()
            }
        }
    }

    /// Display math typeset like the reader's, or its TeX source when it cannot be typeset.
    fn math_block(&self, tex: &str, path: &[usize]) -> AnyElement {
        let theme = self.theme;
        let content = match math_state(tex, true, self.text_color(), FONT_SIZE) {
            GraphicState::Ready(graphic) => div()
                .flex_none()
                .py(px(2.0))
                .child(GraphicElement::new(
                    graphic.image,
                    graphic.width,
                    graphic.height,
                    false,
                ))
                .into_any_element(),
            GraphicState::Pending | GraphicState::Failed(_) => div()
                .font_family(Metrics::FONT_MONO)
                .text_size(px(CODE_FONT_SIZE))
                .text_color(theme.muted_foreground)
                .whitespace_nowrap()
                .child(format!("$${}$$", tex.trim()))
                .into_any_element(),
        };
        div()
            .id(self.element_id(path, "math"))
            .w_full()
            .overflow_x_scroll()
            .child(content)
            .into_any_element()
    }

    fn code_block(&self, language: Option<&str>, code: &str, path: &[usize]) -> AnyElement {
        let theme = self.theme;
        let highlighted = self.highlights.lookup(language, code, theme.color_scheme);
        let text = SharedString::from(code.to_owned());
        let styled = match highlighted {
            Some(highlighted) if highlighted.text == code => {
                let runs = highlighted
                    .runs
                    .iter()
                    .map(|run| TextRun {
                        len: run.len,
                        font: run_font(true, FontWeight::NORMAL, run.italic),
                        color: syntax_color(run.color),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    })
                    .collect::<Vec<_>>();
                StyledText::new(text).with_runs(runs)
            }
            _ => StyledText::new(text).with_runs(vec![TextRun {
                len: code.len(),
                font: run_font(true, FontWeight::NORMAL, false),
                color: theme.foreground,
                background_color: None,
                underline: None,
                strikethrough: None,
            }]),
        };
        div()
            .id(self.element_id(path, "code"))
            .w_full()
            .min_w_0()
            .rounded(px(6.0))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.code_surface)
            .px(px(10.0))
            .py(px(8.0))
            .overflow_x_scroll()
            .text_size(px(CODE_FONT_SIZE))
            .line_height(px(CODE_FONT_SIZE * 1.5))
            .whitespace_nowrap()
            .child(styled)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streamed_partial_markdown_parses_without_losing_text() {
        let document = parse("Here is a list:\n\n- one\n- tw");
        assert_eq!(document.blocks.len(), 3);
        let unfinished_fence = parse("```rust\nfn main() {");
        assert!(matches!(
            &unfinished_fence.blocks[0],
            DocumentBlock::CodeBlock { code, .. } if code.contains("fn main")
        ));
    }

    #[test]
    fn highlight_cache_prepares_nested_code_blocks_once() {
        let document = parse("- item\n\n  ```rust\n  let x = 1;\n  ```\n");
        let mut cache = HighlightCache::default();
        cache.prepare(&document, ColorScheme::Dark);
        assert_eq!(cache.entries.len(), 1);
        cache.prepare(&document, ColorScheme::Dark);
        assert_eq!(cache.entries.len(), 1);
        assert!(
            cache
                .lookup(Some("rust"), "let x = 1;", ColorScheme::Dark)
                .is_some()
        );
    }
}
