//! Label measurement for Mermaid layout that matches the font the diagrams are drawn in.
//!
//! Merman's built-in measurer models proportional browser fonts, so labels drawn in a monospace
//! face overflow their boxes. This measurer reads real advances from the bundled font instead.

use merman::svg::{DeterministicTextMeasurer, TextMeasurer, TextMetrics, TextStyle, WrapMode};
use std::collections::HashMap;

pub(crate) struct FontMeasurer {
    advances: HashMap<char, f32>,
    units_per_em: f32,
    fallback: DeterministicTextMeasurer,
}

impl FontMeasurer {
    pub(crate) fn new(font: &[u8]) -> Self {
        let mut advances = HashMap::new();
        let mut units_per_em = 1000.0;
        if let Ok(face) = ttf_parser::Face::parse(font, 0) {
            units_per_em = f32::from(face.units_per_em());
            if let Some(subtables) = face.tables().cmap.map(|cmap| cmap.subtables) {
                for subtable in subtables.into_iter().filter(|table| table.is_unicode()) {
                    subtable.codepoints(|codepoint| {
                        let Some(character) = char::from_u32(codepoint) else {
                            return;
                        };
                        if let Some(advance) = face
                            .glyph_index(character)
                            .and_then(|glyph| face.glyph_hor_advance(glyph))
                        {
                            advances.insert(character, f32::from(advance));
                        }
                    });
                }
            }
        }
        Self {
            advances,
            units_per_em,
            fallback: DeterministicTextMeasurer::default(),
        }
    }

    /// Advance of one character in em. Characters the font lacks fall back to another face at
    /// draw time; wide scripts get a full em, everything else the monospace cell.
    fn advance_em(&self, character: char) -> f32 {
        match self.advances.get(&character) {
            Some(advance) => advance / self.units_per_em,
            None if is_wide(character) => 1.0,
            None => 0.6,
        }
    }

    fn line_width(&self, line: &str, font_size: f64) -> f64 {
        f64::from(
            line.chars()
                .map(|character| self.advance_em(character))
                .sum::<f32>(),
        ) * font_size
    }

    fn lines(&self, text: &str, font_size: f64, max_width: Option<f64>) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in split_label_lines(text) {
            let Some(max_width) = max_width.filter(|width| *width > 0.0) else {
                lines.push(paragraph.to_owned());
                continue;
            };
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && self.line_width(&candidate, font_size) > max_width {
                    lines.push(std::mem::replace(&mut line, word.to_owned()));
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        lines
    }
}

impl TextMeasurer for FontMeasurer {
    fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
        self.measure_wrapped(text, style, None, WrapMode::SvgLike)
    }

    fn measure_wrapped(
        &self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f64>,
        _wrap_mode: WrapMode,
    ) -> TextMetrics {
        let lines = self.lines(text, style.font_size, max_width);
        let width = lines
            .iter()
            .map(|line| self.line_width(line, style.font_size))
            .fold(0.0, f64::max);
        let line_height = self.fallback.measure("M", style).height;
        TextMetrics {
            width,
            height: line_height * lines.len() as f64,
            line_count: lines.len(),
        }
    }
}

/// Mermaid labels break lines on newlines and `<br>` tags in any of their spellings.
fn split_label_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut rest = text;
    loop {
        let lower = rest.to_ascii_lowercase();
        let tag = lower
            .find("<br")
            .and_then(|start| lower[start..].find('>').map(|end| (start, start + end + 1)));
        let newline = rest.find('\n').map(|start| (start, start + 1));
        let next = match (tag, newline) {
            (Some(tag), Some(newline)) => Some(if tag.0 < newline.0 { tag } else { newline }),
            (tag, newline) => tag.or(newline),
        };
        let Some((start, end)) = next else {
            lines.push(rest);
            return lines;
        };
        lines.push(&rest[..start]);
        rest = &rest[end..];
    }
}

fn is_wide(character: char) -> bool {
    matches!(
        u32::from(character),
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
            | 0x20000..=0x3FFFD
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::mermaid::DIAGRAM_FONT;

    fn style() -> TextStyle {
        TextStyle {
            font_size: 10.0,
            ..TextStyle::default()
        }
    }

    #[test]
    fn measures_monospace_labels_by_their_real_advance() {
        let measurer = FontMeasurer::new(DIAGRAM_FONT);
        let one = measurer.measure("M", &style()).width;
        let ten = measurer.measure("iiiiiiiiii", &style()).width;

        assert!(one > 4.0 && one < 8.0, "{one}");
        assert!(
            (ten - one * 10.0).abs() < 0.01,
            "monospace: {one} x10 != {ten}"
        );
    }

    #[test]
    fn wraps_at_word_boundaries_and_breaks() {
        let measurer = FontMeasurer::new(DIAGRAM_FONT);
        let single = measurer.measure("alpha", &style());
        let wrapped = measurer.measure_wrapped(
            "alpha beta gamma",
            &style(),
            Some(single.width * 1.5),
            WrapMode::SvgLike,
        );

        assert_eq!(wrapped.line_count, 3);
        assert!((wrapped.height - single.height * 3.0).abs() < 0.01);
        assert_eq!(
            split_label_lines("a<br/>b<BR>c\nd"),
            vec!["a", "b", "c", "d"]
        );
    }
}
