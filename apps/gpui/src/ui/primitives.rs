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
        .debug_selector(move || id.to_string())
        // GPUI converts Enter/Space key-up events on a focused clickable element into clicks.
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
        .debug_selector(move || id.to_string())
        .group(id)
        // Keep this focusable: GPUI's clickable element behavior supplies keyboard activation.
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
        .child(
            icon(icon_path, theme.muted_foreground, Metrics::ICON_SIZE)
                .group_hover(id, move |style| style.text_color(theme.foreground)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, KeyUpEvent, Keystroke, Modifiers, MouseButton, Render, TestAppContext,
        VisualTestContext, WindowAppearance,
    };
    use std::{cell::Cell, rc::Rc};

    struct ButtonHarness {
        activation_count: Rc<Cell<usize>>,
    }

    impl Render for ButtonHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let activation_count = self.activation_count.clone();
            outline_button(
                "keyboard-test-button",
                "Open File",
                "icons/file.svg",
                Theme::for_appearance(WindowAppearance::Dark),
                move |_, _, _| activation_count.set(activation_count.get() + 1),
            )
        }
    }

    #[gpui::test]
    fn focused_button_activates_with_enter_and_space(cx: &mut TestAppContext) {
        let activation_count = Rc::new(Cell::new(0));
        let window = cx.update(|cx| {
            let activation_count = activation_count.clone();
            cx.open_window(Default::default(), |_, cx| {
                cx.new(|_| ButtonHarness { activation_count })
            })
            .unwrap()
        });

        let mut visual = VisualTestContext::from_window((*window).into(), cx);
        visual.update(|window, cx| window.draw(cx).clear());
        let button_center = visual
            .debug_bounds("keyboard-test-button")
            .expect("button should be painted")
            .center();
        visual.simulate_mouse_move(button_center, None, Modifiers::none());
        visual.simulate_mouse_down(button_center, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(button_center, MouseButton::Left, Modifiers::none());
        assert_eq!(activation_count.get(), 1);
        assert!(visual.update(|window, cx| window.focused(cx).is_some()));
        activation_count.set(0);
        visual.update(|window, cx| window.draw(cx).clear());
        visual.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").unwrap(),
        });
        visual.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").unwrap(),
        });

        assert_eq!(activation_count.get(), 2);
    }
}
