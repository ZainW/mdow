//! Side-by-side reading: two reader panes under the one tab strip, like Electron's `SplitPane`.
//! Each pane gets a slim header (focus marker, file name, Left/Right badge) and centres its own
//! reading column; a draggable hairline divides them.

use crate::{
    app::MdowApp,
    split::{PaneId, SplitState, clamp_ratio},
    theme::{Metrics, Theme},
    ui::primitives::{compact_icon_button, icon},
};
use gpui::{
    AnyElement, Context, CursorStyle, DragMoveEvent, FontWeight, IntoElement, Render, Window, div,
    prelude::*, px, relative,
};

/// Drag payload for the divider between the panes.
#[derive(Debug, Clone, Copy)]
pub struct SplitDividerDrag;

/// The divider drags without a floating preview; the panes themselves resize live.
struct DividerDragPreview;

impl Render for DividerDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Half the divider's grab area on each side of its 1px line.
const DIVIDER_GRAB: f32 = 4.0;

pub struct SplitPane {
    pub pane: PaneId,
    pub title: Option<String>,
    pub active: bool,
    /// The document surface, or `None` for an empty pane.
    pub content: Option<AnyElement>,
}

fn pane_selector(pane: PaneId) -> &'static str {
    match pane {
        PaneId::Primary => "split-pane-primary",
        PaneId::Secondary => "split-pane-secondary",
    }
}

fn pane_header(theme: Theme, pane: &SplitPane, cx: &Context<MdowApp>) -> AnyElement {
    let ui = theme.ui;
    let header_id = match pane.pane {
        PaneId::Primary => "split-pane-header-primary",
        PaneId::Secondary => "split-pane-header-secondary",
    };
    div()
        .id(header_id)
        .debug_selector(move || header_id.into())
        .flex()
        .flex_none()
        .items_center()
        .gap(px(ui.space(8.0)))
        .h(px(ui.breadcrumb_height))
        .px(px(ui.space(12.0)))
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.background)
        .font_family(Metrics::FONT_SANS)
        .text_size(px(ui.breadcrumb_font))
        // Focus marker: the only difference between the panes, so it stays quiet.
        .child(
            div()
                .debug_selector(move || format!("{header_id}-marker"))
                .flex_none()
                .w(px(2.0))
                .h(px(ui.space(14.0)))
                .rounded(px(1.0))
                .bg(if pane.active {
                    theme.primary
                } else {
                    theme.primary.opacity(0.0)
                }),
        )
        .child(icon(
            "icons/file.svg",
            theme.muted_foreground.opacity(0.65),
            ui.icon_xs,
        ))
        .child(
            div()
                .min_w_0()
                .flex_grow()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .text_color(if pane.active {
                    theme.foreground.opacity(0.85)
                } else {
                    theme.muted_foreground
                })
                .child(pane.title.clone().unwrap_or_else(|| "No document".into())),
        )
        .child(
            div()
                .flex_none()
                .px(px(ui.space(6.0)))
                .py(px(ui.space(2.0)))
                .rounded(px(4.0))
                .bg(theme.muted)
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(ui.text(10.0)))
                .text_color(theme.muted_foreground)
                .child(pane.pane.label()),
        )
        .when(pane.pane == PaneId::Secondary, |header| {
            header.child(compact_icon_button(
                "close-split-view",
                "icons/columns-2.svg",
                20.0,
                12.0,
                theme,
                cx.listener(|this, _, _, cx| {
                    cx.stop_propagation();
                    this.toggle_split_view(cx);
                }),
            ))
        })
        .into_any_element()
}

fn empty_pane(theme: Theme) -> AnyElement {
    let ui = theme.ui;
    div()
        .flex()
        .flex_grow()
        .min_h_0()
        .items_center()
        .justify_center()
        .px(px(ui.space(24.0)))
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(ui.space(8.0)))
                .max_w(px(ui.space(288.0)))
                .text_center()
                .font_family(Metrics::FONT_SANS)
                .child(icon(
                    "icons/columns-2.svg",
                    theme.muted_foreground.opacity(0.6),
                    ui.icon,
                ))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_size(px(ui.text(13.0)))
                        .text_color(theme.foreground)
                        .child("No document in this pane"),
                )
                .child(
                    div()
                        .text_size(px(ui.text(12.0)))
                        .line_height(px(ui.space(20.0)))
                        .text_color(theme.muted_foreground)
                        .child("Select this pane, then open a document or choose a tab."),
                ),
        )
        .into_any_element()
}

fn render_pane(theme: Theme, pane: SplitPane, cx: &Context<MdowApp>) -> gpui::Stateful<gpui::Div> {
    let id = pane.pane;
    let selector = pane_selector(id);
    let header = pane_header(theme, &pane, cx);
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .flex()
        .flex_col()
        .min_w_0()
        .min_h_0()
        .h_full()
        .overflow_hidden()
        .bg(theme.background)
        // Capture so a click on a link or code button still focuses its pane first.
        .capture_any_mouse_down(cx.listener(move |this, _, _, cx| this.focus_pane(id, cx)))
        .child(header)
        .child(pane.content.unwrap_or_else(|| empty_pane(theme)))
}

pub fn render_split(
    theme: Theme,
    split: &SplitState,
    width: f32,
    dragging: bool,
    primary: SplitPane,
    secondary: SplitPane,
    cx: &Context<MdowApp>,
) -> AnyElement {
    let ratio = clamp_ratio(split.ratio(), width);
    let dragging = dragging && cx.has_active_drag();
    let line = if dragging {
        theme.primary.opacity(0.6)
    } else {
        theme.border_subtle
    };
    let hover_line = theme.border;
    let grab = div()
        .id("split-divider")
        .debug_selector(|| "split-divider".into())
        .group("split-divider")
        .absolute()
        .top_0()
        .bottom_0()
        .left(relative(ratio))
        .ml(px(-DIVIDER_GRAB))
        .w(px(DIVIDER_GRAB * 2.0 + 1.0))
        .flex()
        .justify_center()
        .cursor(CursorStyle::ResizeColumn)
        .on_drag(SplitDividerDrag, |_, _, _, cx| {
            cx.new(|_| DividerDragPreview)
        })
        .on_click(cx.listener(|this, event: &gpui::ClickEvent, _, cx| {
            if event.click_count() == 2 {
                this.reset_split_divider(cx);
            }
        }))
        .child(
            div()
                .w(px(1.0))
                .h_full()
                .bg(line.opacity(0.0))
                .group_hover("split-divider", move |style| style.bg(hover_line)),
        );

    div()
        .id("split-view")
        .debug_selector(|| "split-view".into())
        .relative()
        .flex()
        .flex_grow()
        .min_w_0()
        .min_h_0()
        .on_drag_move::<SplitDividerDrag>(cx.listener(
            |this, event: &DragMoveEvent<SplitDividerDrag>, _, cx| {
                let x = f32::from(event.event.position.x - event.bounds.left());
                this.drag_split_divider(x, f32::from(event.bounds.size.width), cx);
            },
        ))
        .on_drop::<SplitDividerDrag>(cx.listener(|this, _, _, cx| this.end_split_divider_drag(cx)))
        .child(
            render_pane(theme, primary, cx)
                .flex_none()
                .w(relative(ratio)),
        )
        .child(div().flex_none().w(px(1.0)).h_full().bg(line))
        .child(render_pane(theme, secondary, cx).flex_1())
        .child(grab)
        .into_any_element()
}
