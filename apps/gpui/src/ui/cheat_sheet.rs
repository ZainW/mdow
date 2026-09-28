//! Hold ⌘ alone to peek at the essential shortcuts, like Electron's `CheatSheetOverlay` and
//! `useCheatSheetHold`: the sheet appears after [`HOLD_DELAY`], and any other key, another
//! modifier, releasing ⌘ or leaving the window dismisses it.

use crate::{
    overlay::{CommandId, command_catalog},
    theme::{ColorScheme, Metrics, Theme},
};
use gpui::{
    Animation, AnimationExt, AnyElement, FontWeight, Modifiers, SharedString, div, ease_out_quint,
    prelude::*, px, relative,
};
use std::time::Duration;

/// Electron's `CHEAT_SHEET_HOLD_MS`.
pub const HOLD_DELAY: Duration = Duration::from_millis(700);
pub const FADE_IN: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Hidden,
    /// ⌘ is down alone; the timer for this generation will show the sheet.
    Pending(u64),
    Shown,
}

/// Timers are keyed by generation, so a stale timer after a release or combo does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheatSheetHold {
    phase: Phase,
    generation: u64,
    /// Bumps each time the sheet appears, keying its entrance animation.
    appearance: u64,
    /// ⌘ was used in a combo (or the sheet dismissed): wait for a full release.
    suppressed: bool,
}

impl Default for CheatSheetHold {
    fn default() -> Self {
        Self {
            phase: Phase::Hidden,
            generation: 0,
            appearance: 0,
            suppressed: false,
        }
    }
}

/// ⌘ held with no other modifier.
pub fn is_hold_modifiers(modifiers: Modifiers) -> bool {
    modifiers.platform
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.shift
        && !modifiers.function
}

impl CheatSheetHold {
    pub fn is_visible(&self) -> bool {
        self.phase == Phase::Shown
    }

    pub fn appearance(&self) -> u64 {
        self.appearance
    }

    /// Returns the generation to fire after [`HOLD_DELAY`] when ⌘ alone just went down.
    /// `blocked` is true while a modal (palette, settings, shortcuts, find, a menu) is open.
    pub fn modifiers_changed(&mut self, modifiers: Modifiers, blocked: bool) -> Option<u64> {
        if modifiers.number_of_modifiers() == 0 {
            self.suppressed = false;
            self.hide();
            return None;
        }
        if !is_hold_modifiers(modifiers) {
            // Another modifier joined: this is a chord, not a peek.
            self.suppressed = true;
            self.hide();
            return None;
        }
        if blocked || self.suppressed || self.phase != Phase::Hidden {
            return None;
        }
        self.generation += 1;
        self.phase = Phase::Pending(self.generation);
        Some(self.generation)
    }

    /// Any key press cancels the peek until ⌘ is released. Returns whether the sheet was
    /// visible (so the caller redraws).
    pub fn key_down(&mut self) -> bool {
        let was_visible = self.is_visible();
        if self.phase != Phase::Hidden {
            self.suppressed = true;
        }
        self.hide();
        was_visible
    }

    /// The hold timer fired; returns whether the sheet just appeared.
    pub fn timer_fired(&mut self, generation: u64, blocked: bool) -> bool {
        if self.phase != Phase::Pending(generation) {
            return false;
        }
        if blocked {
            self.phase = Phase::Hidden;
            return false;
        }
        self.phase = Phase::Shown;
        self.appearance += 1;
        true
    }

    /// The window lost focus or a modal took over.
    pub fn reset(&mut self) -> bool {
        let was_visible = self.is_visible();
        self.suppressed = false;
        self.hide();
        was_visible
    }

