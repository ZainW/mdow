//! Mdow's colours and type, shared by the reader and the window around it. The values are the
//! Swift app's `Palette` (warm paper in light mode, neutral ink in dark, one clay accent).

use rune::{Color, Family, TextStyle};
use std::cell::RefCell;

pub const HEADER_HEIGHT: f32 = 40.0;
/// The margin between the document card and the window's edges.
pub const GUTTER: f32 = 8.0;
pub const CARD_RADIUS: f32 = 12.0;
/// Room for the close, minimise and zoom buttons at the left of the header.
pub const TRAFFIC_LIGHTS_WIDTH: f32 = 86.0;

/// The interface and code families, embedded from the Linux app's assets.
pub const UI_FAMILY: &str = "Inter Variable";
pub const MONO_FAMILY: &str = "Geist Mono";

pub static FONTS: &[&[u8]] = &[
    include_bytes!("../../native-linux/assets/fonts/Inter-400.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Inter-500.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Inter-600.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Inter-700.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Inter-Italic-400.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Inter-Italic-600.ttf"),
    include_bytes!("../../native-linux/assets/fonts/GeistMono-400.ttf"),
    include_bytes!("../../native-linux/assets/fonts/GeistMono-500.ttf"),
    include_bytes!("../../native-linux/assets/fonts/GeistMono-700.ttf"),
    include_bytes!("../../native-linux/assets/fonts/GeistMono-Italic-400.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Charter-Regular.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Charter-Bold.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Charter-Italic.ttf"),
    include_bytes!("../../native-linux/assets/fonts/Charter-BoldItalic.ttf"),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// Which palette this is, for caches of things drawn in its colours: 0 and 1 are Mdow's
    /// light and dark, anything else an Omarchy theme.
    pub id: u32,
    pub dark: bool,
    /// The corner radius of the document card and the cards like it.
    pub radius: f32,
    /// The window itself: sidebar, tab bar and the margin around the document.
    pub chrome: Color,
    /// The card the document sits on.
    pub paper: Color,
    /// Things that float: the active tab, menus, the command palette.
    pub elevated: Color,
    /// Recessed areas inside the paper, like code blocks.
    pub well: Color,
    pub text: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub line: Color,
    pub strong_line: Color,
    pub hover: Color,
    pub selected: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub selection: Color,
    pub inline_code_bg: Color,
    pub quote_bar: Color,
    pub mark: Color,
    pub find_match: Color,
    pub find_current: Color,
    pub shadow: Color,
    /// Syntax colours: comments, keywords, strings, entities and constants.
    pub syntax: [Color; 5],
    /// Alert colours: note, tip, important, warning and caution.
    pub alerts: [Color; 5],
}

/// The palette for a setting: Mdow's own light or dark, or with Automatic the Omarchy theme
/// where there is one, else the system's light or dark.
pub fn current(appearance: crate::prefs::Appearance, system_dark: bool) -> Palette {
    use crate::prefs::Appearance;
    match appearance {
        Appearance::Dark => Palette::get(true),
        Appearance::Light => Palette::get(false),
        Appearance::System => {
            crate::omarchy::palette().unwrap_or_else(|| Palette::get(system_dark))
        }
    }
}

fn rgb(hex: u32) -> Color {
    rgba(hex, 1.0)
}

fn rgba(hex: u32, alpha: f32) -> Color {
    rune::color::hex(&format!("#{hex:06x}")).unwrap_or(Color::BLACK).alpha(alpha)
}

fn srgb(r: f32, g: f32, b: f32, a: f32) -> Color {
    let byte = |c: f32| (c * 255.0).round() as u32;
    rgba((byte(r) << 16) | (byte(g) << 8) | byte(b), a)
}

impl Palette {
    pub fn get(dark: bool) -> Palette {
        thread_local! {
            static CACHE: RefCell<[Option<Palette>; 2]> = const { RefCell::new([None, None]) };
        }
        CACHE.with(|c| *c.borrow_mut()[dark as usize].get_or_insert_with(|| Palette::build(dark)))
    }

