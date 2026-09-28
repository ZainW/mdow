//! Side-by-side reading, mirroring Electron's tab-slice split state (`splitView`, `activePane`,
//! `primaryPaneTabId`, `secondaryPaneTabId`). One tab strip feeds both panes; the focused pane
//! shows the active tab, and the two panes never show the same document.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PaneId {
    #[default]
    Primary,
    Secondary,
}

impl PaneId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Primary => "Left",
            Self::Secondary => "Right",
        }
    }
}

/// Smallest width either pane may be dragged to.
pub const MIN_PANE_WIDTH: f32 = 280.0;
pub const DEFAULT_RATIO: f32 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct SplitState {
    enabled: bool,
    active_pane: PaneId,
    primary: Option<PathBuf>,
    secondary: Option<PathBuf>,
    /// Share of the split width given to the left pane.
    ratio: f32,
}

impl Default for SplitState {
    fn default() -> Self {
        Self {
            enabled: false,
            active_pane: PaneId::Primary,
            primary: None,
            secondary: None,
            ratio: DEFAULT_RATIO,
        }
    }
}

/// What the session remembers about the split (Electron's `sessionSplitView`,
/// `sessionPrimaryPanePath`, `sessionSecondaryPanePath`, `sessionActivePane`).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSplit {
    pub primary: PathBuf,
    pub secondary: PathBuf,
    pub active_pane: PaneId,
    pub ratio: f32,
}

fn next_different(tabs: &[PathBuf], tab: Option<&Path>) -> Option<PathBuf> {
    let first = tabs.first()?;
    let Some(tab) = tab else {
        return Some(first.clone());
    };
    let Some(index) = tabs.iter().position(|candidate| candidate == tab) else {
        return Some(first.clone());
    };
    (1..tabs.len())
        .map(|offset| &tabs[(index + offset) % tabs.len()])
        .find(|candidate| candidate.as_path() != tab)
        .cloned()
}

fn existing(tabs: &[PathBuf], tab: Option<&PathBuf>) -> Option<PathBuf> {
    tab.filter(|tab| tabs.contains(tab)).cloned()
}

impl SplitState {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn active_pane(&self) -> PaneId {
        self.active_pane
    }

    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    pub fn pane_path(&self, pane: PaneId) -> Option<&Path> {
        match pane {
            PaneId::Primary => self.primary.as_deref(),
            PaneId::Secondary => self.secondary.as_deref(),
        }
    }

    /// The pane showing `path`, when split.
    pub fn pane_of(&self, path: &Path) -> Option<PaneId> {
        if !self.enabled {
            return None;
        }
        [PaneId::Primary, PaneId::Secondary]
            .into_iter()
            .find(|pane| self.pane_path(*pane) == Some(path))
    }

    /// Turns the split on or off; returns the tab to activate.
    pub fn toggle(&mut self, tabs: &[PathBuf], active: Option<&Path>) -> Option<PathBuf> {
        if self.enabled {
            self.disable(tabs, active)
        } else {
            self.enable(tabs, active)
        }
    }

    /// Opens the split with the active tab on the left and the next tab on the right.
    pub fn enable(&mut self, tabs: &[PathBuf], active: Option<&Path>) -> Option<PathBuf> {
        let active = active.map(Path::to_owned);
        let primary = existing(tabs, self.primary.as_ref())
            .or_else(|| existing(tabs, active.as_ref()))
            .or_else(|| tabs.first().cloned());
        let secondary = existing(tabs, self.secondary.as_ref())
            .filter(|secondary| Some(secondary) != primary.as_ref())
            .or_else(|| next_different(tabs, primary.as_deref()));
        self.enabled = primary.is_some();
        self.active_pane = PaneId::Primary;
        self.primary = primary.clone();
        self.secondary = if self.enabled { secondary } else { None };
        primary
    }

    /// Closes the split, keeping the active tab in the single reader.
    pub fn disable(&mut self, tabs: &[PathBuf], active: Option<&Path>) -> Option<PathBuf> {
        let active = active
            .map(Path::to_owned)
            .filter(|path| tabs.contains(path))
            .or_else(|| existing(tabs, self.primary.as_ref()))
            .or_else(|| existing(tabs, self.secondary.as_ref()));
        self.enabled = false;
        self.active_pane = PaneId::Primary;
        self.primary = active.clone();
        self.secondary = None;
        active
    }

