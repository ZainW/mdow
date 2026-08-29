use crate::prefs::ThemeMode;
use gpui::{Hsla, Pixels, Point, WindowAppearance, hsla, point, px};

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
    pub const TAB_BAR_HEIGHT: f32 = 36.0;
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
    pub const READER_INSET: f32 = 48.0;
    pub const READER_TOP_PADDING: f32 = 32.0;
    pub const READER_BOTTOM_PADDING: f32 = 40.0;
    pub const RADIUS: f32 = 8.0;
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellLayout {
    pub titlebar: TitlebarLayout,
    pub sidebar: Region,
    pub main: Region,
    pub reader: Region,
    pub tab_bar_height: f32,
    pub tab_height: f32,
    pub breadcrumb_height: f32,
}

impl ShellLayout {
    pub fn for_width(window_width: f32, sidebar_open: bool, wide_mode: bool) -> Self {
        let window_width = window_width.max(0.0);
        let sidebar_width = if sidebar_open
            && window_width >= Metrics::SIDEBAR_WIDTH + Metrics::MIN_MAIN_WIDTH_WITH_SIDEBAR
        {
            Metrics::SIDEBAR_WIDTH
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

        Self {
            titlebar: TitlebarLayout::standard(),
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
            tab_bar_height: Metrics::TAB_BAR_HEIGHT,
            tab_height: Metrics::TAB_HEIGHT,
            breadcrumb_height: Metrics::BREADCRUMB_HEIGHT,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        assert_eq!(layout.tab_bar_height, 36.0);
        assert_eq!(layout.tab_height, 28.0);
        assert_eq!((Metrics::TAB_BAR_HEIGHT - Metrics::TAB_HEIGHT) / 2.0, 4.0);
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
                x: 292.0,
                width: 780.0,
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