    fn build(dark: bool) -> Palette {
        let pair = |l: u32, d: u32| if dark { rgb(d) } else { rgb(l) };
        // The light theme's warm near-black, and white, at an opacity.
        let ink = |a: f32| if dark { rgba(0xFFFFFF, a) } else { rgba(0x3C2D1E, a) };
        Palette {
            id: dark as u32,
            dark,
            radius: CARD_RADIUS,
            chrome: pair(0xF3F1EB, 0x131313),
            paper: pair(0xFDFCF9, 0x1B1B1B),
            elevated: pair(0xFFFFFF, 0x242424),
            well: pair(0xF5F3ED, 0x161616),
            text: pair(0x1E1C19, 0xECECEC),
            secondary: pair(0x6B665E, 0xA3A3A3),
            tertiary: pair(0x9E998F, 0x6E6E6E),
            line: if dark { ink(0.08) } else { ink(0.1) },
            strong_line: if dark { ink(0.14) } else { ink(0.16) },
            hover: ink(0.05),
            selected: ink(0.085),
            accent: pair(0xB95A33, 0xE08A66),
            accent_soft: if dark { rgba(0xE08A66, 0.16) } else { rgba(0xB95A33, 0.12) },
            selection: if dark { rgba(0xE08A66, 0.3) } else { rgba(0xD97757, 0.24) },
            inline_code_bg: if dark { rgba(0xFFFFFF, 0.08) } else { srgb(0.24, 0.18, 0.12, 0.07) },
            quote_bar: if dark { rgba(0xFFFFFF, 0.18) } else { srgb(0.24, 0.18, 0.12, 0.18) },
            mark: if dark { srgb(0.95, 0.75, 0.25, 0.35) } else { srgb(1.0, 0.85, 0.35, 0.55) },
            find_match: if dark { srgb(1.0, 0.8, 0.3, 0.22) } else { srgb(1.0, 0.8, 0.3, 0.35) },
            find_current: if dark {
                srgb(1.0, 0.72, 0.25, 0.6)
            } else {
                srgb(1.0, 0.72, 0.2, 0.85)
            },
            shadow: if dark { rgba(0x000000, 0.5) } else { rgba(0x3C2D1E, 0.12) },
            // Soft enough for paper and ink.
            syntax: [
                pair(0x8C877D, 0x7A7A7A),
                pair(0xA3456B, 0xE38CB0),
                pair(0x4F7A3A, 0x9CC48A),
                pair(0x7A5AA8, 0xB9A0E8),
                pair(0xA65A1E, 0xE5A574),
            ],
            alerts: [
                pair(0x0969DA, 0x4493F8),
                pair(0x1A7F37, 0x3FB950),
                pair(0x8250DF, 0xAB7DF8),
                pair(0x9A6700, 0xD29922),
                pair(0xCF222E, 0xF85149),
            ],
        }
    }

    /// Alert colours: (accent, label) by kind.
    pub fn alert(&self, kind: crate::doc::AlertKind) -> (Color, &'static str) {
        use crate::doc::AlertKind::*;
        let (i, label) = match kind {
            Note => (0, "Note"),
            Tip => (1, "Tip"),
            Important => (2, "Important"),
            Warning => (3, "Warning"),
            Caution => (4, "Caution"),
        };
        (self.alerts[i], label)
    }

    /// Syntax colours for the highlighter's six roles.
    pub fn syntax(&self, tok: crate::highlight::Tok) -> Option<Color> {
        use crate::highlight::Tok::*;
        Some(match tok {
            Plain => return None,
            Comment => self.syntax[0],
            Keyword => self.syntax[1],
            String => self.syntax[2],
            Entity => self.syntax[3],
            Constant => self.syntax[4],
        })
    }

    pub fn kit(&self) -> rune_kit::Kit {
        thread_local! {
            static KIT: RefCell<Vec<(u32, rune_kit::Kit)>> = const { RefCell::new(Vec::new()) };
        }
        let p = *self;
        KIT.with(|c| {
            let mut kits = c.borrow_mut();
            if let Some((_, kit)) = kits.iter().find(|(id, _)| *id == p.id) {
                return kit.clone();
            }
            // Mdow's two, and the Omarchy theme: a theme change replaces the last.
            kits.retain(|(id, _)| *id < 2);
            let mut kit = rune_kit::Kit::from_palette(rune_kit::PaletteSeed {
                bg: p.elevated,
                fg: p.text,
                accent: p.accent,
            });
            kit.c.border_focus = p.accent.alpha(0.6);
            kits.push((p.id, kit.clone()));
            kit
        })
    }
}

/// A UI style in Inter.
pub fn ui(size: f32, weight: f32) -> TextStyle {
    TextStyle { family: Family::Named(UI_FAMILY), ..TextStyle::new(size, weight) }
}

/// Geist Mono, for paths, key hints and code.
pub fn mono(size: f32, weight: f32) -> TextStyle {
    TextStyle {
        family: Family::Named(MONO_FAMILY),
        features: "'liga' 0, 'calt' 0",
        ..TextStyle::new(size, weight)
    }
}
