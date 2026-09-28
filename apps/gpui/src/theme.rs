use crate::prefs::{InterfaceScale, ThemeMode};
use gpui::{App, Global, Hsla, Pixels, Point, WindowAppearance, hsla, point, px};

pub struct TrafficLights;

impl TrafficLights {
    pub const INSET: f32 = 14.0;
    pub const BUTTON_DIAMETER: f32 = 12.0;
    pub const BUTTON_GAP: f32 = 8.0;
    pub const NATIVE_TITLEBAR: f32 = 28.0;

    pub fn position() -> Point<Pixels> {
        point(px(Self::INSET), px(Self::INSET))
    }

    pub const fn cluster_width() -> f32 {
        3.0 * Self::BUTTON_DIAMETER + 2.0 * Self::BUTTON_GAP
    }

    pub const fn titlebar_height() -> f32 {
        2.0 * Self::INSET + Self::BUTTON_DIAMETER
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrafficLightClearance(f32);

impl TrafficLightClearance {
    pub(crate) const fn reserved() -> Self {
        Self(TrafficLights::INSET + TrafficLights::cluster_width() + TrafficLights::INSET)
    }

    pub const fn width(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitlebarLayout {
    pub clearance: TrafficLightClearance,
    pub controls_x: f32,
    pub height: f32,
}

impl TitlebarLayout {
    pub(crate) const fn standard() -> Self {
        let clearance = TrafficLightClearance::reserved();
        Self {
            clearance,
            controls_x: clearance.width(),
            height: TrafficLights::titlebar_height(),
        }
    }
}

pub struct Metrics;

impl Metrics {
    pub const FONT_SANS: &'static str = "Inter Variable";
    pub const FONT_MONO: &'static str = "Geist Mono";
    pub const APP_FONT_SIZE: f32 = 13.0;
    pub const CONTROL_FONT_SIZE: f32 = 12.0;
    pub const ICON_SIZE: f32 = 16.0;
    pub const SIDEBAR_WIDTH: f32 = 244.0;
    pub const MIN_MAIN_WIDTH_WITH_SIDEBAR: f32 = 320.0;
    pub const TITLEBAR_BUTTON: f32 = 28.0;
    /// The tab bar doubles as the titlebar row, so it shares the traffic-light row height.
    pub const TAB_BAR_HEIGHT: f32 = TrafficLights::titlebar_height();
    /// The sidebar column runs to the top of the window; its header is the titlebar row.
    pub const SIDEBAR_HEADER_HEIGHT: f32 = TrafficLights::titlebar_height();
    pub const SIDEBAR_HEADER_END_INSET: f32 = 6.0;
    pub const SEGMENTED_HEIGHT: f32 = 28.0;
    pub const SEGMENTED_INSET: f32 = 10.0;
    pub const SEGMENTED_GAP_BELOW: f32 = 8.0;
    pub const SECTION_HEADER_HEIGHT: f32 = 30.0;
    pub const TREE_ROW_HEIGHT: f32 = 24.0;
    pub const OUTLINE_ROW_HEIGHT: f32 = 26.0;
    pub const RECENT_ROW_HEIGHT: f32 = 34.0;
    pub const MENU_ROW_HEIGHT: f32 = 28.0;
    pub const MENU_ROW_RADIUS: f32 = 6.0;
    pub const MENU_MIN_WIDTH: f32 = 200.0;
    pub const TAB_HEIGHT: f32 = 28.0;
    pub const TAB_MAX_WIDTH: f32 = 200.0;
    pub const TAB_LIST_INSET: f32 = 6.0;
    pub const TAB_GAP: f32 = 1.0;
    pub const TAB_RADIUS: f32 = 6.0;
    pub const TAB_CONTENT_INSET: f32 = 10.0;
    pub const TAB_CONTENT_GAP: f32 = 6.0;
    pub const TAB_ICON_SIZE: f32 = 14.0;
    pub const TAB_CLOSE_SIZE: f32 = 24.0;
    pub const TAB_CLOSE_END_MARGIN: f32 = 4.0;
    pub const BREADCRUMB_HEIGHT: f32 = 28.0;
    pub const READER_MAX_WIDTH: f32 = 768.0;
    pub const READER_INSET: f32 = 32.0;
    pub const READER_TOP_PADDING: f32 = 32.0;
    pub const READER_BOTTOM_PADDING: f32 = 40.0;
    pub const RADIUS: f32 = 8.0;
}

/// Chrome sizes for one interface scale, mirroring Electron's `[data-ui-scale]` token tables
/// (`--control-font-size`, `--button-*`, `--tabbar-height`, `--breadcrumb-*`, `--sidebar-*`).
/// Compact matches the fixed [`Metrics`]; the other scales grow controls, type and spacing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiMetrics {
    pub scale: InterfaceScale,
    pub control_font: f32,
    pub control_xs_font: f32,
    /// Icon buttons and default controls (`--button-height`).
    pub button: f32,
    pub button_sm: f32,
    pub button_xs: f32,
    pub button_lg: f32,
    /// Icon glyph sizes: default (in a `button`), small, extra small and 2xs.
    pub icon: f32,
    pub icon_sm: f32,
    pub icon_xs: f32,
    pub icon_2xs: f32,
    pub sidebar_width: f32,
    pub sidebar_font: f32,
    pub sidebar_title_font: f32,
    /// The titlebar row that also holds the tabs and the sidebar header. The traffic lights
    /// are fixed by macOS, so it never drops below their 40px row.
    pub titlebar_height: f32,
    pub tab_height: f32,
    pub tab_max_width: f32,
    pub tab_icon: f32,
    pub tab_close: f32,
    pub breadcrumb_height: f32,
    pub breadcrumb_font: f32,
    pub breadcrumb_secondary_font: f32,
    /// Dimension factor relative to compact (`button / 28`).
    space_factor: f32,
    /// Type factor relative to compact (`control_font / 12`).
    text_factor: f32,
}

impl UiMetrics {
    pub const fn for_scale(scale: InterfaceScale) -> Self {
        // (control, control_xs, button, sm, xs, lg, sidebar, tab bar, tab, tab max, tab icon,
        //  close, breadcrumb, breadcrumb font, breadcrumb secondary)
        let (c, cxs, b, bsm, bxs, blg, sw, bar, tab, tmax, ticon, close, bc, bcf, bcs) = match scale
        {
            InterfaceScale::Compact => (
                12.0, 10.0, 28.0, 24.0, 20.0, 32.0, 244.0, 36.0, 28.0, 200.0, 14.0, 24.0, 28.0,
                11.0, 10.0,
            ),
            InterfaceScale::Comfortable => (
                13.0, 11.0, 32.0, 28.0, 24.0, 36.0, 264.0, 40.0, 32.0, 224.0, 16.0, 28.0, 32.0,
                12.0, 11.0,
            ),
            InterfaceScale::Large => (
                14.0, 12.0, 36.0, 32.0, 28.0, 40.0, 280.0, 44.0, 36.0, 248.0, 17.0, 32.0, 36.0,
                13.0, 12.0,
            ),
        };
        let step = (b - 28.0) / 2.0;
        Self {
            scale,
            control_font: c,
            control_xs_font: cxs,
            button: b,
            button_sm: bsm,
            button_xs: bxs,
            button_lg: blg,
            icon: 16.0 + step,
            icon_sm: 14.0 + step,
            icon_xs: 12.0 + step,
            icon_2xs: 10.0 + step,
            sidebar_width: sw,
            sidebar_font: c,
            sidebar_title_font: c - 1.0,
            titlebar_height: if bar > TrafficLights::titlebar_height() {
                bar
            } else {
                TrafficLights::titlebar_height()
            },
            tab_height: tab,
            tab_max_width: tmax,
            tab_icon: ticon,
            tab_close: close,
            breadcrumb_height: bc,
            breadcrumb_font: bcf,
            breadcrumb_secondary_font: bcs,
            space_factor: b / 28.0,
            text_factor: c / 12.0,
        }
    }

    /// Scales a compact dimension (padding, gap, row height), rounded to whole pixels.
    pub fn space(&self, compact: f32) -> f32 {
        (compact * self.space_factor).round()
    }

    /// Scales a compact font size, rounded to half pixels.
    pub fn text(&self, compact: f32) -> f32 {
        (compact * self.text_factor * 2.0).round() / 2.0
    }

    pub fn tree_row_height(&self) -> f32 {
        self.space(Metrics::TREE_ROW_HEIGHT)
    }

    pub fn outline_row_height(&self) -> f32 {
        self.space(Metrics::OUTLINE_ROW_HEIGHT)
    }

    pub fn recent_row_height(&self) -> f32 {
        self.space(Metrics::RECENT_ROW_HEIGHT)
    }

    pub fn menu_row_height(&self) -> f32 {
        self.button
    }

    pub fn segmented_height(&self) -> f32 {
        self.button
    }

    pub fn section_header_height(&self) -> f32 {
        self.space(Metrics::SECTION_HEADER_HEIGHT)
    }

    /// Top padding that vertically centres a tab inside the titlebar row.
    pub fn tab_inset_top(&self) -> f32 {
        (self.titlebar_height - self.tab_height) / 2.0
    }
}

impl Default for UiMetrics {
    fn default() -> Self {
        Self::for_scale(InterfaceScale::Compact)
    }
}

/// The interface scale the focused window renders with, for views that only know their theme
/// mode (palette, find, shortcuts). Set by the app shell whenever the preference changes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActiveUiScale(pub InterfaceScale);

impl Global for ActiveUiScale {}

pub fn active_ui_scale(cx: &App) -> InterfaceScale {
    cx.try_global::<ActiveUiScale>()
        .map(|scale| scale.0)
        .unwrap_or_default()
}

const _: () = assert!(
    TrafficLights::titlebar_height()
        <= 2.0 * TrafficLights::NATIVE_TITLEBAR - TrafficLights::BUTTON_DIAMETER
);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub x: f32,
    pub width: f32,
}

/// What sits at the leading edge of the titlebar row (the tab bar or the empty-state toolbar).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitlebarLeading {
    /// Space kept clear for the traffic lights; zero when the sidebar header already holds them.
    pub clearance: f32,
    /// Whether the sidebar toggle lives in this row (it moves to the sidebar header otherwise).
    pub sidebar_toggle: bool,
    /// Size of the sidebar toggle's slot at the current interface scale.
    pub button: f32,
}

impl TitlebarLeading {
    pub fn width(self) -> f32 {
        self.clearance
            + if self.sidebar_toggle {
                self.button
            } else {
                0.0
            }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellLayout {
    pub titlebar: TitlebarLayout,
    pub titlebar_leading: TitlebarLeading,
    pub sidebar: Region,
    pub main: Region,
    pub reader: Region,
    pub tab_bar_height: f32,
    pub tab_height: f32,
    pub breadcrumb_height: f32,
}

impl ShellLayout {
    pub fn for_width(window_width: f32, sidebar_open: bool, wide_mode: bool) -> Self {
        Self::for_width_scaled(window_width, sidebar_open, wide_mode, UiMetrics::default())
    }

    pub fn for_width_scaled(
        window_width: f32,
        sidebar_open: bool,
        wide_mode: bool,
        ui: UiMetrics,
    ) -> Self {
        let window_width = window_width.max(0.0);
        let sidebar_width = if sidebar_open
            && window_width >= ui.sidebar_width + Metrics::MIN_MAIN_WIDTH_WITH_SIDEBAR
        {
            ui.sidebar_width
        } else {
            0.0
        };
        let main_width = (window_width - sidebar_width).max(0.0);
        let reader_width = if wide_mode {
            (main_width - Metrics::READER_INSET * 2.0).max(0.0)
        } else {
            Metrics::READER_MAX_WIDTH.min(main_width)
        };
        let reader_inset = if wide_mode {
            Metrics::READER_INSET.min(main_width)
        } else {
            ((main_width - reader_width) / 2.0).max(0.0)
        };

        let titlebar = TitlebarLayout::standard();
        let titlebar_leading = if sidebar_width > 0.0 {
            TitlebarLeading {
                clearance: 0.0,
                sidebar_toggle: false,
                button: ui.button,
            }
        } else {
            TitlebarLeading {
                clearance: titlebar.clearance.width(),
                sidebar_toggle: true,
                button: ui.button,
            }
        };
        Self {
            titlebar,
            titlebar_leading,
            sidebar: Region {
                x: 0.0,
                width: sidebar_width,
            },
            main: Region {
                x: sidebar_width,
                width: main_width,
            },
            reader: Region {
                x: sidebar_width + reader_inset,
                width: reader_width,
            },
            tab_bar_height: ui.titlebar_height,
            tab_height: ui.tab_height,
            breadcrumb_height: ui.breadcrumb_height,
        }
    }
}

impl ShellLayout {
    /// Chrome stacked above the reader when a document is open: tab row plus breadcrumb.
    pub fn chrome_height(&self) -> f32 {
        self.tab_bar_height + self.breadcrumb_height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorScheme {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub color_scheme: ColorScheme,
    pub background: Hsla,
    pub foreground: Hsla,
    pub card: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub primary: Hsla,
    pub accent: Hsla,
    pub destructive: Hsla,
    pub border: Hsla,
    pub border_subtle: Hsla,
    pub sidebar: Hsla,
    pub sidebar_accent: Hsla,
    pub surface_raised: Hsla,
    pub surface_well: Hsla,
    /// Electron's `--md-alert-*` OKLCH tokens, converted to sRGB HSL.
    pub alert_note: Hsla,
    pub alert_tip: Hsla,
    pub alert_important: Hsla,
    pub alert_warning: Hsla,
    pub alert_caution: Hsla,
    /// Code block surface: the muted well in light mode, a slightly lifted gray in dark mode.
    pub code_surface: Hsla,
    /// Chrome sizes for the interface scale.
    pub ui: UiMetrics,
}

impl Theme {
    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        Self::resolve(ThemeMode::System, appearance)
    }

    pub fn resolve(mode: ThemeMode, appearance: WindowAppearance) -> Self {
        let system_is_dark = matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        if mode.is_dark(system_is_dark) {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// The same palette with chrome sized for `scale`.
    pub fn scaled(mut self, scale: InterfaceScale) -> Self {
        self.ui = UiMetrics::for_scale(scale);
        self
    }

    fn light() -> Self {
        Self {
            color_scheme: ColorScheme::Light,
            background: hsla(0.08672199, 0.39970066, 0.97152986, 1.0),
            foreground: hsla(0.04368636, 0.694_890_4, 0.03135708, 1.0),
            card: hsla(0.08672199, 0.39970066, 0.97152986, 1.0),
            muted: hsla(0.08673897, 0.24669178, 0.944_926_9, 1.0),
            muted_foreground: hsla(0.05796655, 0.08543156, 0.33432802, 1.0),
            primary: hsla(0.60388106, 0.64902184, 0.505_344_5, 1.0),
            accent: hsla(0.08304337, 1.0, 0.40092257, 1.0),
            destructive: hsla(0.99228718, 0.682_701_2, 0.47648946, 1.0),
            border: hsla(0.08681399, 0.15087865, 0.85268928, 1.0),
            border_subtle: hsla(0.086_774_1, 0.189_319_6, 0.90505948, 1.0),
            sidebar: hsla(0.08672199, 0.39970066, 0.97152986, 1.0),
            sidebar_accent: hsla(0.08677273, 0.21983283, 0.918_019, 1.0),
            surface_raised: hsla(0.08672199, 0.28, 0.992, 1.0),
            surface_well: hsla(0.08673897, 0.24669178, 0.93, 1.0),
            // oklch(0.55 0.17 255), (0.55 0.14 150), (0.55 0.17 295), (0.6 0.13 75),
            // (0.56 0.19 25)
            alert_note: hsla(0.586_368, 0.815_088, 0.452_324, 1.0),
            alert_tip: hsla(0.392_263, 0.652_996, 0.320_923, 1.0),
            alert_important: hsla(0.720_128, 0.510_306, 0.562_534, 1.0),
            alert_warning: hsla(0.110_730, 1.0, 0.338_5, 1.0),
            alert_caution: hsla(0.996_916, 0.601_668, 0.500_13, 1.0),
            code_surface: hsla(0.08673897, 0.24669178, 0.944_926_9, 1.0),
            ui: UiMetrics::for_scale(InterfaceScale::Compact),
        }
    }

    fn dark() -> Self {
        Self {
            color_scheme: ColorScheme::Dark,
            background: hsla(0.0, 0.0, 0.03545248, 1.0),
            foreground: hsla(0.0, 0.0, 0.895_576_9, 1.0),
            card: hsla(0.0, 0.0, 0.03545248, 1.0),
            muted: hsla(0.0, 0.0, 0.07734101, 1.0),
            muted_foreground: hsla(0.0, 0.0, 0.56073545, 1.0),
            primary: hsla(0.60397774, 0.86814313, 0.664_164_5, 1.0),
            accent: hsla(0.114_587_8, 0.791_532_5, 0.48821926, 1.0),
            destructive: hsla(0.997_840_4, 0.715_155_9, 0.552_315_2, 1.0),
            border: hsla(0.0, 0.0, 0.15033225, 1.0),
            border_subtle: hsla(0.0, 0.0, 0.10395742, 1.0),
            sidebar: hsla(0.0, 0.0, 0.03545248, 1.0),
            sidebar_accent: hsla(0.0, 0.0, 0.086_104_2, 1.0),
            surface_raised: hsla(0.0, 0.0, 0.09, 1.0),
            surface_well: hsla(0.0, 0.0, 0.06, 1.0),
            // oklch(0.68 0.14 255), (0.68 0.13 150), (0.68 0.14 295), (0.74 0.13 75),
            // (0.66 0.17 25)
            alert_note: hsla(0.592_697, 0.800_9, 0.637_297, 1.0),
            alert_tip: hsla(0.375_692, 0.355_474, 0.509_685, 1.0),
            alert_important: hsla(0.712_244, 0.647_682, 0.709_353, 1.0),
            alert_warning: hsla(0.101_535, 0.678_357, 0.552_497, 1.0),
            alert_caution: hsla(0.006_023, 0.751_532, 0.632_338, 1.0),
            // The mockup's `.dark .cb2 { background: hsl(0 0% 6.5%) }`.
            code_surface: hsla(0.0, 0.0, 0.065, 1.0),
            ui: UiMetrics::for_scale(InterfaceScale::Compact),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{WindowAppearance, hsla};

    #[test]
    fn traffic_lights_geometry_is_derived_from_one_source() {
        assert_eq!(f32::from(TrafficLights::position().y), TrafficLights::INSET);
        assert_eq!(
            TrafficLights::titlebar_height(),
            2.0 * TrafficLights::INSET + TrafficLights::BUTTON_DIAMETER
        );
        assert_eq!(TrafficLights::titlebar_height(), 40.0);
        assert_eq!(TrafficLightClearance::reserved().width(), 80.0);
        assert!(
            TrafficLights::titlebar_height()
                <= 2.0 * TrafficLights::NATIVE_TITLEBAR - TrafficLights::BUTTON_DIAMETER
        );
        assert_eq!(TitlebarLayout::standard().height, 40.0);
        assert_eq!(TitlebarLayout::standard().controls_x, 80.0);
    }

    #[test]
    fn compact_shell_allocates_established_chrome_and_reader_regions() {
        let layout = ShellLayout::for_width(1120.0, true, false);

        assert_eq!(layout.titlebar, TitlebarLayout::standard());
        assert_eq!(
            layout.sidebar,
            Region {
                x: 0.0,
                width: 244.0,
            }
        );
        assert_eq!(
            layout.main,
            Region {
                x: 244.0,
                width: 876.0,
            }
        );
        assert_eq!(layout.tab_bar_height, 40.0);
        assert_eq!(layout.tab_bar_height, TrafficLights::titlebar_height());
        assert_eq!(Metrics::SIDEBAR_HEADER_HEIGHT, layout.tab_bar_height);
        assert_eq!(layout.tab_height, 28.0);
        assert_eq!((Metrics::TAB_BAR_HEIGHT - Metrics::TAB_HEIGHT) / 2.0, 6.0);
        assert_eq!(layout.chrome_height(), 68.0);
        assert_eq!(
            layout.titlebar_leading,
            TitlebarLeading {
                clearance: 0.0,
                sidebar_toggle: false,
                button: 28.0,
            }
        );
        assert_eq!(layout.titlebar_leading.width(), 0.0);
        assert_eq!(Metrics::TAB_RADIUS, 6.0);
        assert_eq!(layout.breadcrumb_height, 28.0);
        assert_eq!(Metrics::READER_TOP_PADDING, 32.0);
        assert_eq!(
            layout.reader,
            Region {
                x: 298.0,
                width: 768.0,
            }
        );
    }

    #[test]
    fn wide_mode_uses_the_available_main_width_with_fixed_insets() {
        let layout = ShellLayout::for_width(1120.0, true, true);

        assert_eq!(
            layout.reader,
            Region {
                x: 276.0,
                width: 812.0,
            }
        );
    }

    #[test]
    fn hidden_sidebar_gives_the_entire_window_to_the_main_region() {
        let layout = ShellLayout::for_width(1120.0, false, false);

        assert_eq!(layout.sidebar, Region { x: 0.0, width: 0.0 });
        assert_eq!(
            layout.main,
            Region {
                x: 0.0,
                width: 1120.0,
            }
        );
        assert_eq!(
            layout.titlebar_leading,
            TitlebarLeading {
                clearance: TrafficLightClearance::reserved().width(),
                sidebar_toggle: true,
                button: 28.0,
            }
        );
        assert_eq!(layout.titlebar_leading.width(), 80.0 + 28.0);
        assert_eq!(layout.chrome_height(), 68.0);
        assert_eq!(
            layout.reader,
            Region {
                x: 176.0,
                width: 768.0,
            }
        );
    }

    #[test]
    fn narrow_windows_auto_collapse_the_sidebar_to_keep_the_main_region_usable() {
        let layout = ShellLayout::for_width(180.0, true, false);

        assert_eq!(layout.sidebar, Region { x: 0.0, width: 0.0 });
        assert_eq!(
            layout.main,
            Region {
                x: 0.0,
                width: 180.0
            }
        );
        assert_eq!(
            layout.reader,
            Region {
                x: 0.0,
                width: 180.0
            }
        );
    }

    #[test]
    fn compact_ui_metrics_match_the_fixed_metrics() {
        let ui = UiMetrics::default();
        assert_eq!(ui.scale, InterfaceScale::Compact);
        assert_eq!(ui.control_font, Metrics::CONTROL_FONT_SIZE);
        assert_eq!(ui.button, Metrics::TITLEBAR_BUTTON);
        assert_eq!(ui.icon, Metrics::ICON_SIZE);
        assert_eq!(ui.sidebar_width, Metrics::SIDEBAR_WIDTH);
        assert_eq!(ui.titlebar_height, Metrics::TAB_BAR_HEIGHT);
        assert_eq!(ui.tab_height, Metrics::TAB_HEIGHT);
        assert_eq!(ui.tab_max_width, Metrics::TAB_MAX_WIDTH);
        assert_eq!(ui.tab_icon, Metrics::TAB_ICON_SIZE);
        assert_eq!(ui.tab_close, Metrics::TAB_CLOSE_SIZE);
        assert_eq!(ui.breadcrumb_height, Metrics::BREADCRUMB_HEIGHT);
        assert_eq!(ui.tree_row_height(), Metrics::TREE_ROW_HEIGHT);
        assert_eq!(ui.menu_row_height(), Metrics::MENU_ROW_HEIGHT);
        assert_eq!(ui.segmented_height(), Metrics::SEGMENTED_HEIGHT);
        assert_eq!(ui.space(7.0), 7.0);
        assert_eq!(ui.text(10.5), 10.5);
    }

    #[test]
    fn larger_scales_follow_electrons_token_tables() {
        let comfortable = UiMetrics::for_scale(InterfaceScale::Comfortable);
        let large = UiMetrics::for_scale(InterfaceScale::Large);

        // --control-font-size, --button-height, --tab-height, --breadcrumb-height, sidebar.
        assert_eq!((comfortable.control_font, large.control_font), (13.0, 14.0));
        assert_eq!((comfortable.button, large.button), (32.0, 36.0));
        assert_eq!((comfortable.tab_height, large.tab_height), (32.0, 36.0));
        assert_eq!((comfortable.tab_close, large.tab_close), (28.0, 32.0));
        assert_eq!(
            (comfortable.breadcrumb_height, large.breadcrumb_height),
            (32.0, 36.0)
        );
        assert_eq!(
            (comfortable.breadcrumb_font, large.breadcrumb_font),
            (12.0, 13.0)
        );
        assert_eq!(
            (comfortable.sidebar_width, large.sidebar_width),
            (264.0, 280.0)
        );
        assert_eq!((comfortable.icon, large.icon), (18.0, 20.0));
        assert_eq!(comfortable.space(28.0), 32.0);
        assert_eq!(large.space(28.0), 36.0);
        assert_eq!(large.text(12.0), 14.0);
    }

    #[test]
    fn the_titlebar_row_never_drops_below_the_traffic_light_row() {
        for scale in [
            InterfaceScale::Compact,
            InterfaceScale::Comfortable,
            InterfaceScale::Large,
        ] {
            let ui = UiMetrics::for_scale(scale);
            assert!(ui.titlebar_height >= TrafficLights::titlebar_height());
            assert!(ui.tab_height < ui.titlebar_height);
            // Tabs sit in the vertical centre of the row.
            assert_eq!(ui.tab_inset_top() * 2.0 + ui.tab_height, ui.titlebar_height);
        }
        assert_eq!(
            UiMetrics::for_scale(InterfaceScale::Comfortable).titlebar_height,
            40.0
        );
        assert_eq!(
            UiMetrics::for_scale(InterfaceScale::Large).titlebar_height,
            44.0
        );
    }

    #[test]
    fn scaled_shell_layout_grows_the_sidebar_and_chrome_but_keeps_the_clearance() {
        let ui = UiMetrics::for_scale(InterfaceScale::Large);
        let layout = ShellLayout::for_width_scaled(1120.0, true, false, ui);
        assert_eq!(layout.sidebar.width, 280.0);
        assert_eq!(layout.main.x, 280.0);
        assert_eq!(layout.tab_bar_height, 44.0);
        assert_eq!(layout.tab_height, 36.0);
        assert_eq!(layout.breadcrumb_height, 36.0);
        assert_eq!(layout.chrome_height(), 80.0);

        let hidden = ShellLayout::for_width_scaled(1120.0, false, false, ui);
        assert_eq!(hidden.titlebar_leading.clearance, 80.0);
        assert_eq!(hidden.titlebar_leading.width(), 80.0 + 36.0);
        assert_eq!(
            Theme::for_appearance(WindowAppearance::Dark)
                .scaled(InterfaceScale::Large)
                .ui,
            ui
        );
    }

    #[test]
    fn light_appearances_use_the_warm_electron_palette() {
        let light = Theme::for_appearance(WindowAppearance::Light);
        let vibrant = Theme::for_appearance(WindowAppearance::VibrantLight);

        assert_eq!(light, vibrant);
        assert_eq!(light.color_scheme, ColorScheme::Light);
        assert_eq!(
            light.background,
            hsla(0.08672199, 0.39970066, 0.97152986, 1.0)
        );
        assert_eq!(
            light.foreground,
            hsla(0.04368636, 0.694_890_4, 0.03135708, 1.0)
        );
        assert_eq!(
            light.primary,
            hsla(0.60388106, 0.64902184, 0.505_344_5, 1.0)
        );
        assert_eq!(
            light.sidebar_accent,
            hsla(0.08677273, 0.21983283, 0.918_019, 1.0)
        );
    }

    #[test]
    fn dark_appearances_use_the_neutral_electron_palette() {
        let dark = Theme::for_appearance(WindowAppearance::Dark);
        let vibrant = Theme::for_appearance(WindowAppearance::VibrantDark);

        assert_eq!(dark, vibrant);
        assert_eq!(dark.color_scheme, ColorScheme::Dark);
        assert_eq!(dark.background, hsla(0.0, 0.0, 0.03545248, 1.0));
        assert_eq!(dark.foreground, hsla(0.0, 0.0, 0.895_576_9, 1.0));
        assert_eq!(dark.border, hsla(0.0, 0.0, 0.15033225, 1.0));
        assert_eq!(dark.sidebar_accent, hsla(0.0, 0.0, 0.086_104_2, 1.0));
    }
}