    /// Focuses a pane; returns the tab that becomes active.
    pub fn set_active_pane(&mut self, pane: PaneId) -> Option<PathBuf> {
        if !self.enabled && pane == PaneId::Secondary {
            return None;
        }
        self.active_pane = pane;
        self.pane_path(pane).map(Path::to_owned)
    }

    /// "Open in Left/Right Pane": shows `tab` in `pane`, opening the split if needed.
    pub fn set_pane_tab(
        &mut self,
        pane: PaneId,
        tab: &Path,
        tabs: &[PathBuf],
        active: Option<&Path>,
    ) -> Option<PathBuf> {
        if !tabs.iter().any(|candidate| candidate == tab) {
            return None;
        }
        let tab = tab.to_owned();
        match pane {
            PaneId::Primary => {
                let secondary = existing(tabs, self.secondary.as_ref())
                    .filter(|secondary| secondary != &tab)
                    .or_else(|| next_different(tabs, Some(&tab)));
                self.enabled = secondary.is_some();
                self.secondary = secondary;
                self.primary = Some(tab.clone());
                self.active_pane = PaneId::Primary;
            }
            PaneId::Secondary => {
                let current_primary = existing(tabs, self.primary.as_ref())
                    .or_else(|| {
                        active
                            .map(Path::to_owned)
                            .filter(|path| tabs.contains(path))
                    })
                    .or_else(|| tabs.first().cloned());
                let primary = if current_primary.as_ref() == Some(&tab) {
                    next_different(tabs, Some(&tab))
                } else {
                    current_primary
                };
                // A lone tab cannot sit on the right with nothing on the left.
                self.enabled = primary.is_some();
                if self.enabled {
                    self.primary = primary;
                    self.secondary = Some(tab.clone());
                    self.active_pane = PaneId::Secondary;
                } else {
                    self.primary = Some(tab.clone());
                    self.secondary = None;
                    self.active_pane = PaneId::Primary;
                }
            }
        }
        Some(tab)
    }

    /// The active tab changed (tab click, keyboard switch, opening a file): the focused pane
    /// follows it, and the other pane moves on if it was showing the same document.
    pub fn activated(&mut self, tab: &Path, tabs: &[PathBuf]) {
        let tab = tab.to_owned();
        if !self.enabled {
            self.primary = Some(tab);
            return;
        }
        match self.active_pane {
            PaneId::Primary => {
                if self.secondary.as_ref() == Some(&tab) {
                    self.secondary = next_different(tabs, Some(&tab));
                }
                self.primary = Some(tab);
            }
            PaneId::Secondary => {
                if self.primary.as_ref() == Some(&tab) {
                    self.primary = next_different(tabs, Some(&tab));
                }
                self.secondary = Some(tab);
            }
        }
        self.settle(tabs, None, true);
    }

    /// A tab closed. `tabs` and `active` are the strip after closing. Returns the tab to
    /// activate so the focused pane keeps showing a document.
    pub fn closed(
        &mut self,
        closed: &Path,
        tabs: &[PathBuf],
        active: Option<&Path>,
    ) -> Option<PathBuf> {
        let active = active.map(Path::to_owned);
        let mut primary = existing(tabs, self.primary.as_ref());
        let mut secondary = existing(tabs, self.secondary.as_ref());
        if self.primary.as_deref() == Some(closed) {
            primary = active
                .clone()
                .filter(|active| Some(active) != secondary.as_ref())
                .or_else(|| next_different(tabs, secondary.as_deref()))
                .or_else(|| tabs.first().cloned());
        }
        if self.secondary.as_deref() == Some(closed) {
            secondary = next_different(tabs, primary.as_deref());
        }
        if primary.is_some() && primary == secondary {
            secondary = next_different(tabs, primary.as_deref());
        }
        self.primary = primary;
        self.secondary = secondary;
        self.settle(tabs, active.as_deref(), false)
    }

    /// Drops the split when it can no longer show two documents (an empty right pane is kept
    /// only while the reader is still filling it); returns the tab the focused pane shows.
    fn settle(
        &mut self,
        tabs: &[PathBuf],
        active: Option<&Path>,
        allow_empty_secondary: bool,
    ) -> Option<PathBuf> {
        let two_documents = self.primary.is_some()
            && (self.secondary.is_some() || allow_empty_secondary)
            && self.primary != self.secondary;
        if self.enabled && two_documents {
            if active.is_some_and(|active| self.secondary.as_deref() == Some(active)) {
                self.active_pane = PaneId::Secondary;
            }
            return self.pane_path(self.active_pane).map(Path::to_owned);
        }
        let keep = active
            .map(Path::to_owned)
            .filter(|path| tabs.contains(path))
            .or_else(|| self.primary.clone())
            .or_else(|| self.secondary.clone());
        self.enabled = false;
        self.active_pane = PaneId::Primary;
        self.primary = keep.clone();
        self.secondary = None;
        keep
    }

