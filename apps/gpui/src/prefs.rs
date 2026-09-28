//! Typed preference values. Illegal combinations do not exist. No IO, no wire strings.

pub const READER_FONT_SIZE: f32 = 15.5;
pub const READER_LINE_HEIGHT: f32 = 1.65;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn is_dark(self, system_is_dark: bool) -> bool {
        match self {
            Self::System => system_is_dark,
            Self::Light => false,
            Self::Dark => true,
        }
    }
}

/// A constrained reading column. `Full` is not a column: it lives on [`ReaderWidth`] so the
/// full-width toggle can return to the last column the reader chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnWidth {
    #[default]
    Narrow,
    Medium,
    Wide,
}

impl ColumnWidth {
    pub const NARROW_PX: f32 = 768.0;
    pub const MEDIUM_PX: f32 = 896.0;
    pub const WIDE_PX: f32 = 1088.0;

    pub fn max_width(self) -> f32 {
        match self {
            Self::Narrow => Self::NARROW_PX,
            Self::Medium => Self::MEDIUM_PX,
            Self::Wide => Self::WIDE_PX,
        }
    }
}

/// The four choices Settings offers for "Line width".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineWidth {
    Narrow,
    Medium,
    Wide,
    Full,
}

impl LineWidth {
    pub const ALL: [Self; 4] = [Self::Narrow, Self::Medium, Self::Wide, Self::Full];

