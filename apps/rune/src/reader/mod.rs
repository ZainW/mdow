//! The document view: one virtual row per top-level block, centred in a column whose width
//! follows the reading width and zoom, inside a scroll view. Only rows near the viewport are
//! built, so very long documents open and scroll as fast as short ones.

pub mod blocks;
pub mod inline;

use crate::app::{App, LoadedDoc};
use crate::theme::{self, Palette};
use blocks::{Ctx, Hit, ReadStyle};
use rune::{El, GrowthSpec, ScrollSpec, col, row, text};
use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;

/// Room between the card's edge and the text column.
pub const SIDE: f32 = 48.0;
pub const TOP: f32 = 44.0;
pub const BOTTOM: f32 = 120.0;

pub fn scroll_key(pane: u64) -> String {
    format!("scroll-{pane}")
}

pub fn rows_key(pane: u64) -> String {
    format!("rows-{pane}")
}

pub fn row_key(pane: u64, row: usize) -> String {
    format!("p{pane}r{row}")
}

/// A find match: the row, which text in it, and the byte range.
#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub row: usize,
    pub text: usize,
    pub range: Range<usize>,
}

/// Every match of `query` in `doc`, case-insensitively, in reading order.
pub fn find(doc: &LoadedDoc, query: &str, st: &ReadStyle, p: &Palette) -> Vec<Match> {
    let needle = query.to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut texts = Vec::new();
    for (row, b) in doc.doc.blocks.iter().enumerate() {
        texts.clear();
        blocks::texts(b, &mut texts, st, p);
        for (ti, t) in texts.iter().enumerate() {
            let hay = t.to_lowercase();
            // Lowercasing can change byte lengths; only trust ranges when it didn't.
            if hay.len() != t.len() {
                continue;
            }
            let mut from = 0;
            while let Some(at) = hay[from..].find(&needle) {
                let start = from + at;
                out.push(Match { row, text: ti, range: start..start + needle.len() });
                from = start + needle.len().max(1);
                if out.len() >= 10_000 {
                    return out;
                }
            }
        }
    }
    out
}

/// Builds one row of the pane `pane`'s document.
fn build_row(
    s: &App,
    pane_id: u64,
    i: usize,
    st: &ReadStyle,
    p: &Palette,
    hits: &HashMap<usize, Vec<Hit>>,
) -> El {
    let Some(pane) = s.pane(pane_id) else { return col() };
    let Some(doc) = pane.doc.as_ref() else { return col() };
    let Some(block) = doc.doc.blocks.get(i) else { return col() };
    let n = doc.doc.blocks.len();
    let gap = doc.gaps.get(i).copied().unwrap_or(0.0) as f32;
    let top = if i == 0 { TOP } else { (gap * st.size * 0.8).round() };
    let bottom = if i + 1 == n { BOTTOM } else { 0.0 };
    let empty = Vec::new();
    let ctx = Ctx {
        p,
        st,
        pane: pane_id,
        doc_path: &doc.path,
        media: &s.media,
        hits: hits.get(&i).unwrap_or(&empty),
        texts: Cell::new(0),
        copied: s.copied_code.map(|(id, _)| id),
    };
    let content = ctx.block(block, st.column, p.text, 0);
    row()
        .w_full()
        .justify_center()
        .px(SIDE)
        .pt(top)
        .pb(bottom)
        .child(col().w(st.column).min_w(0.0).child(content))
}

/// The document in pane `pane_id`, `width` points wide.
pub fn document(s: &App, pane_id: u64, doc: &LoadedDoc, width: f32, p: &Palette) -> El {
    let prefs = &s.prefs;
    let column = prefs.column_width((width - 2.0 * SIDE).max(200.0)).floor();
    let st = ReadStyle {
        size: prefs.font_size(),
        body_family: prefs.body_font.family(),
        code_family: prefs.code_font.family(),
        line: prefs.line_spacing.multiple(),
        column,
    };
    // Find highlights for this pane, by row.
    let mut hits: HashMap<usize, Vec<Hit>> = HashMap::new();
    if let Some(f) = s.find.as_ref().filter(|f| f.pane == pane_id) {
        for (mi, m) in f.matches.iter().enumerate() {
            hits.entry(m.row).or_default().push(Hit {
                text: m.text,
                range: m.range.clone(),
                current: Some(mi) == f.current,
            });
        }
    }
    let palette = *p;
    let count = doc.doc.blocks.len();
    let version = (
        doc.version,
        st.size.to_bits(),
        column.to_bits(),
        p.id,
        s.media_revision,
        s.find_revision,
        s.copied_code.map(|c| c.0),
        (st.body_family, st.code_family, st.line.to_bits()),
    );
    let rows = col()
        .key(rows_key(pane_id))
        .w_full()
        .growth(GrowthSpec::default())
        .selection_group(true)
        .on_secondary_press(|s: &mut App, press, tx: &mut rune::Tx| s.reader_menu(None, press, tx))
        // ⌘A selects every text; scrolling then unmounts rows, so the flag outlives them and
        // ⌘C copies the whole document (`ui::on_key`). Selecting part of anything drops it.
        .on_selection_change(|s: &mut App, change: &rune::SelectionChange, tx: &mut rune::Tx| {
            let whole = change
                .selection()
                .is_some_and(|sel| sel.texts().iter().all(|t| !t.is_selected() || t.is_whole()));
            if !whole {
                s.select_all = None;
            }
            tx.live_only();
        })
        .virtual_rows(
            count,
            st.size * 3.0,
            move |_s: &App, i| row_key(pane_id, i).into(),
            move |s: &App, i| build_row(s, pane_id, i, &st, &palette, &hits),
        )
        .virtual_rows_version(version)
        .on_visible_rows(move |s: &mut App, range| s.rows_visible(pane_id, range));
    col()
        .key(scroll_key(pane_id))
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .scroll_anchor()
        .child(rows)
}

/// A file that couldn't be read.
pub fn error(kind: crate::files::ErrorKind, path: &std::path::Path, p: &Palette) -> El {
    col()
        .w_full()
        .grow(1.0)
        .center()
        .gap(8.0)
        .px(40.0)
        .child(text(kind.title()).font(theme::ui(17.0, 600.0)).color(p.text))
        .child(
            text(kind.body())
                .font(theme::ui(13.0, 400.0))
                .color(p.secondary)
                .wrap()
                .max_w(420.0)
                .align(rune::Align::Center),
        )
        .child(
            text(crate::files::display_dir(path) + "/" + &crate::files::display_name(path))
                .font(theme::mono(11.0, 400.0))
                .color(p.tertiary)
                .pt(6.0),
        )
}

/// The document as plain text, as copying all of it gives: each text on its own line.
pub fn plain_text(doc: &LoadedDoc, st: &ReadStyle, p: &Palette) -> String {
    let mut texts = Vec::new();
    for b in &doc.doc.blocks {
        blocks::texts(b, &mut texts, st, p);
    }
    texts.join("\n")
}