    /// Drag the divider: `x` is the pointer's offset into a split `width` pixels wide.
    pub fn drag_divider_to(&mut self, x: f32, width: f32) -> bool {
        let ratio = clamp_ratio(x / width.max(1.0), width);
        if (ratio - self.ratio).abs() < f32::EPSILON {
            return false;
        }
        self.ratio = ratio;
        true
    }

    pub fn reset_ratio(&mut self) -> bool {
        let changed = self.ratio != DEFAULT_RATIO;
        self.ratio = DEFAULT_RATIO;
        changed
    }

    /// Left pane width for a split `width` pixels wide, honouring both minimum widths.
    pub fn primary_width(&self, width: f32) -> f32 {
        (clamp_ratio(self.ratio, width) * width).round()
    }

    pub fn session(&self) -> Option<SessionSplit> {
        if !self.enabled {
            return None;
        }
        Some(SessionSplit {
            primary: self.primary.clone()?,
            secondary: self.secondary.clone()?,
            active_pane: self.active_pane,
            ratio: self.ratio,
        })
    }

    /// Restores a saved split if both documents reopened; returns the tab to activate.
    pub fn restore(&mut self, saved: &SessionSplit, tabs: &[PathBuf]) -> Option<PathBuf> {
        if saved.primary == saved.secondary
            || !tabs.contains(&saved.primary)
            || !tabs.contains(&saved.secondary)
        {
            return None;
        }
        self.enabled = true;
        self.primary = Some(saved.primary.clone());
        self.secondary = Some(saved.secondary.clone());
        self.active_pane = saved.active_pane;
        self.ratio = if saved.ratio.is_finite() {
            saved.ratio.clamp(0.0, 1.0)
        } else {
            DEFAULT_RATIO
        };
        self.pane_path(self.active_pane).map(Path::to_owned)
    }
}

