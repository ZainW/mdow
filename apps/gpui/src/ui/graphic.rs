//! Elements that paint rendered math and diagrams inside the reader.

use crate::graphics::{GraphicCache, GraphicKey, GraphicState, MATH_SCALE, render_math};
use gpui::{
    AnyElement, App, AvailableSpace, Bounds, Corners, Element, ElementId, Font, GlobalElementId,
    Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, RenderImage, Rgba, Style, TextLayout,
    Window, point, px, size,
};
use std::{ops::Range, sync::Arc};

/// The character inline math reserves its width with. It counts as a word character for line
/// wrapping, so a run of them never breaks across lines, and it is narrow, so the reserved slot
/// is within a fraction of an em of the formula's width.
pub(crate) const MATH_PLACEHOLDER: char = '.';

/// Straight RGBA bytes for a theme color.
pub(crate) fn rgba_bytes(color: Hsla) -> [u8; 4] {
    let Rgba { r, g, b, a } = color.to_rgb();
    [r, g, b, a].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// `#rrggbb` for a theme color.
pub(crate) fn hex_color(color: Hsla) -> String {
    let [r, g, b, _] = rgba_bytes(color);
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// The cache key for inline or display math typeset for text of `font_size`.
pub(crate) fn math_key(tex: &str, display: bool, color: Hsla, font_size: f32) -> GraphicKey {
    GraphicKey::Math {
        tex: tex.to_owned(),
        display,
        color: rgba_bytes(color),
        font_size: (font_size * MATH_SCALE * 100.0).round() as u32,
    }
}

/// Typesets math for text of `font_size`, synchronously on a cache miss. Formulas typeset in
/// well under a millisecond, so doing it inline keeps text layout stable from the first frame.
pub(crate) fn math_state(tex: &str, display: bool, color: Hsla, font_size: f32) -> GraphicState {
    let key = math_key(tex, display, color, font_size);
    GraphicCache::global().get_or_render(&key, || {
        render_math(tex, display, rgba_bytes(color), font_size * MATH_SCALE)
    })
}

/// A rendered image laid out at its logical size times `scale`, optionally shrinking to the
/// available width while keeping its aspect ratio.
pub(crate) struct GraphicElement {
    image: Arc<RenderImage>,
    width: f32,
    height: f32,
    fit_width: bool,
}

impl GraphicElement {
    pub(crate) fn new(image: Arc<RenderImage>, width: f32, height: f32, fit_width: bool) -> Self {
        Self {
            image,
            width,
            height,
            fit_width,
        }
    }
}

/// The size a graphic of `width` x `height` takes when at most `available` wide.
pub(crate) fn fitted_size(width: f32, height: f32, available: Option<f32>) -> (f32, f32) {
    match available {
        Some(available) if available > 0.0 && available < width => {
            (available, height * available / width)
        }
        _ => (width, height),
    }
}

impl Element for GraphicElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        _cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let (width, height, fit_width) = (self.width, self.height, self.fit_width);
        let style = Style {
            flex_shrink: 0.0,
            ..Style::default()
        };
        let layout_id =
            window.request_measured_layout(style, move |known, available, _window, _cx| {
                let available = known.width.map(f32::from).or(match available.width {
                    AvailableSpace::Definite(width) => Some(f32::from(width)),
                    AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
                });
                let (width, height) = if fit_width {
                    fitted_size(width, height, available)
                } else {
                    (width, height)
                };
                size(px(width), px(height))
            });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        window
            .paint_image(bounds, Corners::default(), self.image.clone(), 0, false)
            .ok();
    }
}

impl IntoElement for GraphicElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// One formula inside a paragraph: `range` covers its placeholder characters in the text.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InlineMathSlot {
    pub range: Range<usize>,
    pub tex: String,
    pub display: bool,
    pub color: Hsla,
}