    fn hide(&mut self) {
        if self.phase != Phase::Hidden {
            self.generation += 1;
        }
        self.phase = Phase::Hidden;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheatSheetRow {
    pub label: &'static str,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheatSheetSection {
    pub heading: &'static str,
    pub rows: Vec<CheatSheetRow>,
}

/// Splits a shortcut label ("⇧⌘O", "⌘1–9") into keycaps: one per modifier glyph, then the key.
pub fn keycaps(keys: &str) -> Vec<String> {
    let mut caps = Vec::new();
    let mut rest = String::new();
    for glyph in keys.chars() {
        if rest.is_empty() && matches!(glyph, '⌘' | '⌥' | '⇧' | '⌃') {
            caps.push(glyph.to_string());
        } else {
            rest.push(glyph);
        }
    }
    if !rest.is_empty() {
        // Electron's sheet shows the printed key, not the unshifted one the binding uses.
        caps.push(match rest.as_str() {
            "=" => "+".into(),
            "-" => "−".into(),
            _ => rest,
        });
    }
    caps
}

fn command(id: CommandId, label: &'static str) -> Option<CheatSheetRow> {
    let spec = command_catalog().iter().find(|spec| spec.id == id)?;
    Some(CheatSheetRow {
        label,
        keys: keycaps(spec.keys?),
    })
}

fn fixed(label: &'static str, keys: &str) -> Option<CheatSheetRow> {
    Some(CheatSheetRow {
        label,
        keys: keycaps(keys),
    })
}

/// Electron's three columns: Navigation | View | Files + App. Shortcuts come from the command
/// catalog; the palette, tab switching and full screen have no catalog entry.
pub fn cheat_sheet_columns() -> Vec<Vec<CheatSheetSection>> {
    let section = |heading, rows: Vec<Option<CheatSheetRow>>| CheatSheetSection {
        heading,
        rows: rows.into_iter().flatten().collect(),
    };
    vec![
        vec![section(
            "Navigation",
            vec![
                fixed("Command palette", "⌘K"),
                command(CommandId::FindInDocument, "Find in document"),
                fixed("Switch tab", "⌘1–9"),
                command(CommandId::NextTab, "Next tab"),
                command(CommandId::PreviousTab, "Previous tab"),
                command(CommandId::CloseTab, "Close tab"),
            ],
        )],
        vec![section(
            "View",
            vec![
                command(CommandId::ToggleSidebar, "Toggle sidebar"),
                command(CommandId::ToggleSplitView, "Split view"),
                command(CommandId::ZoomIn, "Zoom in"),
                command(CommandId::ZoomOut, "Zoom out"),
                command(CommandId::ZoomReset, "Reset zoom"),
                fixed("Full screen", "⌃⌘F"),
            ],
        )],
        vec![
            section(
                "Files",
                vec![
                    command(CommandId::OpenFile, "Open file"),
                    command(CommandId::OpenFolder, "Open folder"),
                ],
            ),
            section(
                "App",
                vec![
                    command(CommandId::OpenSettings, "Settings"),
                    command(CommandId::OpenShortcuts, "All shortcuts"),
                ],
            ),
        ],
    ]
}

fn keycap(key: String, theme: Theme) -> AnyElement {
    let ui = theme.ui;
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .min_w(px(ui.space(20.0)))
        .h(px(ui.space(20.0)))
        .px(px(ui.space(5.0)))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.muted)
        .font_family(Metrics::FONT_SANS)
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(ui.text(11.0)))
        .text_color(theme.muted_foreground)
        .child(SharedString::from(key))
        .into_any_element()
}

/// Column shares from Electron's `grid-cols-[1.15fr_1fr_0.9fr]`.
const COLUMN_SHARES: [f32; 3] = [1.15, 1.0, 0.9];

