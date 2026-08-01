use crate::{
    actions::{OpenFile, OpenFolder},
    app::UserFacingError,
    theme::{Metrics, Theme},
    ui::primitives::{brand_logo, icon, outline_button},
};
use gpui::{AnyElement, div, prelude::*, px};

pub fn welcome(theme: Theme, drop_active: bool) -> AnyElement {
    let drop_background = if drop_active {
        theme.primary.opacity(0.08)
    } else {
        theme.muted.opacity(0.38)
    };
    let drop_border = if drop_active {
        theme.primary.opacity(0.68)
    } else {
        theme.border
    };

    div()
        .flex()
        .flex_grow()
        .min_w_0()
        .min_h_0()
        .items_center()
        .justify_center()
        .px(px(32.0))
        .py(px(36.0))
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .w_full()
                .max_w(px(540.0))
                .child(
                    div()
                        .size(px(48.0))
                        .rounded(px(13.0))
                        .border_1()
                        .border_color(theme.border_subtle)
                        .shadow_sm()
                        .child(brand_logo(48.0)),
                )
                .child(
                    div()
                        .mt(px(18.0))
                        .font_family(Metrics::FONT_SANS)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(24.0))
                        .line_height(px(29.0))
                        .text_color(theme.foreground)
                        .child("Mdow"),
                )
                .child(
                    div()
                        .mt(px(8.0))
                        .max_w(px(430.0))
                        .text_center()
                        .font_family(Metrics::FONT_SANS)
                        .text_size(px(14.0))
                        .line_height(px(21.0))
                        .text_color(theme.muted_foreground)
                        .child("A quiet markdown viewer. Drop a file anywhere, or open one below."),
                )
                .child(
                    div()
                        .mt(px(20.0))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .child(outline_button(
                            "welcome-open-file",
                            "Open File",
                            "icons/file.svg",
                            theme,
                            |_, _, cx| cx.dispatch_action(&OpenFile),
                        ))
                        .child(outline_button(
                            "welcome-open-folder",
                            "Open Folder",
                            "icons/folder-open.svg",
                            theme,
                            |_, _, cx| cx.dispatch_action(&OpenFolder),
                        )),
                )
                .child(
                    div()
                        .mt(px(24.0))
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .w_full()
                        .max_w(px(448.0))
                        .h(px(76.0))
                        .px(px(16.0))
                        .rounded(px(Metrics::RADIUS))
                        .border_1()
                        .border_dashed()
                        .border_color(drop_border)
                        .bg(drop_background)
                        .text_color(theme.muted_foreground)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(32.0))
                                .flex_none()
                                .rounded(px(7.0))
                                .bg(theme.muted)
                                .child(icon("icons/file.svg", theme.muted_foreground, 17.0)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(3.0))
                                .font_family(Metrics::FONT_SANS)
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_size(px(12.0))
                                        .text_color(theme.foreground)
                                        .child(if drop_active {
                                            "Release to open in Mdow"
                                        } else {
                                            "Anywhere in this window"
                                        }),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .text_size(px(11.0))
                                        .child("Drop ")
                                        .child(
                                            div()
                                                .font_family(Metrics::FONT_MONO)
                                                .text_color(theme.foreground)
                                                .child(".md"),
                                        )
                                        .child(" files or a folder"),
                                ),
                        ),
                ),
        )
        .into_any_element()
}

pub fn error_state(theme: Theme, error: &UserFacingError, drop_active: bool) -> AnyElement {
    let background = if drop_active {
        theme.primary.opacity(0.06)
    } else {
        theme.background
    };

    div()
        .flex()
        .flex_grow()
        .min_w_0()
        .min_h_0()
        .items_center()
        .justify_center()
        .px(px(32.0))
        .py(px(36.0))
        .bg(background)
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .w_full()
                .max_w(px(520.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(48.0))
                        .rounded(px(24.0))
                        .bg(theme.muted)
                        .child(icon("icons/alert-circle.svg", theme.muted_foreground, 22.0)),
                )
                .child(
                    div()
                        .mt(px(16.0))
                        .font_family(Metrics::FONT_SANS)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_size(px(16.0))
                        .line_height(px(21.0))
                        .text_color(theme.foreground)
                        .child(error.title.clone()),
                )
                .child(
                    div()
                        .mt(px(7.0))
                        .max_w(px(430.0))
                        .text_center()
                        .font_family(Metrics::FONT_SANS)
                        .text_size(px(14.0))
                        .line_height(px(21.0))
                        .text_color(theme.muted_foreground)
                        .child(error.body.clone()),
                )
                .child(
                    div()
                        .mt(px(10.0))
                        .max_w(px(430.0))
                        .truncate()
                        .font_family(Metrics::FONT_MONO)
                        .text_size(px(11.0))
                        .text_color(theme.muted_foreground.opacity(0.72))
                        .child(error.path.to_string_lossy().into_owned()),
                )
                .child(div().mt(px(18.0)).child(outline_button(
                    "error-open-file",
                    "Open File",
                    "icons/file.svg",
                    theme,
                    |_, _, cx| cx.dispatch_action(&OpenFile),
                ))),
        )
        .into_any_element()
}
