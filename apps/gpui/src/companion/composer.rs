//! The companion's multiline prompt input.
//!
//! `ui::field::Field` is a single shaped line; the composer needs wrapped, growing text with
//! newlines, so it lives here and reuses Field's editing actions (arrow keys, clipboard, the
//! Edit menu) through the shared "Field" key context. Enter and ⌘Enter send, ⇧Enter (or ⌥Enter)
//! inserts a newline, Escape hands focus back to the reader.

use crate::theme::{Metrics, Theme};
use crate::ui::field::{
    Backspace, Cancel, Copy, Cut, Delete, End, Home, MoveLeft, MoveRight, Paste, SelectAll,
    SelectLeft, SelectRight,
};
use gpui::{
    App, AvailableSpace, Bounds, ClipboardItem, ContentMask, CursorStyle, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable,
    GlobalElementId, InspectorElementId, KeyBinding, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, SharedString, Size, Style, TextAlign,
    TextRun, UTF16Selection, UnderlineStyle, Window, WrappedLine, div, fill, point, prelude::*, px,
    relative, size,
};
use std::ops::Range;

pub const FONT_SIZE: f32 = 13.0;
pub const LINE_HEIGHT: f32 = 20.0;
pub const MAX_VISIBLE_LINES: usize = 8;
pub const KEY_CONTEXT: &str = "Composer";

gpui::actions!(
    companion_composer,
    [
        Send,
        Newline,
        MoveUp,
        MoveDown,
        SelectUp,
        SelectDown,
        MoveToStart,
        MoveToEnd
    ]
);

