//! TeX math through RaTeX: parse, lay out with KaTeX metrics, emit glyph outlines as SVG.

use super::{Graphic, guarded, rasterize_svg};
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_svg::{SvgOptions, render_to_svg};
use ratex_types::{color::Color, display_item::DisplayList, math_style::MathStyle};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

/// KaTeX sets math at 1.21em of the surrounding text so its x-height matches body fonts.
pub const MATH_SCALE: f32 = 1.21;

/// Typesets `tex` at `font_size` logical pixels per em in `color` (straight RGBA).
///
/// Display math uses TeX display style (large operators, limits above and below); inline math
/// uses text style. The returned graphic's `depth` is how far the formula hangs below its
/// baseline, so inline math can sit on the surrounding text's baseline.
pub fn render_math(
    tex: &str,
    display: bool,
    color: [u8; 4],
    font_size: f32,
) -> Result<Graphic, String> {
    guarded(|| {
        let display_list = layout_math(tex, display, color)?;
        let em = f64::from(font_size);
        let width = (display_list.width * em) as f32;
        let height = ((display_list.height + display_list.depth) * em) as f32;
        let svg = render_to_svg(
            &display_list,
            &SvgOptions {
                font_size: em,
                padding: 0.0,
                embed_glyphs: true,
                ..SvgOptions::default()
            },
        );
        let mut graphic = rasterize_svg(&svg, None, Some((width, height)), 1.0)?;
        graphic.depth = (display_list.depth * em) as f32;
        Ok(graphic)
    })
}

/// The typeset width of `tex` in ems of the math font, or `None` when it does not typeset.
///
/// The reader reserves inline space for a formula before it knows the paragraph's font size;
/// widths scale linearly with size, so one em measurement serves every zoom level.
pub fn math_width_em(tex: &str, display: bool) -> Option<f32> {
    type Widths = HashMap<(String, bool), Option<f32>>;
    static WIDTHS: LazyLock<Mutex<Widths>> = LazyLock::new(Mutex::default);
    let key = (tex.to_owned(), display);
    if let Some(width) = WIDTHS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
    {
        return *width;
    }
    let width = guarded(|| layout_math(tex, display, [0, 0, 0, 255]))
        .ok()
        .map(|display_list| display_list.width as f32)
        .filter(|width| width.is_finite() && *width > 0.0);
    let mut widths = WIDTHS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if widths.len() >= 8192 {
        widths.clear();
    }
    widths.insert(key, width);
    width
}

fn layout_math(tex: &str, display: bool, color: [u8; 4]) -> Result<DisplayList, String> {
    let nodes = ratex_parser::parse(tex).map_err(|error| error.message)?;
    let options = LayoutOptions {
        style: if display {
            MathStyle::Display
        } else {
            MathStyle::Text
        },
        color: Color::new(
            f32::from(color[0]) / 255.0,
            f32::from(color[1]) / 255.0,
            f32::from(color[2]) / 255.0,
            f32::from(color[3]) / 255.0,
        ),
        ..LayoutOptions::default()
    };
    Ok(to_display_list(&layout(&nodes, &options)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const INK: [u8; 4] = [20, 20, 20, 255];

    #[test]
    fn inline_math_reports_its_size_and_depth_below_the_baseline() {
        let graphic = render_math(r"\frac{n(n+1)}{2}", false, INK, 16.0).unwrap();

        assert!(graphic.width > 20.0 && graphic.width < 80.0, "{graphic:?}");
        assert!(graphic.height > 16.0, "{graphic:?}");
        assert!(
            graphic.depth > 0.0 && graphic.depth < graphic.height,
            "{graphic:?}"
        );
    }

    #[test]
    fn display_style_sets_operators_larger_than_text_style() {
        let text = render_math(r"\sum_{k=1}^{n} k", false, INK, 16.0).unwrap();
        let display = render_math(r"\sum_{k=1}^{n} k", true, INK, 16.0).unwrap();

        assert!(display.height > text.height, "{text:?} vs {display:?}");
    }

    #[test]
    fn environments_from_the_fixture_typeset() {
        for tex in [
            r"\begin{aligned} a &= b \\ c &= d \end{aligned}",
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
            r"f(x) = \begin{cases} x^2 & x \ge 0 \\ -x & x < 0 \end{cases}",
            r"\nabla \times \mathbf{B} = \mu_0 \varepsilon_0 \frac{\partial \mathbf{E}}{\partial t}",
        ] {
            assert!(render_math(tex, true, INK, 16.0).is_ok(), "{tex}");
        }
    }

    #[test]
    fn width_in_ems_matches_the_rendered_width_at_any_size() {
        let em = math_width_em(r"x_i^2 + y_i^2", false).unwrap();
        for size in [12.0, 16.0, 24.0] {
            let graphic = render_math(r"x_i^2 + y_i^2", false, INK, size).unwrap();
            assert!((graphic.width - em * size).abs() < 0.01, "{size}");
        }
        assert_eq!(math_width_em(r"\frac{1}{", false), None);
    }

    #[test]
    fn broken_tex_fails_instead_of_rendering_garbage() {
        assert!(render_math(r"\frac{1}{", false, INK, 16.0).is_err());
        assert!(render_math(r"\left( \begin{matrix} 1 \end{matrix}", true, INK, 16.0).is_err());
    }
}
