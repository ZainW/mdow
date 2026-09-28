use crate::theme::{ColorScheme, Metrics, Theme};
use gpui::{
    AbsoluteLength, AnyElement, App, ClickEvent, Context, Div, Entity, EventEmitter, FocusHandle,
    Focusable, FontFeatures, FontWeight, Hsla, Img, IntoElement, KeyDownEvent, Pixels, Point,
    Render, SharedString, Stateful, Svg, Window, anchored, deferred, div, img, prelude::*, px, svg,
};
use std::sync::Arc;

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

pub fn compact_icon_button(
    id: &'static str,
    icon_path: &'static str,
    target_size: f32,
    icon_size: f32,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .tab_index(0)
        .focusable()
        .flex()
        .items_center()
        .justify_center()
        .size(px(target_size))
        .flex_none()
        .rounded(px(5.0))
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
        .active(|style| style.opacity(0.8))
        .focus(move |style| style.border_1().border_color(theme.primary))
        .on_click(on_click)
        .child(icon(icon_path, theme.muted_foreground, icon_size))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ListRowStyle {
    pub selected: bool,
    pub indent: f32,
}

pub fn list_row(id: (&'static str, usize), style: ListRowStyle, theme: Theme) -> Stateful<Div> {
    let selected = style.selected;
    div()
        .id(id)
        .debug_selector(move || format!("{}-{}", id.0, id.1))
        .tab_index(0)
        .focusable()
        .flex()
        .items_center()
        .h(px(28.0))
        .min_w_0()
        .flex_none()
        .pl(px(10.0 + style.indent))
        .pr(px(10.0))
        .rounded(px(5.0))
        .bg(if selected {
            theme.sidebar_accent
        } else {
            theme.sidebar_accent.opacity(0.0)
        })
        .when(!selected, |row| {
            row.hover(move |style| style.bg(theme.sidebar_accent))
        })
        .font_family(Metrics::FONT_SANS)
        .text_size(px(Metrics::APP_FONT_SIZE))
        .text_color(theme.foreground)
        .cursor_pointer()
        .focus(move |style| style.border_1().border_color(theme.primary))
}

pub fn key_hint(keys: &'static str, theme: Theme) -> Div {
    div()
        .flex_none()
        .px(px(6.0))
        .h(px(18.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.muted)
        .font_family(Metrics::FONT_MONO)
        .text_size(px(10.0))
        .text_color(theme.muted_foreground)
        .child(keys)
}

/// Digits keep a fixed advance so live counts never nudge their neighbours.
pub fn tabular_nums<E: Styled>(mut element: E) -> E {
    element
        .text_style()
        .get_or_insert_with(Default::default)
        .font_features = Some(FontFeatures(Arc::new(vec![("tnum".into(), 1)])));
    element
}

/// Sets every border edge to a fractional width (GPUI only ships whole-pixel helpers).
pub fn border_width<E: Styled>(mut element: E, width: f32) -> E {
    let style = element.style();
    let width = AbsoluteLength::from(px(width));
    style.border_widths.top = Some(width);
    style.border_widths.right = Some(width);
    style.border_widths.bottom = Some(width);
    style.border_widths.left = Some(width);
    element
}

/// Keyboard hint chip (`⌘,`), 10.5px medium on a muted keycap.
pub fn kbd(keys: &'static str, theme: Theme) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .px(px(5.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.muted)
        .font_family(Metrics::FONT_SANS)
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(10.5))
        .line_height(px(10.5))
        .text_color(theme.muted_foreground)
        .child(keys)
}

/// Small tabular count label ("42 files", "3 matches").
pub fn count_label(text: impl Into<SharedString>, theme: Theme) -> Div {
    tabular_nums(
        div()
            .flex_none()
            .font_family(Metrics::FONT_SANS)
            .font_weight(FontWeight::MEDIUM)
            .text_size(px(10.5))
            .text_color(theme.muted_foreground)
            .child(text.into()),
    )
}

pub fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{} {noun}", format_count(count))
}

/// Thousands separators, matching the Electron UI's `toLocaleString()` for English.
pub fn format_count(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// One well with equal-width segments; the selected one is raised.
pub fn segmented_track(theme: Theme) -> Div {
    div()
        .flex()
        .flex_none()
        .gap(px(2.0))
        .h(px(Metrics::SEGMENTED_HEIGHT))
        .p(px(2.0))
        .rounded(px(7.0))
        .bg(theme.surface_well)
}

pub fn segment(
    id: &'static str,
    label: &'static str,
    icon_path: &'static str,
    selected: bool,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let raised = match theme.color_scheme {
        ColorScheme::Dark => theme.sidebar_accent,
        ColorScheme::Light => theme.surface_raised,
    };
    let text = if selected {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    // Every segment carries the same hairline so selection never shifts the labels.
    border_width(
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .tab_index(0)
            .focusable()
            .flex()
            .flex_1()
            .flex_basis(px(0.0))
            .min_w_0()
            .items_center()
            .justify_center()
            .gap(px(5.0))
            .rounded(px(5.0))
            .border_color(if selected {
                theme.border
            } else {
                theme.border.opacity(0.0)
            })
            .bg(if selected {
                raised
            } else {
                raised.opacity(0.0)
            })
            .when(
                selected && theme.color_scheme == ColorScheme::Light,
                |segment| segment.shadow_sm(),
            )
            .when(!selected, |segment| {
                segment.hover(move |style| style.text_color(theme.foreground))
            })
            .font_family(Metrics::FONT_SANS)
            .font_weight(FontWeight::MEDIUM)
            .text_size(px(11.5))
            .text_color(text)
            .cursor_pointer()
            .focus(move |style| style.border_color(theme.primary))
            .on_click(on_click)
            .child(icon(icon_path, text, 13.0))
            .child(div().truncate().child(label)),
        0.5,
    )
}

/// Filled (foreground on background) or outlined 32px action button with a shortcut hint.
pub fn action_button(
    id: &'static str,
    label: &'static str,
    icon_path: &'static str,
    shortcut: &'static str,
    primary: bool,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (bg, fg, border) = if primary {
        (
            theme.foreground,
            theme.background,
            theme.foreground.opacity(0.0),
        )
    } else {
        let bg = match theme.color_scheme {
            ColorScheme::Dark => theme.muted,
            ColorScheme::Light => theme.surface_raised,
        };
        (bg, theme.foreground, theme.border)
    };
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .tab_index(0)
        .focusable()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(7.0))
        .h(px(32.0))
        .px(px(12.0))
        .rounded(px(7.0))
        .border_1()
        .border_color(border)
        .bg(bg)
        .shadow_sm()
        .font_family(Metrics::FONT_SANS)
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(13.0))
        .text_color(fg)
        .cursor_pointer()
        .hover(move |style| style.opacity(0.9))
        .active(|style| style.opacity(0.8))
        .focus(move |style| style.border_color(theme.primary))
        .on_click(on_click)
        .child(icon(icon_path, fg, 14.0))
        .child(label)
        .child(
            div()
                .ml(px(2.0))
                .text_size(px(11.0))
                .text_color(fg.opacity(0.55))
                .child(shortcut),
        )
}

/// Quiet text action ("Clear") sized to sit inside a 30px section header.
pub fn text_button(
    id: &'static str,
    label: &'static str,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .tab_index(0)
        .focusable()
        .flex()
        .flex_none()
        .items_center()
        .h(px(22.0))
        .px(px(6.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(theme.primary.opacity(0.0))
        .font_family(Metrics::FONT_SANS)
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(10.5))
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
        .active(|style| style.opacity(0.8))
        .focus(move |style| style.border_color(theme.primary))
        .on_click(on_click)
        .child(label)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuEntry {
    Item { label: SharedString, enabled: bool },
    Separator,
}

impl ContextMenuEntry {
    pub fn item(label: impl Into<SharedString>) -> Self {
        Self::Item {
            label: label.into(),
            enabled: true,
        }
    }

    pub fn disabled(label: impl Into<SharedString>) -> Self {
        Self::Item {
            label: label.into(),
            enabled: false,
        }
    }

    fn is_enabled_item(&self) -> bool {
        matches!(self, Self::Item { enabled: true, .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuEvent {
    /// Index into the entries the menu was built with.
    Confirmed(usize),
    Dismissed,
}

/// A small anchored menu: Up/Down move, Enter confirms, Escape or an outside click dismisses.
/// Hosts render it with [`context_menu_layer`] so it floats above everything else.
pub struct ContextMenu {
    entries: Vec<ContextMenuEntry>,
    highlighted: Option<usize>,
    theme: Theme,
    focus_handle: FocusHandle,
}

impl EventEmitter<ContextMenuEvent> for ContextMenu {}

impl Focusable for ContextMenu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ContextMenu {
    pub fn new(
        entries: Vec<ContextMenuEntry>,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        Self {
            entries,
            highlighted: None,
            theme,
            focus_handle,
        }
    }

    pub fn entries(&self) -> &[ContextMenuEntry] {
        &self.entries
    }

    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    pub fn move_highlight(&mut self, step: isize, cx: &mut Context<Self>) {
        let enabled = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.is_enabled_item().then_some(index))
            .collect::<Vec<_>>();
        if enabled.is_empty() {
            return;
        }
        let next = match self
            .highlighted
            .and_then(|current| enabled.iter().position(|index| *index == current))
        {
            Some(position) => {
                enabled[(position as isize + step).rem_euclid(enabled.len() as isize) as usize]
            }
            None if step < 0 => enabled[enabled.len() - 1],
            None => enabled[0],
        };
        self.highlighted = Some(next);
        cx.notify();
    }

    pub fn confirm(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.highlighted
            && self
                .entries
                .get(index)
                .is_some_and(ContextMenuEntry::is_enabled_item)
        {
            cx.emit(ContextMenuEvent::Confirmed(index));
        }
    }

    fn set_highlight(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.highlighted != index {
            self.highlighted = index;
            cx.notify();
        }
    }
}

impl Render for ContextMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mut menu = div()
            .id("context-menu")
            .debug_selector(|| "context-menu".into())
            .key_context("ContextMenu")
            .track_focus(&self.focus_handle)
            .occlude()
            .flex()
            .flex_col()
            .min_w(px(Metrics::MENU_MIN_WIDTH))
            .p(px(4.0))
            .rounded(px(Metrics::RADIUS))
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface_raised)
            .shadow_lg()
            .font_family(Metrics::FONT_SANS)
            .text_size(px(12.0))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let modifiers = event.keystroke.modifiers;
                if modifiers.platform || modifiers.control || modifiers.alt {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "down" => this.move_highlight(1, cx),
                    "up" => this.move_highlight(-1, cx),
                    "enter" | "space" => this.confirm(cx),
                    "escape" => cx.emit(ContextMenuEvent::Dismissed),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(ContextMenuEvent::Dismissed)))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !hovered {
                    this.set_highlight(None, cx);
                }
            }));
        for (index, entry) in self.entries.iter().enumerate() {
            menu = menu.child(match entry {
                ContextMenuEntry::Separator => div()
                    .my(px(4.0))
                    .mx(px(4.0))
                    .h(px(1.0))
                    .flex_none()
                    .bg(theme.border_subtle)
                    .into_any_element(),
                ContextMenuEntry::Item { label, enabled } => {
                    let enabled = *enabled;
                    let highlighted = self.highlighted == Some(index);
                    div()
                        .id(("context-menu-item", index))
                        .debug_selector(move || format!("context-menu-item-{index}"))
                        .flex()
                        .flex_none()
                        .items_center()
                        .h(px(Metrics::MENU_ROW_HEIGHT))
                        .px(px(8.0))
                        .rounded(px(Metrics::MENU_ROW_RADIUS))
                        .bg(if highlighted {
                            theme.muted
                        } else {
                            theme.muted.opacity(0.0)
                        })
                        .text_color(if enabled {
                            theme.foreground
                        } else {
                            theme.muted_foreground.opacity(0.6)
                        })
                        .when(enabled, |row| {
                            row.cursor_pointer()
                                .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                    this.set_highlight(Some(index), cx);
                                }))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.highlighted = Some(index);
                                    this.confirm(cx);
                                }))
                        })
                        .child(div().truncate().child(label.clone()))
                        .into_any_element()
                }
            });
        }
        menu
    }
}

/// Floats a context menu at `position`, kept inside the window, above every other layer.
pub fn context_menu_layer(menu: Entity<ContextMenu>, position: Point<Pixels>) -> AnyElement {
    deferred(
        anchored()
            .position(position)
            .snap_to_window_with_margin(px(8.0))
            .child(menu),
    )
    .with_priority(2)
    .into_any_element()
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

        let mut visual = VisualTestContext::from_window(*window, cx);
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
