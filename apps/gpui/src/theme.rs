use gpui::{Hsla, WindowAppearance, hsla};

pub struct Metrics;

impl Metrics {
    pub const FONT_SANS: &'static str = "Inter Variable";
    pub const FONT_MONO: &'static str = "Geist Mono";
    pub const APP_FONT_SIZE: f32 = 13.0;
    pub const CONTROL_FONT_SIZE: f32 = 12.0;
    pub const ICON_SIZE: f32 = 16.0;
    pub const SIDEBAR_WIDTH: f32 = 244.0;
    pub const MIN_MAIN_WIDTH_WITH_SIDEBAR: f32 = 320.0;
    pub const TITLEBAR_INSET: f32 = 28.0;
    pub const TAB_BAR_HEIGHT: f32 = 36.0;
    pub const TAB_HEIGHT: f32 = 28.0;
    pub const TAB_MAX_WIDTH: f32 = 200.0;
    pub const BREADCRUMB_HEIGHT: f32 = 28.0;
    pub const READER_MAX_WIDTH: f32 = 768.0;
    pub const READER_INSET: f32 = 48.0;
    pub const READER_TOP_PADDING: f32 = 22.0;
    pub const READER_BOTTOM_PADDING: f32 = 40.0;
    pub const RADIUS: f32 = 8.0;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub x: f32,
    pub width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellLayout {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
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
}

impl Theme {
    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
        }
    }

    fn light() -> Self {
        Self {
            background: hsla(0.08672199, 0.39970066, 0.97152986, 1.0),
            foreground: hsla(0.04368636, 0.69489038, 0.03135708, 1.0),
            card: hsla(0.08672199, 0.39970066, 0.97152986, 1.0),
            muted: hsla(0.08673897, 0.24669178, 0.94492692, 1.0),
            muted_foreground: hsla(0.05796655, 0.08543156, 0.33432802, 1.0),
            primary: hsla(0.60388106, 0.64902184, 0.50534449, 1.0),
            accent: hsla(0.08304337, 1.0, 0.40092257, 1.0),
            destructive: hsla(0.99228718, 0.68270120, 0.47648946, 1.0),
            border: hsla(0.08681399, 0.15087865, 0.85268928, 1.0),
            border_subtle: hsla(0.08677410, 0.18931960, 0.90505948, 1.0),
            sidebar: hsla(0.08672199, 0.39970066, 0.97152986, 1.0),
            sidebar_accent: hsla(0.08677273, 0.21983283, 0.91801898, 1.0),
        }
    }

    fn dark() -> Self {
        Self {
            background: hsla(0.0, 0.0, 0.03545248, 1.0),
            foreground: hsla(0.0, 0.0, 0.89557687, 1.0),
            card: hsla(0.0, 0.0, 0.03545248, 1.0),
            muted: hsla(0.0, 0.0, 0.07734101, 1.0),
            muted_foreground: hsla(0.0, 0.0, 0.56073545, 1.0),
            primary: hsla(0.60397774, 0.86814313, 0.66416450, 1.0),
            accent: hsla(0.11458780, 0.79153254, 0.48821926, 1.0),
            destructive: hsla(0.99784042, 0.71515589, 0.55231520, 1.0),
            border: hsla(0.0, 0.0, 0.15033225, 1.0),
            border_subtle: hsla(0.0, 0.0, 0.10395742, 1.0),
            sidebar: hsla(0.0, 0.0, 0.03545248, 1.0),
            sidebar_accent: hsla(0.0, 0.0, 0.08610420, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{WindowAppearance, hsla};

    #[test]
    fn compact_shell_allocates_established_chrome_and_reader_regions() {
        let layout = ShellLayout::for_width(1120.0, true, false);

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
        assert_eq!(layout.breadcrumb_height, 28.0);
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
        assert_eq!(
            light.background,
            hsla(0.08672199, 0.39970066, 0.97152986, 1.0)
        );
        assert_eq!(
            light.foreground,
            hsla(0.04368636, 0.69489038, 0.03135708, 1.0)
        );
        assert_eq!(light.primary, hsla(0.60388106, 0.64902184, 0.50534449, 1.0));
        assert_eq!(
            light.sidebar_accent,
            hsla(0.08677273, 0.21983283, 0.91801898, 1.0)
        );
    }

    #[test]
    fn dark_appearances_use_the_neutral_electron_palette() {
        let dark = Theme::for_appearance(WindowAppearance::Dark);
        let vibrant = Theme::for_appearance(WindowAppearance::VibrantDark);

        assert_eq!(dark, vibrant);
        assert_eq!(dark.background, hsla(0.0, 0.0, 0.03545248, 1.0));
        assert_eq!(dark.foreground, hsla(0.0, 0.0, 0.89557687, 1.0));
        assert_eq!(dark.border, hsla(0.0, 0.0, 0.15033225, 1.0));
        assert_eq!(dark.sidebar_accent, hsla(0.0, 0.0, 0.08610420, 1.0));
    }
}