/// Paints inline formulas over the transparent placeholder characters that reserve their room
/// in a paragraph's text layout, sitting each formula on the text baseline.
pub(crate) struct InlineMathText {
    text: AnyElement,
    layout: TextLayout,
    slots: Vec<InlineMathSlot>,
    placeholder_font: Font,
}

impl InlineMathText {
    pub(crate) fn new(
        text: AnyElement,
        layout: TextLayout,
        slots: Vec<InlineMathSlot>,
        placeholder_font: Font,
    ) -> Self {
        Self {
            text,
            layout,
            slots,
            placeholder_font,
        }
    }

    fn paint_slot(&self, slot: &InlineMathSlot, font_size: Pixels, window: &mut Window) {
        let GraphicState::Ready(graphic) =
            math_state(&slot.tex, slot.display, slot.color, f32::from(font_size))
        else {
            return;
        };
        let text_system = window.text_system();
        let advance = text_system
            .advance(
                text_system.resolve_font(&self.placeholder_font),
                font_size,
                MATH_PLACEHOLDER,
            )
            .map(|advance| advance.width)
            .unwrap_or(px(0.0));
        let placeholder_len = MATH_PLACEHOLDER.len_utf8();
        // The index just after the first placeholder is unambiguous even when the slot starts a
        // wrapped row (the slot's start index would report the end of the previous row).
        let Some(first) = self
            .layout
            .position_for_index(slot.range.start + placeholder_len)
        else {
            return;
        };
        let Some(line) = self
            .layout
            .line_layout_for_index(slot.range.start + placeholder_len)
        else {
            return;
        };
        let start_x = first.x - advance;
        let count = (slot.range.len() / placeholder_len) as f32;
        let end_x = self
            .layout
            .position_for_index(slot.range.end)
            .filter(|end| end.y == first.y)
            .map_or(start_x + advance * count, |end| end.x);
        let line_height = self.layout.line_height();
        let ascent = line.unwrapped_layout.ascent;
        let descent = line.unwrapped_layout.descent;
        let baseline = first.y + (line_height - ascent - descent) / 2.0 + ascent;
        let width = px(graphic.width);
        let height = px(graphic.height);
        let origin = point(
            start_x + ((end_x - start_x) - width).max(px(0.0)) / 2.0,
            baseline - (height - px(graphic.depth)),
        );
        window
            .paint_image(
                Bounds::new(origin, size(width, height)),
                Corners::default(),
                graphic.image,
                0,
                false,
            )
            .ok();
    }
}

impl Element for InlineMathText {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.text.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.text.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text.paint(window, cx);
        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        for slot in &self.slots {
            self.paint_slot(slot, font_size, window);
        }
    }
}

impl IntoElement for InlineMathText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::hsla;

    #[test]
    fn graphics_shrink_to_the_column_but_never_grow() {
        assert_eq!(fitted_size(800.0, 400.0, Some(400.0)), (400.0, 200.0));
        assert_eq!(fitted_size(300.0, 100.0, Some(400.0)), (300.0, 100.0));
        assert_eq!(fitted_size(300.0, 100.0, None), (300.0, 100.0));
    }

    #[test]
    fn converts_theme_colors_to_bytes_and_hex() {
        let red = hsla(0.0, 1.0, 0.5, 1.0);
        assert_eq!(rgba_bytes(red), [255, 0, 0, 255]);
        assert_eq!(hex_color(red), "#ff0000");
    }

    #[test]
    fn math_keys_track_size_and_color() {
        let ink = hsla(0.0, 0.0, 0.1, 1.0);
        assert_ne!(
            math_key("x", false, ink, 16.0),
            math_key("x", false, ink, 18.0)
        );
        assert_ne!(
            math_key("x", false, ink, 16.0),
            math_key("x", false, hsla(0.6, 0.6, 0.5, 1.0), 16.0)
        );
        assert!(matches!(
            math_state(r"\alpha", false, ink, 16.0),
            GraphicState::Ready(_)
        ));
        assert!(matches!(
            math_state(r"\frac{", false, ink, 16.0),
            GraphicState::Failed(_)
        ));
    }
}