/// Bindings layered over the "Field" ones; registered later so they win at the same depth.
pub fn key_bindings() -> Vec<KeyBinding> {
    let context = Some(KEY_CONTEXT);
    vec![
        KeyBinding::new("enter", Send, context),
        KeyBinding::new("cmd-enter", Send, context),
        KeyBinding::new("ctrl-enter", Send, context),
        KeyBinding::new("shift-enter", Newline, context),
        KeyBinding::new("alt-enter", Newline, context),
        KeyBinding::new("up", MoveUp, context),
        KeyBinding::new("down", MoveDown, context),
        KeyBinding::new("shift-up", SelectUp, context),
        KeyBinding::new("shift-down", SelectDown, context),
        KeyBinding::new("cmd-left", Home, context),
        KeyBinding::new("cmd-right", End, context),
        KeyBinding::new("cmd-up", MoveToStart, context),
        KeyBinding::new("cmd-down", MoveToEnd, context),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerEvent {
    Edited,
    Submitted,
    Cancelled,
}

/// One hard line (between newlines), shaped and wrapped at the composer width.
struct LineBox {
    start: usize,
    top: Pixels,
    line: WrappedLine,
    rows: usize,
}

pub struct Composer {
    theme: Theme,
    content: String,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    lines: Vec<LineBox>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_y: Pixels,
    is_selecting: bool,
    focus_handle: FocusHandle,
}

impl EventEmitter<ComposerEvent> for Composer {}

impl Composer {
    pub fn new(placeholder: impl Into<SharedString>, theme: Theme, cx: &mut Context<Self>) -> Self {
        Self {
            theme,
            content: String::new(),
            placeholder: placeholder.into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            lines: Vec::new(),
            last_bounds: None,
            scroll_y: px(0.0),
            is_selecting: false,
            focus_handle: cx.focus_handle().tab_index(0).tab_stop(true),
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let text = text.into();
        if text == self.content {
            return;
        }
        let end = text.len();
        self.content = text;
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.emit(ComposerEvent::Edited);
        cx.notify();
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus_handle.is_focused(window)
    }

    pub fn focus(&self, window: &mut Window) {
        self.focus_handle.focus(window);
    }

    pub fn cursor(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = offset.min(self.content.len());
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = offset.min(self.content.len());
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content[..offset]
            .chars()
            .next_back()
            .map(|ch| offset - ch.len_utf8())
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content[offset..]
            .chars()
            .next()
            .map(|ch| offset + ch.len_utf8())
            .unwrap_or(self.content.len())
    }

    fn line_start(&self, offset: usize) -> usize {
        self.content[..offset]
            .rfind('\n')
            .map_or(0, |index| index + 1)
    }

    fn line_end(&self, offset: usize) -> usize {
        self.content[offset..]
            .find('\n')
            .map_or(self.content.len(), |index| offset + index)
    }

    fn replace(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        let changed = &self.content[range.clone()] != text;
        self.content.replace_range(range.clone(), text);
        let cursor = range.start + text.len();
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.marked_range = None;
        if changed {
            cx.emit(ComposerEvent::Edited);
        }
        cx.notify();
    }

    pub fn insert(&mut self, text: &str, cx: &mut Context<Self>) {
        let range = self
            .marked_range
            .clone()
            .unwrap_or_else(|| self.selected_range.clone());
        self.replace(range, text, cx);
    }

    fn send(&mut self, _: &Send, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(ComposerEvent::Submitted);
    }

    fn newline(&mut self, _: &Newline, _: &mut Window, cx: &mut Context<Self>) {
        self.insert("\n", cx);
    }

    fn cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(ComposerEvent::Cancelled);
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let start = self.previous_boundary(self.cursor());
            self.selected_range = start..self.selected_range.end;
        }
        self.insert("", cx);
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let end = self.next_boundary(self.cursor());
            self.selected_range = self.selected_range.start..end;
        }
        self.insert("", cx);
    }

    fn move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.cursor()), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_start(self.cursor()), cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_end(self.cursor()), cx);
    }

    fn move_to_start(&mut self, _: &MoveToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn move_to_end(&mut self, _: &MoveToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    /// The offset one visual row above or below the cursor.
    fn vertical_target(&self, rows: f32) -> usize {
        let cursor = self.cursor();
        let Some(position) = self.position_for_index(cursor) else {
            // Not laid out yet: fall back to hard-line starts and ends.
            return if rows < 0.0 {
                self.line_start(cursor).saturating_sub(1).min(cursor)
            } else {
                (self.line_end(cursor) + 1).min(self.content.len())
            };
        };
        let target = point(
            position.x,
            position.y + px(LINE_HEIGHT * rows + LINE_HEIGHT / 2.0),
        );
        if target.y < px(0.0) {
            return 0;
        }
        self.index_for_position(target)
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        let target = self.vertical_target(-1.0);
        self.move_to(target, cx);
    }

    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        let target = self.vertical_target(1.0);
        self.move_to(target, cx);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let target = self.vertical_target(-1.0);
        self.select_to(target, cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let target = self.vertical_target(1.0);
        self.select_to(target, cx);
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.insert(&text.replace("\r\n", "\n"), cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_owned(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_owned(),
            ));
            self.insert("", cx);
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window);
        self.is_selecting = true;
        let index = self.index_for_window_point(event.position);
        if event.modifiers.shift {
            self.select_to(index, cx);
        } else {
            self.move_to(index, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let index = self.index_for_window_point(event.position);
            self.select_to(index, cx);
        }
    }

    fn index_for_window_point(&self, position: Point<Pixels>) -> usize {
        let Some(bounds) = self.last_bounds else {
            return self.content.len();
        };
        let local = point(
            position.x - bounds.left(),
            position.y - bounds.top() + self.scroll_y,
        );
        self.index_for_position(local)
    }

    /// Content-space position (top-left of the caret) for a byte offset.
    fn position_for_index(&self, index: usize) -> Option<Point<Pixels>> {
        let line = self.lines.iter().rev().find(|line| line.start <= index)?;
        let local = index.checked_sub(line.start)?;
        if local > line.line.len() {
            return None;
        }
        line.line
            .position_for_index(local, px(LINE_HEIGHT))
            .map(|position| point(position.x, position.y + line.top))
    }

    fn index_for_position(&self, position: Point<Pixels>) -> usize {
        let Some(line) = self
            .lines
            .iter()
            .rev()
            .find(|line| line.top <= position.y)
            .or(self.lines.first())
        else {
            return self.content.len();
        };
        let bottom = line.top + px(LINE_HEIGHT * line.rows as f32);
        if position.y >= bottom && std::ptr::eq(line, self.lines.last().expect("non-empty")) {
            return self.content.len();
        }
        let local = point(
            position.x.max(px(0.0)),
            (position.y - line.top).max(px(0.0)),
        );
        let index = match line.line.closest_index_for_position(local, px(LINE_HEIGHT)) {
            Ok(index) | Err(index) => index,
        };
        (line.start + index).min(self.content.len())
    }

    fn utf16_to_offset(&self, offset: usize) -> usize {
        let mut utf8 = 0;
        let mut utf16 = 0;
        for ch in self.content.chars() {
            if utf16 >= offset {
                break;
            }
            utf16 += ch.len_utf16();
            utf8 += ch.len_utf8();
        }
        utf8
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        self.content[..offset.min(self.content.len())]
            .chars()
            .map(char::len_utf16)
            .sum()
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.utf16_to_offset(range.start)..self.utf16_to_offset(range.end)
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }
}

impl Focusable for Composer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for Composer {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_owned())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        self.replace(range, text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        new_selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        self.content.replace_range(range.clone(), text);
        self.marked_range = (!text.is_empty()).then(|| range.start..range.start + text.len());
        self.selected_range = new_selected
            .as_ref()
            .map(|selected| {
                let selected = self.range_from_utf16(selected);
                range.start + selected.start..range.start + selected.end
            })
            .unwrap_or_else(|| range.start + text.len()..range.start + text.len());
        cx.emit(ComposerEvent::Edited);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range);
        let start = self.position_for_index(range.start)?;
        let end = self.position_for_index(range.end).unwrap_or(start);
        let origin = point(bounds.left(), bounds.top() - self.scroll_y);
        Some(Bounds::from_corners(
            origin + start,
            origin + point(end.x, end.y + px(LINE_HEIGHT)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let index = self.index_for_window_point(position);
        Some(self.offset_to_utf16(index))
    }
}

impl Render for Composer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("companion-composer-input")
            .debug_selector(|| "companion-composer-input".into())
            .key_context(gpui::KeyContext::parse("Field Composer").expect("valid key context"))
            .track_focus(&self.focus_handle)
            .w_full()
            .cursor(CursorStyle::IBeam)
            .font_family(Metrics::FONT_SANS)
            .text_size(px(FONT_SIZE))
            .line_height(px(LINE_HEIGHT))
            .text_color(self.theme.foreground)
            .on_action(cx.listener(Self::send))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::move_to_start))
            .on_action(cx.listener(Self::move_to_end))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(ComposerElement {
                composer: cx.entity(),
            })
    }
}

