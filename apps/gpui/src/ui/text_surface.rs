//! The reader's text surfaces: every painted `StyledText` (a paragraph, heading, table cell, code
//! block, ...) gets a stable [`SurfaceId`] — its top-level block plus its order within that block.
//! Find matches and the selection are stored against those ids and byte offsets, never pixels, so
//! they survive scrolling and list virtualization. Geometry (pixel rects, hit testing) is only
//! derived from the `TextLayout`s of surfaces that were painted in the last frame.

use gpui::{
    Bounds, Hsla, Pixels, Point, TextAlign, TextLayout, TextRun, WrappedLineLayout, point, px, size,
};
use std::{ops::Range, sync::Arc};

/// One painted text surface: the `surface`-th text surface (in document order) of top-level
/// block `block`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceId {
    pub block: usize,
    pub surface: usize,
}

impl SurfaceId {
    pub const fn new(block: usize, surface: usize) -> Self {
        Self { block, surface }
    }
}

/// A caret position: a byte offset into a surface's painted text. Orders in document order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPoint {
    pub block: usize,
    pub surface: usize,
    pub offset: usize,
}

impl TextPoint {
    pub const fn new(id: SurfaceId, offset: usize) -> Self {
        Self {
            block: id.block,
            surface: id.surface,
            offset,
        }
    }

    pub const fn id(self) -> SurfaceId {
        SurfaceId::new(self.block, self.surface)
    }

    /// A point after every surface of a document with `block_count` blocks.
    pub const fn document_end(block_count: usize) -> Self {
        Self {
            block: block_count,
            surface: 0,
            offset: 0,
        }
    }
}

/// An anchored selection; `head` moves while dragging or shift-clicking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: TextPoint,
    pub head: TextPoint,
}

impl TextSelection {
    pub const fn collapsed(point: TextPoint) -> Self {
        Self {
            anchor: point,
            head: point,
        }
    }

    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }

    pub fn ordered(self) -> (TextPoint, TextPoint) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }
}

/// The byte range of `id` (whose painted text is `len` bytes) covered by `start..end`.
pub fn selected_range(
    start: TextPoint,
    end: TextPoint,
    id: SurfaceId,
    len: usize,
) -> Option<Range<usize>> {
    if id < start.id() || id > end.id() {
        return None;
    }
    let lo = if id == start.id() {
        start.offset.min(len)
    } else {
        0
    };
    let hi = if id == end.id() {
        end.offset.min(len)
    } else {
        len
    };
    (lo < hi).then_some(lo..hi)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    /// Wrapped inline text: paragraphs, headings, list items, table cells.
    Inline,
    /// A code block: triple-click selects a line.
    Code,
}

/// How a surface joins the one before it (inside the same top-level block) when copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceSeparator {
    Newline,
    Tab,
}

/// The painted text of one surface, plus what copying it adds around the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceText {
    pub text: String,
    pub kind: SurfaceKind,
    pub separator: SurfaceSeparator,
    /// List marker / indentation copied before the text when it is copied from its start.
    pub prefix: String,
}

/// The text find searches for a surface: the painted text with line breaks folded to spaces,
/// byte for byte the same length so match ranges index the painted text directly.
pub fn searchable_text(surface: &SurfaceText) -> String {
    match surface.kind {
        SurfaceKind::Inline => surface.text.replace('\n', " "),
        SurfaceKind::Code => surface.text.clone(),
    }
}

