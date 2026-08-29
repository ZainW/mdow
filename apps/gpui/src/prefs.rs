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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnWidth {
    #[default]
    Standard,
    Comfortable,
    Wide,
}

impl ColumnWidth {
    pub const STANDARD_PX: f32 = 768.0;
    pub const COMFORTABLE_PX: f32 = 896.0;
    pub const WIDE_PX: f32 = 1088.0;

    pub fn max_width(self) -> f32 {
        match self {
            Self::Standard => Self::STANDARD_PX,
            Self::Comfortable => Self::COMFORTABLE_PX,
            Self::Wide => Self::WIDE_PX,
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
        Self::Column(ColumnWidth::Standard)
    }
}

impl ReaderWidth {
    pub fn toggled_full(self) -> Self {
        match self {
            Self::Column(column) => Self::Full { returns_to: column },
            Self::Full { returns_to } => Self::Column(returns_to),
        }
    }

    pub fn with_column(self, column: ColumnWidth) -> Self {
        match self {
            Self::Column(_) => Self::Column(column),
            Self::Full { .. } => Self::Full { returns_to: column },
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleTokens {
    pub control_font: f32,
    pub control_xs_font: f32,
    pub button_height: f32,
    pub button_xs_height: f32,
}

impl InterfaceScale {
    pub fn tokens(self) -> ScaleTokens {
        match self {
            Self::Compact => ScaleTokens {
                control_font: 12.0,
                control_xs_font: 10.0,
                button_height: 28.0,
                button_xs_height: 20.0,
            },
            Self::Comfortable => ScaleTokens {
                control_font: 13.0,
                control_xs_font: 11.0,
                button_height: 32.0,
                button_xs_height: 24.0,
            },
            Self::Large => ScaleTokens {
                control_font: 14.0,
                control_xs_font: 12.0,
                button_height: 36.0,
                button_xs_height: 28.0,
            },
        }
    }
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
            Self::SystemSans => ".AppleSystemUIFont",
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Prefs {
    pub theme_mode: ThemeMode,
    pub content_font: ContentFont,
    pub code_font: CodeFont,
    pub interface_scale: InterfaceScale,
    pub reader_width: ReaderWidth,
    pub zoom: ZoomLevel,
    pub sidebar_mode: SidebarMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrefEdit {
    Theme(ThemeMode),
    ContentFont(ContentFont),
    CodeFont(CodeFont),
    InterfaceScale(InterfaceScale),
    Column(ColumnWidth),
    ToggleFull,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Sidebar(SidebarMode),
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
            PrefEdit::Column(column) => self.reader_width = self.reader_width.with_column(column),
            PrefEdit::ToggleFull => self.reader_width = self.reader_width.toggled_full(),
            PrefEdit::ZoomIn => self.zoom = self.zoom.zoomed_in(),
            PrefEdit::ZoomOut => self.zoom = self.zoom.zoomed_out(),
            PrefEdit::ZoomReset => self.zoom = ZoomLevel::default(),
            PrefEdit::Sidebar(sidebar_mode) => self.sidebar_mode = sidebar_mode,
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
        let comfortable = ReaderWidth::Column(ColumnWidth::Comfortable);
        let full = comfortable.toggled_full();
        assert_eq!(
            full,
            ReaderWidth::Full {
                returns_to: ColumnWidth::Comfortable
            }
        );
        assert!(full.is_full());
        assert_eq!(full.max_width(), None);
        assert_eq!(full.toggled_full(), comfortable);
        assert_eq!(
            full.with_column(ColumnWidth::Wide).toggled_full(),
            ReaderWidth::Column(ColumnWidth::Wide)
        );
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
        assert_eq!(ColumnWidth::Standard.max_width(), 768.0);
        assert_eq!(ColumnWidth::Comfortable.max_width(), 896.0);
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
}