struct ComposerElement {
    composer: Entity<Composer>,
}

impl IntoElement for ComposerElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct Prepaint {
    selections: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    placeholder: Option<WrappedLine>,
}

fn shape_lines(
    content: &str,
    marked: Option<&Range<usize>>,
    width: Pixels,
    theme: Theme,
    window: &mut Window,
) -> Vec<LineBox> {
    let font_size = px(FONT_SIZE);
    let base_font = gpui::font(Metrics::FONT_SANS);
    let mut lines = Vec::new();
    let mut top = px(0.0);
    let mut start = 0;
    for text in content.split('\n') {
        let end = start + text.len();
        let run = |len: usize, underline: bool| TextRun {
            len,
            font: base_font.clone(),
            color: theme.foreground,
            background_color: None,
            underline: underline.then_some(UnderlineStyle {
                color: Some(theme.foreground),
                thickness: px(1.0),
                wavy: false,
            }),
            strikethrough: None,
        };
        let runs = match marked {
            Some(marked) if marked.start < end && marked.end > start => {
                let mark_start = marked.start.max(start) - start;
                let mark_end = marked.end.min(end) - start;
                vec![
                    run(mark_start, false),
                    run(mark_end - mark_start, true),
                    run(text.len() - mark_end, false),
                ]
            }
            _ => vec![run(text.len(), false)],
        };
        let shaped = window
            .text_system()
            .shape_text(
                SharedString::from(text.to_owned()),
                font_size,
                &runs,
                Some(width),
                None,
            )
            .ok()
            .and_then(|mut shaped| (!shaped.is_empty()).then(|| shaped.remove(0)))
            .unwrap_or_default();
        let rows = shaped.wrap_boundaries().len() + 1;
        lines.push(LineBox {
            start,
            top,
            line: shaped,
            rows,
        });
        top += px(LINE_HEIGHT * rows as f32);
        start = end + 1;
    }
    lines
}

