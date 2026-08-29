use crate::{
    app::{MdowApp, UserFacingError},
    session::Recents,
    theme::{Metrics, Theme},
    ui::primitives::{brand_logo, icon, outline_button},
};
use gpui::{AnyElement, Context, Hsla, div, prelude::*, px};

pub fn welcome(
    theme: Theme,
    recents: &Recents,
    drop_active: bool,
    cx: &Context<MdowApp>,
) -> AnyElement {
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

    let has_recents = !recents.is_empty();
    let hero = welcome_hero(
        theme,
        drop_active,
        drop_background,
        drop_border,
        has_recents,
        cx,
    );
    let mut shell = div()
        .flex()
        .w_full()
        .max_w(px(if has_recents { 720.0 } else { 540.0 }));
    if has_recents {
        shell = shell
            .flex_row()
            .items_start()
            .gap(px(40.0))
            .child(hero)
            .child(welcome_recents(theme, recents, cx));
    } else {
        shell = shell.flex_col().items_center().child(hero);
    }

    div()
        .flex()
        .flex_grow()
        .min_w_0()
        .min_h_0()
        .items_center()
        .justify_center()
        .px(px(32.0))
        .py(px(36.0))
        .child(shell)
        .into_any_element()
}

fn welcome_hero(
    theme: Theme,
    drop_active: bool,
    drop_background: Hsla,
    drop_border: Hsla,
    has_recents: bool,
    cx: &Context<MdowApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .flex_grow()
        .min_w_0()
        .when(!has_recents, |hero| hero.items_center())
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
                .when(!has_recents, |copy| copy.text_center())
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
                .when(has_recents, |row| row.justify_start())
                .when(!has_recents, |row| row.justify_center())
                .gap(px(8.0))
                .child(outline_button(
                    "welcome-open-file",
                    "Open File",
                    "icons/file.svg",
                    theme,
                    cx.listener(|this, _, _, cx| this.open_file_prompt(cx)),
                ))
                .child(outline_button(
                    "welcome-open-folder",
                    "Open Folder",
                    "icons/folder-open.svg",
                    theme,
                    cx.listener(|this, _, _, cx| this.open_folder_prompt(cx)),
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
        )
        .into_any_element()
}

fn welcome_recents(theme: Theme, recents: &Recents, cx: &Context<MdowApp>) -> AnyElement {
    let mut list = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .w(px(260.0))
        .flex_none()
        .child(
            div()
                .mb(px(6.0))
                .font_family(Metrics::FONT_MONO)
                .text_size(px(10.0))
                .text_color(theme.muted_foreground.opacity(0.7))
                .child("RECENT"),
        );
    for (index, path) in recents.iter().take(6).enumerate() {
        let path_buf = path.to_owned();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
            .to_owned();
        list = list.child(
            div()
                .id(("welcome-recent", index))
                .debug_selector(move || format!("welcome-recent-{index}"))
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(32.0))
                .px(px(8.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(move |style| style.bg(theme.muted))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.open_path(&path_buf, cx);
                }))
                .child(icon(
                    "icons/file.svg",
                    theme.muted_foreground.opacity(0.55),
                    13.0,
                ))
                .child(
                    div()
                        .min_w_0()
                        .flex_grow()
                        .truncate()
                        .font_family(Metrics::FONT_SANS)
                        .text_size(px(12.0))
                        .text_color(theme.foreground)
                        .child(name),
                ),
        );
    }
    list.into_any_element()
}

pub fn error_state(
    theme: Theme,
    error: &UserFacingError,
    drop_active: bool,
    cx: &Context<MdowApp>,
) -> AnyElement {
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
                    cx.listener(|this, _, _, cx| this.open_file_prompt(cx)),
                ))),
        )
        .into_any_element()
}