/// Case-insensitive matches of `query` in `text`, as byte ranges of `text`. Lowercasing can change
/// a character's byte length, so matching runs on a lowered copy mapped back to `text` offsets.
pub fn find_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    if query.is_empty() || text.is_empty() {
        return Vec::new();
    }
    let needle = query.to_lowercase();
    let mut lowered = String::with_capacity(text.len());
    // For each byte of `lowered`, the `text` offset of the character it came from.
    let mut origin = Vec::with_capacity(text.len());
    for (index, character) in text.char_indices() {
        for lower in character.to_lowercase() {
            let before = lowered.len();
            lowered.push(lower);
            origin.extend(std::iter::repeat_n(index, lowered.len() - before));
        }
    }
    let char_end = |offset: usize| {
        offset
            + text[offset..]
                .chars()
                .next()
                .map_or(0, |character| character.len_utf8())
    };
    let mut ranges = Vec::new();
    let mut from = 0;
    while let Some(found) = lowered[from..].find(&needle) {
        let start = from + found;
        let end = start + needle.len();
        ranges.push(origin[start]..char_end(origin[end - 1]));
        from = end.max(start + 1);
        while from < lowered.len() && !lowered.is_char_boundary(from) {
            from += 1;
        }
    }
    ranges
}

/// Word under `offset` for double-click: a run of word characters, whitespace, or one symbol.
pub fn word_range(text: &str, offset: usize) -> Range<usize> {
    #[derive(PartialEq)]
    enum Class {
        Word,
        Space,
        Other,
    }
    fn class(character: char) -> Class {
        if character.is_alphanumeric() || character == '_' || character == '\'' {
            Class::Word
        } else if character.is_whitespace() {
            Class::Space
        } else {
            Class::Other
        }
    }
    let offset = floor_char_boundary(text, offset.min(text.len()));
    let Some((at, character)) = text[offset..]
        .chars()
        .next()
        .map(|character| (offset, character))
        .or_else(|| {
            text[..offset]
                .chars()
                .next_back()
                .map(|character| (offset - character.len_utf8(), character))
        })
    else {
        return offset..offset;
    };
    let target = class(character);
    if target == Class::Other {
        return at..at + character.len_utf8();
    }
    let start = text[..at]
        .char_indices()
        .rev()
        .take_while(|(_, character)| class(*character) == target)
        .last()
        .map_or(at, |(index, _)| index);
    let end = text[at..]
        .char_indices()
        .find(|(_, character)| class(*character) != target)
        .map_or(text.len(), |(index, _)| at + index);
    start..end
}

/// What triple-click selects: the whole inline surface, or one line of code.
pub fn block_unit_range(text: &str, kind: SurfaceKind, offset: usize) -> Range<usize> {
    match kind {
        SurfaceKind::Inline => 0..text.len(),
        SurfaceKind::Code => {
            let offset = offset.min(text.len());
            let start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
            let end = text[offset..]
                .find('\n')
                .map_or(text.len(), |index| offset + index);
            start..end
        }
    }
}