fn content_rows(lines: &[LineBox]) -> usize {
    lines.iter().map(|line| line.rows).sum::<usize>().max(1)
}

impl Element for ComposerElement {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let composer = self.composer.clone();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let layout_id =
            window.request_measured_layout(style, move |known, available, window, cx| {
                let width = known.width.unwrap_or(match available.width {
                    AvailableSpace::Definite(width) => width,
                    _ => px(320.0),
                });
                let (content, theme) = {
                    let composer = composer.read(cx);
                    (composer.content.clone(), composer.theme)
                };
                let lines = shape_lines(&content, None, width, theme, window);
                let rows = content_rows(&lines).min(MAX_VISIBLE_LINES);
                Size {
                    width,
                    height: px(LINE_HEIGHT * rows as f32),
                }
            });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let (content, marked, theme, placeholder, selected, cursor_index) = {
            let composer = self.composer.read(cx);
            (
                composer.content.clone(),
                composer.marked_range.clone(),
                composer.theme,
                composer.placeholder.clone(),
                composer.selected_range.clone(),
                composer.cursor(),
            )
        };
        let lines = shape_lines(&content, marked.as_ref(), bounds.size.width, theme, window);
        let placeholder = content.is_empty().then(|| {
            window
                .text_system()
                .shape_text(
                    placeholder,
                    px(FONT_SIZE),
                    &[TextRun {
                        len: self.composer.read(cx).placeholder.len(),
                        font: gpui::font(Metrics::FONT_SANS),
                        color: theme.muted_foreground,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    Some(bounds.size.width),
                    Some(1),
                )
                .ok()
                .and_then(|mut shaped| (!shaped.is_empty()).then(|| shaped.remove(0)))
                .unwrap_or_default()
        });

        self.composer.update(cx, |composer, _| {
            composer.lines = lines;
            composer.last_bounds = Some(bounds);
            // Keep the caret row inside the visible rows.
            if let Some(caret) = composer.position_for_index(cursor_index) {
                let visible = bounds.size.height;
                if caret.y < composer.scroll_y {
                    composer.scroll_y = caret.y;
                } else if caret.y + px(LINE_HEIGHT) > composer.scroll_y + visible {
                    composer.scroll_y = caret.y + px(LINE_HEIGHT) - visible;
                }
                let max_scroll =
                    (px(LINE_HEIGHT * content_rows(&composer.lines) as f32) - visible).max(px(0.0));
                composer.scroll_y = composer.scroll_y.clamp(px(0.0), max_scroll);
            }
        });

        let composer = self.composer.read(cx);
        let origin = point(bounds.left(), bounds.top() - composer.scroll_y);
        let mut selections = Vec::new();
        if !selected.is_empty() {
            for line in &composer.lines {
                let line_end = line.start + line.line.len();
                if selected.end < line.start || selected.start > line_end {
                    continue;
                }
                for row in 0..line.rows {
                    let row_top = px(LINE_HEIGHT * row as f32);
                    let row_start = if row == 0 {
                        0
                    } else {
                        match line.line.closest_index_for_position(
                            point(px(0.0), row_top + px(1.0)),
                            px(LINE_HEIGHT),
                        ) {
                            Ok(index) | Err(index) => index,
                        }
                    };
                    let row_end = if row + 1 == line.rows {
                        line.line.len()
                    } else {
                        match line.line.closest_index_for_position(
                            point(px(0.0), row_top + px(LINE_HEIGHT + 1.0)),
                            px(LINE_HEIGHT),
                        ) {
                            Ok(index) | Err(index) => index,
                        }
                    };
                    let start = selected.start.max(line.start + row_start);
                    let end = selected.end.min(line.start + row_end);
                    let continues = selected.end > line_end && row + 1 == line.rows;
                    if start > end || (start == end && !continues) {
                        continue;
                    }
                    let x_start = if start == line.start + row_start {
                        px(0.0)
                    } else {
                        line.line
                            .position_for_index(start - line.start, px(LINE_HEIGHT))
                            .map_or(px(0.0), |position| position.x)
                    };
                    let mut x_end = line
                        .line
                        .position_for_index(end - line.start, px(LINE_HEIGHT))
                        .map_or(x_start, |position| position.x);
                    if continues {
                        x_end += px(4.0);
                    }
                    let top = origin.y + line.top + row_top;
                    selections.push(fill(
                        Bounds::from_corners(
                            point(origin.x + x_start, top),
                            point(origin.x + x_end, top + px(LINE_HEIGHT)),
                        ),
                        theme.primary.opacity(0.24),
                    ));
                }
            }
        }
        let cursor = selected.is_empty().then(|| {
            let caret = composer
                .position_for_index(cursor_index)
                .unwrap_or_default();
            fill(
                Bounds::new(
                    point(origin.x + caret.x, origin.y + caret.y + px(2.0)),
                    size(px(1.5), px(LINE_HEIGHT - 4.0)),
                ),
                theme.primary,
            )
        });
        Prepaint {
            selections,
            cursor,
            placeholder,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.composer.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.composer.clone()),
            cx,
        );
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for quad in prepaint.selections.drain(..) {
                window.paint_quad(quad);
            }
            if let Some(placeholder) = prepaint.placeholder.take() {
                let _ = placeholder.paint(
                    bounds.origin,
                    px(LINE_HEIGHT),
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
            let composer = self.composer.read(cx);
            let origin = point(bounds.left(), bounds.top() - composer.scroll_y);
            let lines = composer
                .lines
                .iter()
                .map(|line| (line.top, line.line.clone()))
                .collect::<Vec<_>>();
            for (top, line) in lines {
                let _ = line.paint(
                    point(origin.x, origin.y + top),
                    px(LINE_HEIGHT),
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
            if focus_handle.is_focused(window)
                && let Some(cursor) = prepaint.cursor.take()
            {
                window.paint_quad(cursor);
            }
        });
    }
}

#[cfg(test)]
/// The "Field" bindings from `main.rs` this composer relies on, then its own.
pub(crate) fn test_bindings() -> Vec<KeyBinding> {
    let field = Some("Field");
    let mut bindings = vec![
        KeyBinding::new("left", MoveLeft, field),
        KeyBinding::new("right", MoveRight, field),
        KeyBinding::new("backspace", Backspace, field),
        KeyBinding::new("cmd-a", SelectAll, field),
        KeyBinding::new("enter", crate::ui::field::Submit, field),
        KeyBinding::new("shift-enter", crate::ui::field::SubmitBackward, field),
        KeyBinding::new("escape", Cancel, field),
    ];
    bindings.extend(super::key_bindings());
    bindings.push(KeyBinding::new("escape", crate::actions::Dismiss, None));
    bindings
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext, WindowAppearance};

    struct Harness {
        composer: Entity<Composer>,
        events: Vec<ComposerEvent>,
        _subscription: gpui::Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(240.0)).child(self.composer.clone())
        }
    }

    fn harness(cx: &mut TestAppContext) -> (Entity<Harness>, &mut VisualTestContext) {
        cx.update(|cx| {
            cx.bind_keys(test_bindings());
        });
        let (harness, visual) = cx.add_window_view(|window, cx| {
            let composer = cx
                .new(|cx| Composer::new("Ask", Theme::for_appearance(WindowAppearance::Dark), cx));
            composer.read(cx).focus(window);
            let subscription = cx.subscribe(&composer, |this: &mut Harness, _, event, _| {
                this.events.push(*event);
            });
            Harness {
                composer,
                events: Vec::new(),
                _subscription: subscription,
            }
        });
        (harness, visual)
    }

    #[gpui::test]
    fn enter_sends_shift_enter_inserts_a_newline_and_cmd_enter_sends(cx: &mut TestAppContext) {
        let (harness, visual) = harness(cx);
        visual.simulate_input("first");
        visual.simulate_keystrokes("shift-enter");
        visual.simulate_input("second");
        let composer = harness.read_with(visual, |harness, _| harness.composer.clone());
        assert_eq!(
            composer.read_with(visual, |c, _| c.text().to_owned()),
            "first\nsecond"
        );
        visual.simulate_keystrokes("enter");
        visual.simulate_keystrokes("cmd-enter");
        let submits = harness.read_with(visual, |harness, _| {
            harness
                .events
                .iter()
                .filter(|event| **event == ComposerEvent::Submitted)
                .count()
        });
        assert_eq!(submits, 2);
        assert_eq!(
            composer.read_with(visual, |c, _| c.text().to_owned()),
            "first\nsecond"
        );
    }

    #[gpui::test]
    fn editing_keys_and_vertical_movement(cx: &mut TestAppContext) {
        let (harness, visual) = harness(cx);
        let composer = harness.read_with(visual, |harness, _| harness.composer.clone());
        visual.simulate_input("ab");
        visual.simulate_keystrokes("shift-enter");
        visual.simulate_input("cd");
        visual.update(|window, cx| window.draw(cx).clear());
        visual.simulate_keystrokes("up");
        visual.simulate_input("X");
        assert_eq!(
            composer.read_with(visual, |c, _| c.text().to_owned()),
            "abX\ncd"
        );
        visual.simulate_keystrokes("cmd-down backspace");
        assert_eq!(
            composer.read_with(visual, |c, _| c.text().to_owned()),
            "abX\nc"
        );
        visual.simulate_keystrokes("cmd-a backspace");
        assert_eq!(composer.read_with(visual, |c, _| c.text().to_owned()), "");
        visual.simulate_keystrokes("escape");
        assert!(harness.read_with(visual, |h, _| h.events.contains(&ComposerEvent::Cancelled)));
    }

    #[gpui::test]
    fn grows_with_its_content_up_to_a_limit(cx: &mut TestAppContext) {
        let (harness, visual) = harness(cx);
        let composer = harness.read_with(visual, |harness, _| harness.composer.clone());
        visual.update(|window, cx| window.draw(cx).clear());
        let one = visual
            .debug_bounds("companion-composer-input")
            .unwrap()
            .size
            .height;
        composer.update(visual, |c, cx| c.set_text("1\n2\n3", cx));
        visual.update(|window, cx| window.draw(cx).clear());
        let three = visual
            .debug_bounds("companion-composer-input")
            .unwrap()
            .size
            .height;
        assert_eq!(three, one * 3.0);
        composer.update(visual, |c, cx| c.set_text("x\n".repeat(20), cx));
        visual.update(|window, cx| window.draw(cx).clear());
        let many = visual
            .debug_bounds("companion-composer-input")
            .unwrap()
            .size
            .height;
        assert_eq!(many, one * MAX_VISIBLE_LINES as f32);
    }
}