/// Keeps both panes at least [`MIN_PANE_WIDTH`] wide; too narrow a split stays even.
pub fn clamp_ratio(ratio: f32, width: f32) -> f32 {
    if width < MIN_PANE_WIDTH * 2.0 {
        return DEFAULT_RATIO;
    }
    let min = MIN_PANE_WIDTH / width;
    ratio.clamp(min, 1.0 - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tabs(names: &[&str]) -> Vec<PathBuf> {
        names
            .iter()
            .map(|name| PathBuf::from(format!("/t/{name}.md")))
            .collect()
    }

    fn p(name: &str) -> PathBuf {
        PathBuf::from(format!("/t/{name}.md"))
    }

    fn panes(split: &SplitState) -> (Option<PathBuf>, Option<PathBuf>) {
        (
            split.pane_path(PaneId::Primary).map(Path::to_owned),
            split.pane_path(PaneId::Secondary).map(Path::to_owned),
        )
    }

    #[test]
    fn toggling_puts_the_active_tab_left_and_the_next_tab_right() {
        let tabs = tabs(&["a", "b", "c"]);
        let mut split = SplitState::default();
        assert_eq!(split.toggle(&tabs, Some(&p("b"))), Some(p("b")));
        assert!(split.is_enabled());
        assert_eq!(panes(&split), (Some(p("b")), Some(p("c"))));
        assert_eq!(split.active_pane(), PaneId::Primary);

        assert_eq!(split.toggle(&tabs, Some(&p("b"))), Some(p("b")));
        assert!(!split.is_enabled());
        assert_eq!(panes(&split), (Some(p("b")), None));
    }

    #[test]
    fn a_single_tab_splits_against_an_empty_right_pane() {
        let tabs = tabs(&["a"]);
        let mut split = SplitState::default();
        split.enable(&tabs, Some(&p("a")));
        assert!(split.is_enabled());
        assert_eq!(panes(&split), (Some(p("a")), None));
        // Nothing to split without documents.
        let mut empty = SplitState::default();
        assert_eq!(empty.enable(&[], None), None);
        assert!(!empty.is_enabled());
    }

    #[test]
    fn activating_a_tab_retargets_only_the_focused_pane() {
        let tabs = tabs(&["a", "b", "c"]);
        let mut split = SplitState::default();
        split.enable(&tabs, Some(&p("a")));
        assert_eq!(split.set_active_pane(PaneId::Secondary), Some(p("b")));
        split.activated(&p("c"), &tabs);
        assert_eq!(panes(&split), (Some(p("a")), Some(p("c"))));

        // Picking the other pane's document moves that pane on instead of duplicating it.
        split.activated(&p("a"), &tabs);
        assert_eq!(panes(&split), (Some(p("b")), Some(p("a"))));
        assert_eq!(split.active_pane(), PaneId::Secondary);
    }

    #[test]
    fn open_in_pane_assigns_and_opens_the_split() {
        let tabs = tabs(&["a", "b", "c"]);
        let mut split = SplitState::default();
        assert_eq!(
            split.set_pane_tab(PaneId::Secondary, &p("c"), &tabs, Some(&p("a"))),
            Some(p("c"))
        );
        assert!(split.is_enabled());
        assert_eq!(panes(&split), (Some(p("a")), Some(p("c"))));
        assert_eq!(split.active_pane(), PaneId::Secondary);

        split.set_pane_tab(PaneId::Primary, &p("c"), &tabs, Some(&p("c")));
        assert_eq!(panes(&split), (Some(p("c")), Some(p("a"))));
        assert_eq!(split.active_pane(), PaneId::Primary);

        // Moving the left document right swaps in the next one on the left.
        split.set_pane_tab(PaneId::Secondary, &p("c"), &tabs, Some(&p("c")));
        assert_eq!(panes(&split), (Some(p("a")), Some(p("c"))));
    }

    #[test]
    fn closing_a_pane_document_backfills_or_collapses_the_split() {
        let mut all = tabs(&["a", "b", "c"]);
        let mut split = SplitState::default();
        split.enable(&all, Some(&p("a")));
        // Right pane's document closes: the next different tab fills it.
        all.retain(|tab| tab != &p("b"));
        assert_eq!(split.closed(&p("b"), &all, Some(&p("a"))), Some(p("a")));
        assert_eq!(panes(&split), (Some(p("a")), Some(p("c"))));
        // Down to one document: the split closes.
        all.retain(|tab| tab != &p("c"));
        assert_eq!(split.closed(&p("c"), &all, Some(&p("a"))), Some(p("a")));
        assert!(!split.is_enabled());
        assert_eq!(panes(&split), (Some(p("a")), None));
    }

    #[test]
    fn closing_the_focused_left_document_keeps_the_right_pane() {
        let mut all = tabs(&["a", "b", "c"]);
        let mut split = SplitState::default();
        split.enable(&all, Some(&p("a")));
        all.retain(|tab| tab != &p("a"));
        // The strip activates the tab that slid into a's slot (b), which the right pane shows.
        let active = split.closed(&p("a"), &all, Some(&p("b")));
        assert_eq!(panes(&split), (Some(p("c")), Some(p("b"))));
        assert_eq!(active, Some(p("b")));
        assert_eq!(split.active_pane(), PaneId::Secondary);
    }

    #[test]
    fn divider_ratio_respects_minimum_widths_and_resets() {
        let mut split = SplitState::default();
        assert!(split.drag_divider_to(100.0, 1000.0));
        assert_eq!(split.ratio(), 0.28);
        assert_eq!(split.primary_width(1000.0), 280.0);
        assert!(split.drag_divider_to(900.0, 1000.0));
        assert_eq!(split.primary_width(1000.0), 720.0);
        assert!(split.reset_ratio());
        assert_eq!(split.primary_width(1000.0), 500.0);
        // Too narrow to honour both minimums: stay even.
        assert_eq!(clamp_ratio(0.2, 400.0), 0.5);
    }

    #[test]
    fn session_round_trip_requires_both_documents() {
        let tabs = tabs(&["a", "b", "c"]);
        let mut split = SplitState::default();
        assert_eq!(split.session(), None);
        split.set_pane_tab(PaneId::Secondary, &p("c"), &tabs, Some(&p("a")));
        split.drag_divider_to(400.0, 1000.0);
        let saved = split.session().unwrap();
        assert_eq!(saved.active_pane, PaneId::Secondary);

        let mut restored = SplitState::default();
        assert_eq!(restored.restore(&saved, &tabs), Some(p("c")));
        assert_eq!(restored, split);

        let mut missing = SplitState::default();
        assert_eq!(missing.restore(&saved, &tabs[..2]), None);
        assert!(!missing.is_enabled());
    }
}