    pub fn label(self) -> &'static str {
        match self {
            Self::Narrow => "Narrow",
            Self::Medium => "Medium",
            Self::Wide => "Wide",
            Self::Full => "Full",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderWidth {
    Column(ColumnWidth),
    Full { returns_to: ColumnWidth },
}

impl Default for ReaderWidth {
    fn default() -> Self {
        Self::Column(ColumnWidth::Narrow)
    }
}

impl ReaderWidth {
    pub fn toggled_full(self) -> Self {
        match self {
            Self::Column(column) => Self::Full { returns_to: column },
            Self::Full { returns_to } => Self::Column(returns_to),
        }
    }

    /// Choosing a column always leaves full width; choosing `Full` remembers the current column
    /// so the toggle (and the breadcrumb button) can return to it.
    pub fn with_line_width(self, width: LineWidth) -> Self {
        match width {
            LineWidth::Narrow => Self::Column(ColumnWidth::Narrow),
            LineWidth::Medium => Self::Column(ColumnWidth::Medium),
            LineWidth::Wide => Self::Column(ColumnWidth::Wide),
            LineWidth::Full => Self::Full {
                returns_to: self.column(),
            },
        }
    }

    pub fn line_width(self) -> LineWidth {
        match self {
            Self::Column(ColumnWidth::Narrow) => LineWidth::Narrow,
            Self::Column(ColumnWidth::Medium) => LineWidth::Medium,
            Self::Column(ColumnWidth::Wide) => LineWidth::Wide,
            Self::Full { .. } => LineWidth::Full,
        }
    }

    pub fn column(self) -> ColumnWidth {
        match self {
            Self::Column(column) | Self::Full { returns_to: column } => column,
        }
    }

    pub fn max_width(self) -> Option<f32> {
        match self {
            Self::Column(column) => Some(column.max_width()),
            Self::Full { .. } => None,
        }
    }

    pub fn is_full(self) -> bool {
        matches!(self, Self::Full { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterfaceScale {
    #[default]
    Compact,
    Comfortable,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoomLevel(u16);

impl Default for ZoomLevel {
    fn default() -> Self {
        Self::from_percent(100.0)
    }
}

impl ZoomLevel {
    pub const MIN: u16 = 60;
    pub const MAX: u16 = 200;
    pub const STEP: u16 = 10;

    pub fn from_percent(raw: f64) -> Self {
        let clamped = raw.clamp(Self::MIN as f64, Self::MAX as f64);
        let snapped = ((clamped / Self::STEP as f64).round() * Self::STEP as f64) as u16;
        Self(snapped.clamp(Self::MIN, Self::MAX))
    }

    pub fn percent(self) -> u16 {
        self.0
    }

    pub fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }

    pub fn zoomed_in(self) -> Self {
        Self((self.0 + Self::STEP).min(Self::MAX))
    }

    pub fn zoomed_out(self) -> Self {
        Self(self.0.saturating_sub(Self::STEP).max(Self::MIN))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentFont {
    #[default]
    Inter,
    Charter,
    SystemSans,
    Georgia,
}

impl ContentFont {
    pub fn family(self) -> &'static str {
        match self {
            Self::Inter => "Inter Variable",
            Self::Charter => "Charter",
            Self::SystemSans => ".SystemUIFont",
            Self::Georgia => "Georgia",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodeFont {
    #[default]
    GeistMono,
    SystemMono,
    SfMono,
    JetBrainsMono,
}

impl CodeFont {
    pub fn family(self) -> &'static str {
        match self {
            Self::GeistMono => "Geist Mono",
            Self::SystemMono => "Menlo",
            Self::SfMono => "SF Mono",
            Self::JetBrainsMono => "JetBrains Mono",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarMode {
    #[default]
    Recents,
    Folder,
    Outline,
}

// --- Companion (AI) prefs -------------------------------------------------------------
/// Whether the AI companion is offered, and which local agent it prefers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionPrefs {
    pub enabled: bool,
    /// `None`: the first installed agent.
    pub provider: Option<crate::companion::ProviderId>,
}

impl Default for CompanionPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: None,
        }
    }
}
// --- end Companion prefs ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prefs {
    pub theme_mode: ThemeMode,
    pub content_font: ContentFont,
    pub code_font: CodeFont,
    pub interface_scale: InterfaceScale,
    pub reader_width: ReaderWidth,
    pub zoom: ZoomLevel,
    pub sidebar_mode: SidebarMode,
    /// Electron's `autoUpdateEnabled`: whether Sparkle checks for updates in the background.
    pub auto_update: bool,
    pub companion: CompanionPrefs,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::default(),
            content_font: ContentFont::default(),
            code_font: CodeFont::default(),
            interface_scale: InterfaceScale::default(),
            reader_width: ReaderWidth::default(),
            zoom: ZoomLevel::default(),
            sidebar_mode: SidebarMode::default(),
            auto_update: true,
            companion: CompanionPrefs::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrefEdit {
    Theme(ThemeMode),
    ContentFont(ContentFont),
    CodeFont(CodeFont),
    InterfaceScale(InterfaceScale),
    LineWidth(LineWidth),
    ToggleFull,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Sidebar(SidebarMode),
    AutoUpdate(bool),
    CompanionEnabled(bool),
    CompanionProvider(Option<crate::companion::ProviderId>),
    ResetAll,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReaderStyle {
    pub content_family: &'static str,
    pub code_family: &'static str,
    pub font_size: f32,
    pub line_height: f32,
    pub max_width: Option<f32>,
}

impl Prefs {
    pub fn apply(&mut self, edit: PrefEdit) -> bool {
        let before = *self;
        match edit {
            PrefEdit::Theme(theme_mode) => self.theme_mode = theme_mode,
            PrefEdit::ContentFont(content_font) => self.content_font = content_font,
            PrefEdit::CodeFont(code_font) => self.code_font = code_font,
            PrefEdit::InterfaceScale(interface_scale) => self.interface_scale = interface_scale,
            PrefEdit::LineWidth(width) => {
                self.reader_width = self.reader_width.with_line_width(width)
            }
            PrefEdit::ToggleFull => self.reader_width = self.reader_width.toggled_full(),
            PrefEdit::ZoomIn => self.zoom = self.zoom.zoomed_in(),
            PrefEdit::ZoomOut => self.zoom = self.zoom.zoomed_out(),
            PrefEdit::ZoomReset => self.zoom = ZoomLevel::default(),
            PrefEdit::Sidebar(sidebar_mode) => self.sidebar_mode = sidebar_mode,
            PrefEdit::AutoUpdate(enabled) => self.auto_update = enabled,
            PrefEdit::CompanionEnabled(enabled) => self.companion.enabled = enabled,
            PrefEdit::CompanionProvider(provider) => self.companion.provider = provider,
            PrefEdit::ResetAll => *self = Self::default(),
        }
        *self != before
    }

    pub fn reader_style(&self) -> ReaderStyle {
        ReaderStyle {
            content_family: self.content_font.family(),
            code_family: self.code_font.family(),
            font_size: READER_FONT_SIZE * self.zoom.factor(),
            line_height: READER_LINE_HEIGHT,
            max_width: self.reader_width.max_width(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_full_is_an_involution_that_remembers_the_column() {
        let medium = ReaderWidth::Column(ColumnWidth::Medium);
        let full = medium.toggled_full();
        assert_eq!(
            full,
            ReaderWidth::Full {
                returns_to: ColumnWidth::Medium
            }
        );
        assert!(full.is_full());
        assert_eq!(full.max_width(), None);
        assert_eq!(full.line_width(), LineWidth::Full);
        assert_eq!(full.toggled_full(), medium);
    }

    #[test]
    fn full_line_width_round_trips_through_the_toggle_to_the_last_column() {
        let mut prefs = Prefs::default();
        prefs.apply(PrefEdit::LineWidth(LineWidth::Wide));
        assert!(prefs.apply(PrefEdit::LineWidth(LineWidth::Full)));
        assert_eq!(prefs.reader_width.line_width(), LineWidth::Full);
        assert_eq!(prefs.reader_style().max_width, None);
        assert!(!prefs.apply(PrefEdit::LineWidth(LineWidth::Full)));

        assert!(prefs.apply(PrefEdit::ToggleFull));
        assert_eq!(prefs.reader_width.line_width(), LineWidth::Wide);
        assert!(prefs.apply(PrefEdit::ToggleFull));
        assert_eq!(prefs.reader_width.line_width(), LineWidth::Full);

        // Picking a column while full leaves full width; the toggle then remembers it.
        assert!(prefs.apply(PrefEdit::LineWidth(LineWidth::Narrow)));
        assert_eq!(prefs.reader_style().max_width, Some(768.0));
        prefs.apply(PrefEdit::ToggleFull);
        prefs.apply(PrefEdit::ToggleFull);
        assert_eq!(prefs.reader_width.line_width(), LineWidth::Narrow);
    }

    #[test]
    fn line_width_choices_cover_every_reader_width() {
        for width in LineWidth::ALL {
            assert_eq!(
                ReaderWidth::default().with_line_width(width).line_width(),
                width
            );
        }
    }

    #[test]
    fn automatic_update_checks_default_on_and_reset_with_everything_else() {
        let mut prefs = Prefs::default();
        assert!(prefs.auto_update);
        assert!(prefs.apply(PrefEdit::AutoUpdate(false)));
        assert!(!prefs.auto_update);
        assert!(prefs.apply(PrefEdit::ResetAll));
        assert!(prefs.auto_update);
    }

    #[test]
    fn zoom_clamps_and_snaps_to_ten_percent_steps() {
        assert_eq!(ZoomLevel::from_percent(100.0).percent(), 100);
        assert_eq!(ZoomLevel::from_percent(67.0).percent(), 70);
        assert_eq!(ZoomLevel::from_percent(64.0).percent(), 60);
        assert_eq!(ZoomLevel::from_percent(12.0).percent(), 60);
        assert_eq!(ZoomLevel::from_percent(400.0).percent(), 200);
        assert_eq!(ZoomLevel::from_percent(200.0).zoomed_in().percent(), 200);
        assert_eq!(ZoomLevel::from_percent(60.0).zoomed_out().percent(), 60);
        assert_eq!(ZoomLevel::from_percent(100.0).zoomed_in().percent(), 110);
    }

    #[test]
    fn applying_the_same_pref_twice_is_a_noop_the_second_time() {
        let mut prefs = Prefs::default();
        assert!(prefs.apply(PrefEdit::Theme(ThemeMode::Dark)));
        assert!(!prefs.apply(PrefEdit::Theme(ThemeMode::Dark)));
        assert!(prefs.apply(PrefEdit::ToggleFull));
        assert!(prefs.apply(PrefEdit::ToggleFull));
        assert!(!prefs.apply(PrefEdit::ZoomReset));
        assert!(prefs.apply(PrefEdit::ResetAll));
        assert!(!prefs.apply(PrefEdit::ResetAll));
    }

    #[test]
    fn column_widths_match_electron_rem_values_at_sixteen_px() {
        assert_eq!(ColumnWidth::Narrow.max_width(), 768.0);
        assert_eq!(ColumnWidth::Medium.max_width(), 896.0);
        assert_eq!(ColumnWidth::Wide.max_width(), 1088.0);
    }

    #[test]
    fn reader_style_scales_type_not_leading() {
        let mut prefs = Prefs::default();
        prefs.apply(PrefEdit::ZoomIn);
        let style = prefs.reader_style();
        assert_eq!(style.font_size, READER_FONT_SIZE * 1.1);
        assert_eq!(style.line_height, READER_LINE_HEIGHT);
        assert_eq!(style.max_width, Some(768.0));
    }

    #[test]
    fn reader_style_uses_the_selected_content_and_code_fonts() {
        let mut prefs = Prefs::default();
        prefs.apply(PrefEdit::ContentFont(ContentFont::Charter));
        prefs.apply(PrefEdit::CodeFont(CodeFont::SystemMono));
        let style = prefs.reader_style();
        assert_eq!(style.content_family, ContentFont::Charter.family());
        assert_eq!(style.code_family, CodeFont::SystemMono.family());
    }

    #[test]
    fn bundled_reader_fonts_keep_stable_family_names() {
        assert_eq!(ContentFont::Inter.family(), "Inter Variable");
        assert_eq!(ContentFont::Charter.family(), "Charter");
        assert_eq!(CodeFont::GeistMono.family(), "Geist Mono");
        assert_eq!(CodeFont::JetBrainsMono.family(), "JetBrains Mono");
    }
}