pub fn render_cheat_sheet(hold: &CheatSheetHold, reduce_motion: bool, theme: Theme) -> AnyElement {
    let ui = theme.ui;
    let total: f32 = COLUMN_SHARES.iter().sum();
    let mut grid = div().flex().items_start().w_full();
    for (index, sections) in cheat_sheet_columns().into_iter().enumerate() {
        let mut column = div()
            .flex()
            .flex_col()
            .flex_none()
            .min_w_0()
            .w(relative(COLUMN_SHARES[index] / total))
            .when(index > 0, |column| {
                column
                    .border_l_1()
                    .border_color(theme.border_subtle)
                    .pl(px(ui.space(24.0)))
            })
            .when(index + 1 < COLUMN_SHARES.len(), |column| {
                column.pr(px(ui.space(24.0)))
            });
        for (section_index, section) in sections.into_iter().enumerate() {
            let mut rows = div().flex().flex_col();
            for row in section.rows {
                let mut caps = div().flex().flex_none().items_center().gap(px(3.0));
                for key in row.keys {
                    caps = caps.child(keycap(key, theme));
                }
                rows = rows.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(ui.space(12.0)))
                        .h(px(ui.space(26.0)))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(ui.text(13.0)))
                                .text_color(theme.muted_foreground)
                                .child(row.label),
                        )
                        .child(caps),
                );
            }
            column = column.child(
                div()
                    .when(section_index > 0, |section| section.mt(px(ui.space(14.0))))
                    .child(
                        div()
                            .mb(px(ui.space(8.0)))
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(ui.text(10.0)))
                            .text_color(theme.muted_foreground.opacity(0.75))
                            .child(section.heading.to_uppercase()),
                    )
                    .child(rows),
            );
        }
        grid = grid.child(column);
    }

    let footer = div()
        .mt(px(ui.space(12.0)))
        .pt(px(ui.space(10.0)))
        .flex()
        .items_center()
        .justify_between()
        .border_t_1()
        .border_color(theme.border_subtle)
        .text_size(px(ui.text(11.0)))
        .text_color(theme.muted_foreground.opacity(0.75))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child("Hold")
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child("⌘"),
                )
                .child("· release to dismiss"),
        )
        .child("⌘/ for full list");

    let card = div()
        .debug_selector(|| "cheat-sheet".into())
        .relative()
        .w(px(ui.space(720.0)))
        .max_w_full()
        .px(px(ui.space(20.0)))
        .pt(px(ui.space(16.0)))
        .pb(px(ui.space(12.0)))
        .rounded(px(Metrics::RADIUS))
        .bg(theme.surface_raised)
        .border_1()
        .map(|card| match theme.color_scheme {
            ColorScheme::Light => card.border_color(theme.border_subtle).shadow_lg(),
            ColorScheme::Dark => card.border_color(theme.border),
        })
        .font_family(Metrics::FONT_SANS)
        .text_color(theme.foreground)
        .child(grid)
        .child(footer);

    // Bottom-centred and click-through, like Electron's pointer-events-none dialog.
    let layer = div()
        .absolute()
        .left_0()
        .right_0()
        .bottom(px(24.0))
        .px(px(16.0))
        .flex()
        .justify_center();
    if reduce_motion {
        return layer.child(card).into_any_element();
    }
    layer
        .child(card.with_animation(
            ("cheat-sheet-in", hold.appearance()),
            Animation::new(FADE_IN).with_easing(ease_out_quint()),
            |card, delta| card.opacity(delta).top(px(4.0 * (1.0 - delta))),
        ))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd() -> Modifiers {
        Modifiers::command()
    }

    #[test]
    fn holding_command_alone_schedules_then_shows_the_sheet() {
        let mut hold = CheatSheetHold::default();
        let generation = hold
            .modifiers_changed(cmd(), false)
            .expect("timer scheduled");
        assert!(!hold.is_visible());
        assert!(hold.timer_fired(generation, false));
        assert!(hold.is_visible());
        // Releasing ⌘ hides it.
        hold.modifiers_changed(Modifiers::none(), false);
        assert!(!hold.is_visible());
    }

    #[test]
    fn a_combo_or_early_release_never_shows_the_sheet() {
        let mut hold = CheatSheetHold::default();
        let generation = hold.modifiers_changed(cmd(), false).unwrap();
        hold.key_down();
        assert!(!hold.timer_fired(generation, false));
        // Still holding ⌘ after the combo: no new peek until a full release.
        assert_eq!(hold.modifiers_changed(cmd(), false), None);
        hold.modifiers_changed(Modifiers::none(), false);
        assert!(hold.modifiers_changed(cmd(), false).is_some());

        let mut early = CheatSheetHold::default();
        let generation = early.modifiers_changed(cmd(), false).unwrap();
        early.modifiers_changed(Modifiers::none(), false);
        assert!(!early.timer_fired(generation, false));

        let mut chord = CheatSheetHold::default();
        let generation = chord.modifiers_changed(cmd(), false).unwrap();
        chord.modifiers_changed(
            Modifiers {
                platform: true,
                shift: true,
                ..Default::default()
            },
            false,
        );
        assert!(!chord.timer_fired(generation, false));
    }

    #[test]
    fn modals_block_the_sheet_and_blur_resets_it() {
        let mut hold = CheatSheetHold::default();
        assert_eq!(hold.modifiers_changed(cmd(), true), None);
        let generation = hold.modifiers_changed(cmd(), false).unwrap();
        assert!(!hold.timer_fired(generation, true));
        assert!(!hold.is_visible());

        let generation = hold.modifiers_changed(cmd(), false).unwrap();
        assert!(hold.timer_fired(generation, false));
        assert!(hold.reset());
        assert!(!hold.is_visible());
    }

    #[test]
    fn any_key_while_shown_dismisses_until_release() {
        let mut hold = CheatSheetHold::default();
        let generation = hold.modifiers_changed(cmd(), false).unwrap();
        hold.timer_fired(generation, false);
        assert!(hold.key_down());
        assert!(!hold.is_visible());
        assert_eq!(hold.modifiers_changed(cmd(), false), None);
    }

    #[test]
    fn keycaps_split_modifier_glyphs_from_the_key() {
        assert_eq!(keycaps("⇧⌘O"), vec!["⇧", "⌘", "O"]);
        assert_eq!(keycaps("⌥⌘→"), vec!["⌥", "⌘", "→"]);
        assert_eq!(keycaps("⌘1–9"), vec!["⌘", "1–9"]);
        assert_eq!(keycaps("⌘="), vec!["⌘", "+"]);
        assert_eq!(keycaps("⌘-"), vec!["⌘", "−"]);
        assert_eq!(keycaps("⌘\\"), vec!["⌘", "\\"]);
    }

    #[test]
    fn columns_follow_electrons_layout_and_every_row_has_keys() {
        let columns = cheat_sheet_columns();
        let headings = columns
            .iter()
            .map(|column| {
                column
                    .iter()
                    .map(|section| section.heading)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            headings,
            vec![vec!["Navigation"], vec!["View"], vec!["Files", "App"]]
        );
        let labels = columns
            .iter()
            .flatten()
            .flat_map(|section| section.rows.iter().map(|row| row.label))
            .collect::<Vec<_>>();
        for label in [
            "Command palette",
            "Find in document",
            "Switch tab",
            "Next tab",
            "Previous tab",
            "Close tab",
            "Toggle sidebar",
            "Split view",
            "Zoom in",
            "Full screen",
            "Open file",
            "Open folder",
            "Settings",
            "All shortcuts",
        ] {
            assert!(labels.contains(&label), "{label} missing");
        }
        assert!(
            columns
                .iter()
                .flatten()
                .flat_map(|section| &section.rows)
                .all(|row| !row.keys.is_empty())
        );
    }
}
