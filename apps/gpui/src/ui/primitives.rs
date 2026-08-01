use crate::theme::{Metrics, Theme};
use gpui::{App, ClickEvent, Hsla, Img, IntoElement, Svg, Window, div, img, prelude::*, px, svg};

pub fn brand_logo(size: f32) -> Img {
    img("icons/mdow-logo.svg").size(px(size)).flex_none()
}

pub fn icon(path: &'static str, color: Hsla, size: f32) -> Svg {
    svg()
        .path(path)
        .size(px(size))
        .text_color(color)
        .flex_none()
}

pub fn outline_button(
    id: &'static str,
    label: &'static str,
    icon_path: &'static str,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .tab_index(0)
        .focusable()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(7.0))
        .h(px(30.0))
        .px(px(12.0))
        .rounded(px(7.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.card)
        .font_family(Metrics::FONT_SANS)
        .text_size(px(Metrics::CONTROL_FONT_SIZE))
        .text_color(theme.foreground)
        .cursor_pointer()
        .hover(move |style| {
            style
                .bg(theme.muted)
                .border_color(theme.muted_foreground.opacity(0.42))
        })
        .active(|style| style.opacity(0.82))
        .focus(move |style| style.border_color(theme.primary))
        .on_click(on_click)
        .child(icon(icon_path, theme.muted_foreground, Metrics::ICON_SIZE))
        .child(label)
}

pub fn icon_button(
    id: &'static str,
    icon_path: &'static str,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .tab_index(0)
        .focusable()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.0))
        .rounded(px(6.0))
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
        .active(|style| style.opacity(0.8))
        .focus(move |style| style.border_1().border_color(theme.primary))
        .on_click(on_click)
        .child(icon(icon_path, theme.muted_foreground, Metrics::ICON_SIZE))
}