pub fn floor_char_boundary(text: &str, mut offset: usize) -> usize {
    offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// Plain text for `start..end` over `surfaces` (per top-level block, in document order).
/// Top-level blocks are separated by a blank line, except consecutive list items (one newline),
/// like a browser copying rendered paragraphs and lists.
pub fn selection_text(
    block_count: usize,
    mut surfaces_for: impl FnMut(usize) -> Vec<SurfaceText>,
    mut is_list_item: impl FnMut(usize) -> bool,
    start: TextPoint,
    end: TextPoint,
) -> String {
    let mut output = String::new();
    let mut previous_block: Option<usize> = None;
    let last_block = end.block.min(block_count.saturating_sub(1));
    if block_count == 0 || start >= end {
        return output;
    }
    for block in start.block..=last_block {
        for (index, surface) in surfaces_for(block).into_iter().enumerate() {
            let id = SurfaceId::new(block, index);
            if id < start.id() {
                continue;
            }
            if id > end.id() {
                break;
            }
            let len = surface.text.len();
            let lo = if id == start.id() {
                floor_char_boundary(&surface.text, start.offset)
            } else {
                0
            };
            let hi = if id == end.id() {
                floor_char_boundary(&surface.text, end.offset)
            } else {
                len
            };
            if lo >= hi && (id == start.id() || id == end.id()) {
                continue;
            }
            match previous_block {
                None => {}
                Some(previous) if previous == block => output.push(match surface.separator {
                    SurfaceSeparator::Newline => '\n',
                    SurfaceSeparator::Tab => '\t',
                }),
                Some(previous) => {
                    output.push('\n');
                    if !(previous + 1 == block && is_list_item(previous) && is_list_item(block)) {
                        output.push('\n');
                    }
                }
            }
            if lo == 0 {
                output.push_str(&surface.prefix);
            }
            output.push_str(&surface.text[lo..hi.max(lo)]);
            previous_block = Some(block);
        }
    }
    output
}

/// Splits `runs` so highlighted ranges can drop their own background (which would otherwise
/// cover the highlight painted behind the glyphs) and the active find match can recolor its text.
pub fn restyle_runs(
    runs: Vec<TextRun>,
    clear_background: &[Range<usize>],
    recolor: Option<(Range<usize>, Hsla)>,
) -> Vec<TextRun> {
    if clear_background.is_empty() && recolor.is_none() {
        return runs;
    }
    let mut cuts = clear_background
        .iter()
        .chain(recolor.as_ref().map(|(range, _)| range))
        .flat_map(|range| [range.start, range.end])
        .collect::<Vec<_>>();
    cuts.sort_unstable();
    cuts.dedup();
    let mut output = Vec::with_capacity(runs.len() + cuts.len());
    let mut offset = 0;
    for run in runs {
        let run_end = offset + run.len;
        let mut start = offset;
        for cut in cuts
            .iter()
            .copied()
            .filter(|cut| *cut > offset && *cut < run_end)
            .chain([run_end])
        {
            let mut piece = run.clone();
            piece.len = cut - start;
            let inside = |range: &Range<usize>| range.start <= start && cut <= range.end;
            if piece.background_color.is_some() && clear_background.iter().any(inside) {
                piece.background_color = None;
            }
            if let Some((range, color)) = &recolor
                && inside(range)
            {
                piece.color = *color;
            }
            output.push(piece);
            start = cut;
        }
        offset = run_end;
    }
    output
}

/// One visual (wrapped) line of a painted surface.
#[derive(Clone)]
pub struct VisualLine {
    /// Byte range of the surface text on this visual line.
    pub start: usize,
    pub end: usize,
    /// Byte range of the logical (`\n`-separated) line this visual line belongs to.
    pub logical_start: usize,
    pub logical_end: usize,
    layout: Arc<WrappedLineLayout>,
    /// Window position of the line's left edge (after text alignment) and top.
    pub origin: Point<Pixels>,
    /// Unwrapped x of `start`, subtracted to get positions on this visual line.
    start_x: Pixels,
}

impl VisualLine {
    fn x_for(&self, index: usize) -> Pixels {
        self.origin.x
            + self
                .layout
                .unwrapped_layout
                .x_for_index(index - self.logical_start)
            - self.start_x
    }
}

/// Pixel geometry of a painted surface, derived from its `TextLayout` after prepaint.
#[derive(Clone)]
pub struct SurfaceGeometry {
    pub bounds: Bounds<Pixels>,
    pub line_height: Pixels,
    pub lines: Vec<VisualLine>,
}

impl SurfaceGeometry {
    pub fn measure(layout: &TextLayout, text: &str, align: TextAlign) -> Self {
        let bounds = layout.bounds();
        let line_height = layout.line_height();
        let mut lines = Vec::new();
        let mut y = bounds.top();
        let mut logical_start = 0;
        for logical in text.split('\n') {
            let Some(wrapped) = layout.line_layout_for_index(logical_start) else {
                break;
            };
            let unwrapped = &wrapped.unwrapped_layout;
            let logical_end = logical_start + logical.len();
            let mut visual_start = 0;
            let boundaries = wrapped
                .wrap_boundaries
                .iter()
                .map(|boundary| unwrapped.runs[boundary.run_ix].glyphs[boundary.glyph_ix].index)
                .chain([logical.len()])
                .collect::<Vec<_>>();
            for visual_end in boundaries {
                let start_x = unwrapped.x_for_index(visual_start);
                let width = unwrapped.x_for_index(visual_end) - start_x;
                let x = match align {
                    TextAlign::Left => bounds.left(),
                    TextAlign::Center => bounds.left() + (bounds.size.width - width) / 2.0,
                    TextAlign::Right => bounds.right() - width,
                };
                lines.push(VisualLine {
                    start: logical_start + visual_start,
                    end: logical_start + visual_end,
                    logical_start,
                    logical_end,
                    layout: wrapped.clone(),
                    origin: point(x, y),
                    start_x,
                });
                y += line_height;
                visual_start = visual_end;
            }
            logical_start = logical_end + 1;
        }
        Self {
            bounds,
            line_height,
            lines,
        }
    }

    /// Rects covering `range`, one per visual line. A range that runs past the end of a logical
    /// line (it includes the line break) gets a small tail so empty lines still read selected.
    pub fn rects(&self, range: Range<usize>) -> Vec<Bounds<Pixels>> {
        let tail = self.line_height * 0.3;
        let mut rects = Vec::new();
        for line in &self.lines {
            let start = range.start.max(line.start);
            let end = range.end.min(line.end);
            let includes_break = range.end > line.end && line.end == line.logical_end;
            if start > end || (start == end && !includes_break) || range.start > line.end {
                continue;
            }
            let x0 = line.x_for(start);
            let mut x1 = line.x_for(end);
            if includes_break && range.start <= line.end {
                x1 += tail;
            }
            if x1 > x0 {
                rects.push(Bounds::new(
                    point(x0, line.origin.y),
                    size(x1 - x0, self.line_height),
                ));
            }
        }
        rects
    }

    /// The window position of the caret at `offset`, vertically centered on its line.
    pub fn caret_position(&self, offset: usize) -> Option<Point<Pixels>> {
        let line = self
            .lines
            .iter()
            .find(|line| line.start <= offset && offset <= line.end)?;
        Some(point(
            line.x_for(offset),
            line.origin.y + self.line_height / 2.0,
        ))
    }

    /// The caret offset closest to `position`.
    pub fn index_for_point(&self, position: Point<Pixels>) -> usize {
        let Some(line) = self
            .lines
            .iter()
            .rev()
            .find(|line| line.origin.y <= position.y)
            .or(self.lines.first())
        else {
            return 0;
        };
        let x = position.x - line.origin.x + line.start_x;
        let index = line.layout.unwrapped_layout.closest_index_for_x(x) + line.logical_start;
        index.clamp(line.start, line.end)
    }
}

/// A surface painted in the last frame, kept for hit testing between frames.
#[derive(Clone)]
pub struct PaintedSurface {
    pub id: SurfaceId,
    pub kind: SurfaceKind,
    pub text: gpui::SharedString,
    pub layout: TextLayout,
    pub align: TextAlign,
    /// The content mask the surface painted under (code blocks and tables scroll horizontally).
    pub clip: Bounds<Pixels>,
}

impl PaintedSurface {
    pub fn geometry(&self) -> SurfaceGeometry {
        SurfaceGeometry::measure(&self.layout, &self.text, self.align)
    }

    fn visible_bounds(&self) -> Bounds<Pixels> {
        let bounds = self.layout.bounds();
        let top = bounds.top().max(self.clip.top());
        let bottom = bounds.bottom().min(self.clip.bottom());
        let left = bounds.left().max(self.clip.left());
        let right = bounds.right().min(self.clip.right()).max(left);
        Bounds::from_corners(point(left, top), point(right, bottom.max(top)))
    }
}

/// Maps a window position to the nearest caret among painted surfaces (sorted by id). Points in
/// the gap between blocks land at the start of the next surface; beyond the painted content they
/// clamp to its first or last caret.
pub fn hit_test(surfaces: &[PaintedSurface], position: Point<Pixels>) -> Option<TextPoint> {
    let mut best: Option<(&PaintedSurface, Pixels)> = None;
    for surface in surfaces {
        let visible = surface.visible_bounds();
        if position.y < visible.top() || position.y >= visible.bottom() {
            continue;
        }
        let distance = if position.x < visible.left() {
            visible.left() - position.x
        } else if position.x > visible.right() {
            position.x - visible.right()
        } else {
            px(0.0)
        };
        if best.is_none_or(|(_, best_distance)| distance < best_distance) {
            best = Some((surface, distance));
        }
    }
    if let Some((surface, _)) = best {
        let offset = surface.geometry().index_for_point(position);
        return Some(TextPoint::new(surface.id, offset));
    }
    if let Some(next) = surfaces
        .iter()
        .find(|surface| surface.visible_bounds().top() > position.y)
    {
        return Some(TextPoint::new(next.id, 0));
    }
    surfaces
        .last()
        .map(|last| TextPoint::new(last.id, last.text.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Font, font};

    fn run(len: usize, background: Option<Hsla>) -> TextRun {
        let run_font: Font = font("Test");
        TextRun {
            len,
            font: run_font,
            color: gpui::black(),
            background_color: background,
            underline: None,
            strikethrough: None,
        }
    }

    #[test]
    fn find_ranges_are_case_insensitive_byte_ranges_of_the_painted_text() {
        assert_eq!(find_ranges("Match a MATCH", "match"), vec![0..5, 8..13]);
        assert_eq!(find_ranges("aaaa", "aa"), vec![0..2, 2..4]);
        // `İ` lowercases to two characters; offsets still index the original text.
        let text = "İx match";
        assert_eq!(find_ranges(text, "match"), vec![4..9]);
        assert_eq!(&text[4..9], "match");
        assert!(find_ranges("", "a").is_empty());
        assert!(find_ranges("abc", "").is_empty());
    }

    #[test]
    fn selected_range_clips_to_the_endpoints_surfaces() {
        let start = TextPoint::new(SurfaceId::new(1, 0), 3);
        let end = TextPoint::new(SurfaceId::new(3, 1), 2);
        assert_eq!(selected_range(start, end, SurfaceId::new(0, 5), 10), None);
        assert_eq!(
            selected_range(start, end, SurfaceId::new(1, 0), 10),
            Some(3..10)
        );
        assert_eq!(
            selected_range(start, end, SurfaceId::new(2, 4), 7),
            Some(0..7)
        );
        assert_eq!(
            selected_range(start, end, SurfaceId::new(3, 1), 7),
            Some(0..2)
        );
        assert_eq!(selected_range(start, end, SurfaceId::new(3, 2), 7), None);
        let all_end = TextPoint::document_end(4);
        assert_eq!(
            selected_range(start, all_end, SurfaceId::new(3, 9), 4),
            Some(0..4)
        );
    }

    #[test]
    fn word_and_line_units() {
        let text = "fn open_document(path) -> x";
        assert_eq!(&text[word_range(text, 5)], "open_document");
        assert_eq!(&text[word_range(text, 16)], "(");
        assert_eq!(&text[word_range(text, text.len())], "x");
        let quoted = "don't stop";
        assert_eq!(&quoted[word_range(quoted, 1)], "don't");
        let code = "one\ntwo three\nfour";
        assert_eq!(
            &code[block_unit_range(code, SurfaceKind::Code, 6)],
            "two three"
        );
        assert_eq!(
            block_unit_range(code, SurfaceKind::Inline, 6),
            0..code.len()
        );
    }

    #[test]
    fn restyle_runs_splits_highlights_without_changing_total_length() {
        let bg = Some(gpui::red());
        let runs = vec![run(4, None), run(6, bg), run(3, None)];
        let clear = vec![2..7, 7..7];
        let styled = restyle_runs(runs, &clear, Some((5..7, gpui::white())));
        assert_eq!(
            styled.iter().map(|run| run.len).collect::<Vec<_>>(),
            vec![2, 2, 1, 2, 3, 3]
        );
        assert_eq!(styled.iter().map(|run| run.len).sum::<usize>(), 13);
        // The code-chip background drops only inside the highlighted part.
        assert_eq!(styled[2].background_color, None);
        assert_eq!(styled[3].background_color, None);
        assert_eq!(styled[4].background_color, bg);
        assert_eq!(styled[3].color, gpui::white());
        assert_eq!(styled[2].color, gpui::black());
    }
}
