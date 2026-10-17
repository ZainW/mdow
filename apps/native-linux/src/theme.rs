//! The desktop app's design tokens (`apps/desktop/src/renderer/src/assets/styles/index.css`),
//! converted from OKLCH, and the GTK stylesheet built from them.

use crate::prefs::{InterfaceScale, Prefs};
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Rgba {
    pub fn hex(self) -> String {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}", c(self.r), c(self.g), c(self.b))
    }

    /// `rgba()` for CSS, keeping the alpha channel.
    pub fn css(self) -> String {
        if self.a >= 1.0 {
            return self.hex();
        }
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("rgba({},{},{},{:.3})", c(self.r), c(self.g), c(self.b), self.a)
    }

    pub fn alpha(self, a: f64) -> Rgba {
        Rgba { a: self.a * a, ..self }
    }

    /// Mix toward `other` by `t` (0 keeps self), like CSS `color-mix`.
    pub fn mix(self, other: Rgba, t: f64) -> Rgba {
        Rgba {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Composite over an opaque backdrop.
    pub fn over(self, backdrop: Rgba) -> Rgba {
        Rgba { a: 1.0, ..backdrop.mix(Rgba { a: 1.0, ..self }, self.a) }
    }

    pub fn u16s(self) -> (u16, u16, u16) {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 65535.0).round() as u16;
        (c(self.r), c(self.g), c(self.b))
    }
}

/// CSS `oklch(l c h)` in sRGB.
pub fn oklch(l: f64, c: f64, h: f64) -> Rgba {
    let h = h.to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());
    let l_ = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m_ = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s_ = (l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    let lin = [
        4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_,
        -1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_614_7 * m_ + 1.707_614_701 * s_,
    ];
    let gamma = |x: f64| {
        let x = x.clamp(0.0, 1.0);
        if x <= 0.003_130_8 { 12.92 * x } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 }
    };
    Rgba { r: gamma(lin[0]), g: gamma(lin[1]), b: gamma(lin[2]), a: 1.0 }
}

pub fn hex(s: &str) -> Rgba {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    Rgba {
        r: ((v >> 16) & 0xff) as f64 / 255.0,
        g: ((v >> 8) & 0xff) as f64 / 255.0,
        b: (v & 0xff) as f64 / 255.0,
        a: 1.0,
    }
}

#[derive(Clone, Debug)]
pub struct Syntax {
    pub fg: Rgba,
    pub comment: Rgba,
    pub keyword: Rgba,
    pub string: Rgba,
    pub entity: Rgba,
    pub constant: Rgba,
}

#[derive(Clone, Debug)]
pub struct Palette {
    pub dark: bool,
    pub background: Rgba,
    pub foreground: Rgba,
    pub card: Rgba,
    pub popover: Rgba,
    pub primary: Rgba,
    pub primary_fg: Rgba,
    pub secondary: Rgba,
    pub muted: Rgba,
    pub muted_fg: Rgba,
    pub muted_fg_subtle: Rgba,
    pub accent: Rgba,
    pub destructive: Rgba,
    pub border: Rgba,
    pub border_subtle: Rgba,
    pub input: Rgba,
    pub ring: Rgba,
    pub sidebar: Rgba,
    pub sidebar_accent: Rgba,
    pub alerts: [Rgba; 5],
    pub chart: [Rgba; 6],
    pub find_match: Rgba,
    pub find_match_fg: Option<Rgba>,
    pub find_active: Rgba,
    pub find_active_fg: Rgba,
    pub scrollbar: Rgba,
    pub scrollbar_hover: Rgba,
    pub syntax: Syntax,
}

impl Palette {
    pub fn light() -> Palette {
        Palette {
            dark: false,
            background: oklch(0.98, 0.005, 70.0),
            foreground: oklch(0.13, 0.02, 50.0),
            card: oklch(0.98, 0.005, 70.0),
            popover: oklch(1.0, 0.0, 0.0),
            primary: oklch(0.55, 0.17, 260.0),
            primary_fg: oklch(1.0, 0.0, 0.0),
            secondary: oklch(0.96, 0.006, 70.0),
            muted: oklch(0.96, 0.006, 70.0),
            muted_fg: oklch(0.45, 0.015, 50.0),
            muted_fg_subtle: oklch(0.52, 0.012, 50.0),
            accent: oklch(0.62, 0.16, 55.0),
            destructive: oklch(0.55, 0.2, 25.0),
            border: oklch(0.89, 0.01, 70.0),
            border_subtle: oklch(0.93, 0.008, 70.0),
            input: oklch(0.89, 0.01, 70.0),
            ring: oklch(0.55, 0.17, 260.0),
            sidebar: oklch(0.98, 0.005, 70.0),
            sidebar_accent: oklch(0.94, 0.008, 70.0),
            alerts: [
                oklch(0.55, 0.17, 255.0),
                oklch(0.55, 0.14, 150.0),
                oklch(0.55, 0.17, 295.0),
                oklch(0.6, 0.13, 75.0),
                oklch(0.56, 0.19, 25.0),
            ],
            chart: [
                oklch(0.62, 0.16, 55.0),
                oklch(0.6, 0.15, 160.0),
                oklch(0.55, 0.17, 260.0),
                oklch(0.65, 0.15, 310.0),
                oklch(0.7, 0.15, 30.0),
                hex("#0f8080"),
            ],
            find_match: oklch(0.93, 0.08, 85.0),
            find_match_fg: None,
            find_active: oklch(0.82, 0.12, 260.0),
            find_active_fg: oklch(0.15, 0.0, 0.0),
            scrollbar: oklch(0.6, 0.005, 70.0).alpha(0.25),
            scrollbar_hover: oklch(0.5, 0.01, 70.0).alpha(0.45),
            syntax: Syntax {
                fg: hex("#24292f"),
                comment: hex("#6e7781"),
                keyword: hex("#cf222e"),
                string: hex("#0a3069"),
                entity: hex("#8250df"),
                constant: hex("#0550ae"),
            },
        }
    }

    pub fn dark() -> Palette {
        Palette {
            dark: true,
            background: oklch(0.14, 0.0, 0.0),
            foreground: oklch(0.92, 0.0, 0.0),
            card: oklch(0.14, 0.0, 0.0),
            popover: oklch(0.17, 0.0, 0.0),
            primary: oklch(0.68, 0.15, 260.0),
            primary_fg: oklch(0.14, 0.0, 0.0),
            secondary: oklch(0.2, 0.0, 0.0),
            muted: oklch(0.19, 0.0, 0.0),
            muted_fg: oklch(0.65, 0.0, 0.0),
            muted_fg_subtle: oklch(0.6, 0.0, 0.0),
            accent: oklch(0.75, 0.15, 80.0),
            destructive: oklch(0.6, 0.2, 25.0),
            border: oklch(0.27, 0.0, 0.0),
            border_subtle: oklch(0.24, 0.0, 0.0),
            input: oklch(0.27, 0.0, 0.0),
            ring: oklch(0.68, 0.15, 260.0),
            sidebar: oklch(0.14, 0.0, 0.0),
            sidebar_accent: oklch(0.2, 0.0, 0.0),
            alerts: [
                oklch(0.68, 0.14, 255.0),
                oklch(0.68, 0.13, 150.0),
                oklch(0.68, 0.14, 295.0),
                oklch(0.74, 0.13, 75.0),
                oklch(0.66, 0.17, 25.0),
            ],
            chart: [
                oklch(0.75, 0.15, 80.0),
                oklch(0.65, 0.15, 160.0),
                oklch(0.68, 0.15, 260.0),
                oklch(0.7, 0.15, 310.0),
                oklch(0.75, 0.15, 30.0),
                hex("#5a9e98"),
            ],
            find_match: oklch(0.45, 0.08, 85.0),
            find_match_fg: Some(oklch(0.92, 0.0, 0.0)),
            find_active: oklch(0.45, 0.12, 260.0),
            find_active_fg: oklch(0.92, 0.0, 0.0),
            scrollbar: oklch(0.75, 0.0, 0.0).alpha(0.2),
            scrollbar_hover: oklch(0.75, 0.0, 0.0).alpha(0.4),
            syntax: Syntax {
                fg: hex("#e6edf3"),
                comment: hex("#8b949e"),
                keyword: hex("#ff7b72"),
                string: hex("#a5d6ff"),
                entity: hex("#d2a8ff"),
                constant: hex("#79c0ff"),
            },
        }
    }

    pub fn for_dark(dark: bool) -> Palette {
        if dark { Palette::dark() } else { Palette::light() }
    }
}

/// Reader typography: the desktop's 15.5px at a 1.65 line height, scaled by zoom.
pub const READER_FONT_SIZE: f64 = 15.5;
pub const READER_LINE_HEIGHT: f64 = 1.65;
/// Heading sizes (em of the body) and line heights, h1 to h6.
pub const HEADING_SCALE: [f64; 6] = [1.875, 1.5, 1.15, 1.0, 0.95, 0.875];
const HEADING_LINE: [f64; 6] = [1.2, 1.25, 1.3, 1.4, 1.4, 1.4];
const HEADING_WEIGHT: [u32; 6] = [700, 650, 600, 600, 600, 600];
const HEADING_TRACKING: [f64; 6] = [-0.025, -0.02, -0.01, 0.0, 0.0, 0.03];

struct ScaleTokens {
    control_font: f64,
    control_xs_font: f64,
    button: f64,
    button_xs: f64,
    sidebar_width: f64,
    sidebar_title: f64,
    sidebar_text: f64,
    row_pad_x: f64,
    row_pad_y: f64,
    tabbar: f64,
    tab: f64,
    breadcrumb: f64,
    breadcrumb_text: f64,
    breadcrumb_secondary: f64,
}

fn scale_tokens(scale: InterfaceScale) -> ScaleTokens {
    match scale {
        InterfaceScale::Compact => ScaleTokens {
            control_font: 12.0,
            control_xs_font: 10.0,
            button: 28.0,
            button_xs: 20.0,
            sidebar_width: 244.0,
            sidebar_title: 11.0,
            sidebar_text: 12.0,
            row_pad_x: 6.0,
            row_pad_y: 4.0,
            tabbar: 36.0,
            tab: 28.0,
            breadcrumb: 28.0,
            breadcrumb_text: 11.0,
            breadcrumb_secondary: 10.0,
        },
        InterfaceScale::Comfortable => ScaleTokens {
            control_font: 13.0,
            control_xs_font: 11.0,
            button: 32.0,
            button_xs: 24.0,
            sidebar_width: 264.0,
            sidebar_title: 12.0,
            sidebar_text: 13.0,
            row_pad_x: 8.0,
            row_pad_y: 5.0,
            tabbar: 40.0,
            tab: 32.0,
            breadcrumb: 32.0,
            breadcrumb_text: 12.0,
            breadcrumb_secondary: 11.0,
        },
        InterfaceScale::Large => ScaleTokens {
            control_font: 14.0,
            control_xs_font: 12.0,
            button: 36.0,
            button_xs: 28.0,
            sidebar_width: 280.0,
            sidebar_title: 13.0,
            sidebar_text: 14.0,
            row_pad_x: 10.0,
            row_pad_y: 6.0,
            tabbar: 44.0,
            tab: 36.0,
            breadcrumb: 36.0,
            breadcrumb_text: 13.0,
            breadcrumb_secondary: 12.0,
        },
    }
}

pub fn sidebar_width(scale: InterfaceScale) -> f64 {
    scale_tokens(scale).sidebar_width
}

/// Reader body size in px for a zoom percentage.
pub fn reader_font_size(zoom: u32) -> f64 {
    READER_FONT_SIZE * zoom as f64 / 100.0
}

/// The whole application stylesheet for one palette and set of preferences.
pub fn stylesheet(p: &Palette, prefs: &Prefs) -> String {
    let t = scale_tokens(prefs.interface_scale);
    let fs = reader_font_size(prefs.zoom_level);
    let content_font = prefs.content_font.family();
    let code_font = prefs.code_font.family();
    let mut css = String::with_capacity(24 * 1024);

    // Libadwaita's named colours, as legacy @define-color and as the 1.6+ custom properties,
    // so stock widgets (buttons, entries, popovers, dialogs) pick up the Mdow palette.
    let named: &[(&str, Rgba)] = &[
        ("window_bg_color", p.background),
        ("window_fg_color", p.foreground),
        ("view_bg_color", p.background),
        ("view_fg_color", p.foreground),
        ("headerbar_bg_color", p.background),
        ("headerbar_fg_color", p.foreground),
        ("headerbar_border_color", p.border_subtle),
        ("headerbar_backdrop_color", p.background),
        ("headerbar_shade_color", p.border_subtle),
        ("headerbar_darker_shade_color", p.border),
        ("sidebar_bg_color", p.sidebar),
        ("sidebar_fg_color", p.foreground),
        ("sidebar_backdrop_color", p.sidebar),
        ("sidebar_border_color", p.border_subtle),
        ("sidebar_shade_color", p.border_subtle),
        ("card_bg_color", p.card),
        ("card_fg_color", p.foreground),
        ("card_shade_color", p.border_subtle),
        ("dialog_bg_color", p.popover),
        ("dialog_fg_color", p.foreground),
        ("popover_bg_color", p.popover),
        ("popover_fg_color", p.foreground),
        ("popover_shade_color", p.border_subtle),
        ("accent_bg_color", p.primary),
        ("accent_fg_color", p.primary_fg),
        ("accent_color", p.primary),
        ("destructive_bg_color", p.destructive),
        ("destructive_fg_color", hex("#ffffff")),
        ("destructive_color", p.destructive),
        ("error_color", p.destructive),
    ];
    for (name, color) in named {
        let _ = writeln!(css, "@define-color {name} {};", color.css());
    }
    // GTK understands `:root` and custom properties from 4.16 on.
    if gtk::minor_version() >= 16 {
        css.push_str(":root {\n");
        for (name, color) in named {
            let _ = writeln!(css, "  --{}: {};", name.replace('_', "-"), color.css());
        }
        css.push_str("}\n");
    }

    let bg = p.background.css();
    let fg = p.foreground.css();
    let muted = p.muted.css();
    let muted_fg = p.muted_fg.css();
    let subtle = p.muted_fg_subtle.css();
    let border = p.border.css();
    let border_subtle = p.border_subtle.css();
    let primary = p.primary.css();
    let accent = p.accent.css();
    let sidebar = p.sidebar.css();
    let sidebar_accent = p.sidebar_accent.css();
    let card = p.card.css();
    let popover = p.popover.css();
    let destructive = p.destructive.css();
    let hover = p.muted.alpha(0.6).css();
    let shadow = if p.dark { "rgba(0,0,0,0.5)" } else { "rgba(0,0,0,0.08)" };

    let _ = write!(
        css,
        r#"
window.mdow, window.mdow-dialog, .mdow-dialog {{
  font-family: "Inter Variable", "Inter", sans-serif;
  font-size: 13px;
  font-feature-settings: "cv02", "cv03", "cv04", "cv11";
}}
window.mdow {{ background: {bg}; color: {fg}; }}
.mdow-dim {{ color: {muted_fg}; }}
.mdow-subtle {{ color: {subtle}; }}

/* Scrollbars: thin, rounded, quiet. */
.mdow scrollbar slider {{ background-color: {scroll}; min-width: 6px; min-height: 6px; border-radius: 999px; }}
.mdow scrollbar slider:hover {{ background-color: {scroll_hover}; }}
.mdow-sidebar scrollbar slider {{ min-width: 4px; min-height: 4px; }}

/* ── Sidebar ── */
.mdow-sidebar {{ background: {sidebar}; color: {fg}; border-right: 1px solid {border_subtle}; }}
.mdow-sidebar-header {{ padding: 8px 12px; border-bottom: 1px solid {border_subtle}; min-height: {tabbar_inner}px; }}
.mdow-sidebar-footer {{ padding: 8px; border-top: 1px solid {border_subtle}; }}
.mdow-mode-tabs {{ background: none; padding: 0; border-spacing: 2px; }}
.mdow-mode-tabs > button {{ min-height: 28px; padding: 0 4px; border-radius: 6px; font-size: {sidebar_title}px; color: {muted_fg}; background: none; box-shadow: none; }}
.mdow-mode-tabs > button:hover {{ background: {sidebar_hover}; color: {fg}; }}
.mdow-mode-tabs > button:checked {{ background: {sidebar_accent}; color: {fg}; box-shadow: none; }}
.mdow-mode-tabs separator {{ opacity: 0; min-width: 0; }}
.mdow-group-label {{ font-size: 11px; font-weight: 500; color: {subtle}; padding: 8px 10px 4px 10px; }}
.mdow-group-label .mdow-folder-name {{ font-weight: 400; opacity: 0.6; color: {muted_fg}; }}
.mdow-sidebar list, .mdow-sidebar listview {{ background: none; color: {fg}; }}
.mdow-sidebar row {{ background: none; padding: 0; min-height: 0; border-radius: 6px; margin: 0 6px 1px 6px; }}
.mdow-sidebar row:hover {{ background: {sidebar_hover}; }}
.mdow-sidebar row:selected, .mdow-sidebar row.mdow-active {{ background: {sidebar_accent}; color: {fg}; }}
.mdow-sidebar row:focus-visible {{ outline: 2px solid {ring}; outline-offset: -2px; }}
.mdow-sidebar row .mdow-row {{ padding: {row_pad_y}px {row_pad_x}px; font-size: {sidebar_text}px; }}
.mdow-sidebar row.mdow-active .mdow-row {{ box-shadow: inset 2px 0 0 {accent}; border-radius: 6px; }}
.mdow-row-icon {{ opacity: 0.4; }}
.mdow-row-dir {{ font-size: 10px; color: {subtle}; }}
.mdow-outline row .mdow-row {{ color: {sidebar_fg_dim}; }}
.mdow-outline row.mdow-active .mdow-row {{ color: {fg}; }}
.mdow-tree row .mdow-row {{ padding-top: 3px; padding-bottom: 3px; font-size: 13px; }}
.mdow-tree row .mdow-row-icon {{ opacity: 0.55; }}
.mdow-tree treeexpander indent {{ -gtk-icon-size: 12px; }}
.mdow-sidebar-footer button {{ min-height: 24px; padding: 2px 8px; font-size: {control_font}px; color: {muted_fg}; }}
.mdow-sidebar-footer button:hover {{ color: {fg}; }}
.mdow-filter {{ margin: 0 8px 8px 8px; min-height: 28px; font-size: {control_font}px; }}
.mdow-note {{ margin: 0 8px 4px 8px; padding: 6px 8px; border-radius: 6px; border: 1px solid {border_subtle}; background: {muted_soft}; font-size: 11px; color: {muted_fg}; }}
.mdow-match-count {{ margin: -4px 12px 6px 12px; font-size: 10px; color: {subtle}; }}
.mdow-empty {{ padding: 24px 12px; }}
.mdow-empty image {{ color: {empty_icon}; -gtk-icon-size: 20px; }}
.mdow-empty .mdow-empty-title {{ font-size: 12px; color: {sidebar_fg_dim}; }}
.mdow-empty .mdow-empty-hint {{ font-size: 11px; color: {subtle}; }}

/* ── Tab bar ── */
.mdow-tabbar {{ background: {bg}; border-bottom: 1px solid {border_subtle}; min-height: {tabbar}px; }}
.mdow-tabbar tabbar, .mdow-tabbar tabbar > revealer > box {{ background: none; box-shadow: none; border: none; padding: 0; min-height: {tabbar}px; }}
.mdow-tabbar tabbar tabbox {{ padding: 0 2px; }}
.mdow-tabbar tabbar tab {{
  min-height: {tab}px; margin: 4px 1px; padding: 0 2px 0 6px; border-radius: 6px;
  font-size: {control_font}px; color: {muted_fg}; background: none; box-shadow: none;
  transition: background-color 140ms cubic-bezier(0.23, 1, 0.32, 1), color 140ms cubic-bezier(0.23, 1, 0.32, 1);
}}
.mdow-tabbar tabbar tab:hover {{ background-color: {hover}; color: {fg}; }}
.mdow-tabbar tabbar tab:selected {{
  background-color: {card}; color: {fg};
  box-shadow: 0 0 0 1px {border_subtle}, 0 1px 0 {border_subtle}, 0 1px 2px rgba(0,0,0,0.04);
}}
.mdow-tabbar tabbar tab .tab-close-button {{ min-width: 20px; min-height: 20px; padding: 0; border-radius: 4px; opacity: 0.5; }}
.mdow-tabbar tabbar tab .tab-close-button:hover {{ opacity: 1; background: {muted_strong}; }}
.mdow-tabbar tabbar tab image {{ -gtk-icon-size: 14px; color: {muted_fg}; }}
.mdow-tabbar tabbar tab:selected image {{ color: {muted_fg}; }}
.mdow-tabbar tabbar tabbox > separator {{ background: {border_subtle}; min-width: 1px; margin: 11px 0; opacity: 1; }}
.mdow-tabbar tabbar .needs-attention {{ color: {accent}; }}
.mdow-tabbar .mdow-tab-actions {{ border-left: 1px solid {border_subtle}; padding: 0 6px; }}
.mdow-icon-button {{ min-width: {button_sm}px; min-height: {button_sm}px; padding: 0; border-radius: 6px; color: {muted_fg}; }}
.mdow-icon-button:hover {{ color: {fg}; }}
.mdow-icon-button:checked {{ background: {secondary}; color: {fg}; }}
.mdow-icon-button.mdow-quiet:checked {{ background: none; color: {muted_fg}; }}
.mdow-icon-button.mdow-quiet:checked:hover {{ background: {hover}; color: {fg}; }}
.mdow-icon-button image {{ -gtk-icon-size: 14px; }}

/* ── Breadcrumb ── */
.mdow-breadcrumb {{ min-height: {breadcrumb}px; padding: 0 12px; border-bottom: 1px solid {border_subtle}; background: {bg}; font-size: {breadcrumb_text}px; color: {subtle}; }}
.mdow-breadcrumb button.mdow-crumb {{ min-height: 0; min-width: 0; padding: 1px 2px; border-radius: 4px; font-weight: 400; color: {subtle}; background: none; }}
.mdow-breadcrumb button.mdow-crumb:hover {{ background: {muted}; color: {fg}; }}
.mdow-breadcrumb button.mdow-crumb-file {{ padding: 2px 4px; font-weight: 500; color: {fg_85}; }}
.mdow-breadcrumb .mdow-crumb-sep {{ color: {chevron}; -gtk-icon-size: 10px; }}
.mdow-breadcrumb .mdow-crumb-secondary {{ font-size: {breadcrumb_secondary}px; color: {subtle}; }}
.mdow-breadcrumb button.mdow-icon-button {{ min-width: {button_xs}px; min-height: {button_xs}px; color: {chevron_btn}; }}
.mdow-breadcrumb button.mdow-icon-button image {{ -gtk-icon-size: 10px; }}
.mdow-pane-header {{ min-height: {breadcrumb}px; padding: 0 12px; border-bottom: 1px solid {border_subtle}; font-size: {breadcrumb_text}px; }}
.mdow-pane-header .mdow-pane-mark {{ min-width: 2px; min-height: 14px; border-radius: 99px; background: transparent; }}
.mdow-pane-active .mdow-pane-header .mdow-pane-mark {{ background: {primary}; }}
.mdow-pane-badge {{ padding: 2px 6px; border-radius: 4px; background: {muted}; font-size: 10px; font-weight: 500; color: {muted_fg}; }}
.mdow-split-line {{ background: {border_subtle}; min-width: 1px; }}

/* ── Reader ── */
.mdow-reader, .mdow-reader listview {{ background: {bg}; color: {fg}; }}
.mdow-reader:focus-visible {{ outline: none; }}
.mdow-reader listview > row {{ background: none; padding: 0; border: none; outline: none; box-shadow: none; }}
.mdow-reader listview > row:hover, .mdow-reader listview > row:selected {{ background: none; }}
.md {{ font-family: {content_font}; font-size: {fs}px; color: {fg}; }}
.md label {{ line-height: {lh:.2}px; }}
.md label link {{ color: {primary}; }}
.md label link:hover {{ color: {primary}; }}
.md label selection {{ background-color: {selection}; color: {fg}; }}
"#,
        scroll = p.scrollbar.css(),
        scroll_hover = p.scrollbar_hover.css(),
        tabbar_inner = t.tabbar - 17.0,
        sidebar_title = t.sidebar_title,
        sidebar_hover = p.sidebar_accent.alpha(0.7).css(),
        sidebar_text = t.sidebar_text,
        row_pad_x = t.row_pad_x,
        row_pad_y = t.row_pad_y,
        ring = p.ring.alpha(0.5).css(),
        sidebar_fg_dim = p.foreground.alpha(0.75).css(),
        control_font = t.control_font,
        muted_soft = p.muted.alpha(0.4).css(),
        empty_icon = p.muted_fg.alpha(0.4).css(),
        tabbar = t.tabbar,
        tab = t.tab,
        muted_strong = p.muted.alpha(0.8).over(p.background).css(),
        secondary = p.secondary.css(),
        button_sm = t.button - 4.0,
        breadcrumb = t.breadcrumb,
        breadcrumb_text = t.breadcrumb_text,
        breadcrumb_secondary = t.breadcrumb_secondary,
        fg_85 = p.foreground.alpha(0.85).css(),
        chevron = p.muted_fg.alpha(0.4).css(),
        chevron_btn = p.muted_fg.alpha(0.7).css(),
        button_xs = t.button_xs,
        content_font = content_font,
        fs = fs,
        lh = fs * READER_LINE_HEIGHT,
        selection = p.primary.alpha(0.25).css(),
    );

    for (i, scale) in HEADING_SCALE.iter().enumerate() {
        let size = fs * scale;
        let color = if i == 5 { muted_fg.clone() } else { fg.clone() };
        let _ = writeln!(
            css,
            ".md-h{n} {{ font-size: {size:.2}px; font-weight: {w}; letter-spacing: {ls:.3}px; color: {color}; }}\n.md label.md-h{n} {{ line-height: {lh:.2}px; }}",
            n = i + 1,
            w = HEADING_WEIGHT[i],
            ls = HEADING_TRACKING[i] * size,
            lh = HEADING_LINE[i] * size,
        );
    }

    let code_size = fs * 0.875;
    let alert_names = ["note", "tip", "important", "warning", "caution"];
    let _ = write!(
        css,
        r#"
.md-quote {{ border-left: 3px solid {quote_bar}; padding: {qy:.1}px {qx:.1}px; }}
.md-quote label {{ color: {muted_fg}; }}
.md-alert {{ border: 1px solid {border}; border-left-width: 4px; border-radius: 8px; background: {muted}; padding: {ay:.1}px {ax:.1}px; }}
.md-marker {{ color: {muted_fg}; }}
.md-task check {{ min-width: 13px; min-height: 13px; margin: 0; }}
.md-task check:checked {{ background: {primary}; color: {primary_fg}; }}
.md-hr {{ background: {border}; min-height: 1px; }}
.md-code {{ background: {muted}; border: 1px solid {border}; border-radius: 10px; box-shadow: {code_shadow}; }}
.md-code.md-part-first {{ border-bottom-left-radius: 0; border-bottom-right-radius: 0; border-bottom: none; }}
.md-code.md-part-middle {{ border-radius: 0; border-top: none; border-bottom: none; box-shadow: none; }}
.md-code.md-part-last {{ border-top-left-radius: 0; border-top-right-radius: 0; border-top: none; box-shadow: {code_shadow_last}; }}
.md-code scrolledwindow, .md-code viewport {{ background: none; }}
.md-code-text {{ font-family: {code_font}; font-size: {code_size:.2}px; color: {code_fg}; }}
.md label.md-code-text {{ line-height: {code_lh:.2}px; }}
.md-code-lang {{ font-family: {code_font}; font-size: 11px; font-weight: 500; letter-spacing: 0.2px; color: {muted_fg}; background: {muted}; border-radius: 4px; padding: 1px 6px; }}
.md-copy {{ min-width: 28px; min-height: 28px; padding: 0; border-radius: 6px; color: {muted_fg}; background: none; border: 1px solid transparent; opacity: 0; transition: opacity 120ms ease-out, background-color 120ms ease-out; }}
.md-code-wrap.mdow-hover .md-copy, .md-copy.md-copied, .md-copy:focus-visible {{ opacity: 1; }}
.md-copy:hover {{ background: {copy_hover}; color: {fg}; border-color: {border_subtle}; }}
.md-copy.md-copied {{ color: {fg}; }}
.md-copy image {{ -gtk-icon-size: 14px; }}
.md-table {{ border: 1px solid {border}; border-radius: 8px; }}
.md-table scrolledwindow, .md-table viewport {{ background: none; }}
.md-th {{ background: {muted}; border-bottom: 1px solid {border}; }}
.md-th label {{ font-size: {th_size:.2}px; font-weight: 600; letter-spacing: {th_ls:.2}px; color: {muted_fg}; line-height: {th_lh:.2}px; }}
.md-td label {{ font-size: {td_size:.2}px; line-height: {td_lh:.2}px; }}
.md-cell {{ padding: 10px 14px; }}
.md-cell.md-cell-right {{ border-right: 1px solid {border_subtle}; }}
.md-cell.md-cell-below {{ border-bottom: 1px solid {border_subtle}; }}
.md-th.md-cell-right {{ border-right: 1px solid {border_subtle}; }}
.md-image {{ border-radius: 8px; }}
.md-images, .md-images > flowboxchild {{ padding: 0; background: none; }}
.md-image-missing {{ color: {muted_fg}; font-size: {small:.1}px; padding: 8px 12px; border-radius: 8px; border: 1px dashed {border}; }}
.md-mermaid-error {{ color: {destructive}; font-family: {code_font}; font-size: {code_size:.2}px; padding: 16px; background: {muted}; border-radius: 8px; }}
.md-math {{ font-family: {code_font}; font-size: {code_size:.2}px; color: {fg}; }}
.md-footnotes label {{ font-size: {small:.1}px; color: {muted_fg}; line-height: {small_lh:.2}px; }}
.md-footnotes-rule {{ background: {border}; min-height: 1px; }}
.md-render-error {{ border-radius: 8px; border: 1px solid {destructive_soft}; background: {destructive_wash}; padding: 16px; }}
.md-render-error label {{ color: {destructive}; }}
.md-partial {{ color: {muted_fg}; font-size: 12px; }}
"#,
        quote_bar = p.muted_fg.alpha(0.45).css(),
        qy = fs * 0.4,
        qx = fs,
        ay = fs * 0.85,
        ax = fs,
        primary_fg = p.primary_fg.css(),
        code_shadow = if p.dark {
            "inset 0 1px 0 rgba(255,255,255,0.04)"
        } else {
            "0 1px 0 rgba(0,0,0,0.02)"
        },
        code_shadow_last = if p.dark { "none" } else { "0 1px 0 rgba(0,0,0,0.02)" },
        code_font = code_font,
        code_size = code_size,
        code_lh = code_size * 1.6,
        th_lh = fs * 0.925 * 0.8 * READER_LINE_HEIGHT,
        td_lh = fs * 0.925 * READER_LINE_HEIGHT,
        small_lh = fs * 0.875 * READER_LINE_HEIGHT,
        code_fg = p.foreground.css(),
        copy_hover = p.foreground.alpha(0.08).css(),
        th_size = fs * 0.925 * 0.8,
        th_ls = fs * 0.925 * 0.8 * 0.04,
        td_size = fs * 0.925,
        small = fs * 0.875,
        destructive_soft = p.destructive.alpha(0.2).css(),
        destructive_wash = p.destructive.alpha(0.05).css(),
    );
    for (name, color) in alert_names.iter().zip(p.alerts.iter()) {
        let _ = writeln!(css, ".md-alert-{name} {{ border-left-color: {}; }}", color.css());
    }

    let _ = write!(
        css,
        r#"
/* ── Floating chrome over the reader ── */
.mdow-find {{ background: {find_bg}; border-bottom: 1px solid {find_border}; border-left: 1px solid {find_border}; border-bottom-left-radius: 12px; padding: 8px 12px; box-shadow: 0 4px 12px {shadow}; }}
.mdow-find entry {{ min-height: 28px; min-width: 176px; font-size: {control_font}px; }}
.mdow-find .mdow-find-count {{ min-width: 64px; font-size: {control_xs}px; color: {muted_fg}; font-feature-settings: "tnum"; }}
.mdow-find .mdow-find-hint {{ font-size: {control_xs}px; color: {subtle}; }}
.mdow-find button {{ min-width: {button_xs}px; min-height: {button_xs}px; padding: 0; border-radius: 4px; }}
.mdow-find button image {{ -gtk-icon-size: 12px; }}
.mdow-kbd {{ font-family: "Geist Mono", monospace; font-size: 10px; padding: 0 4px; min-width: 12px; border-radius: 4px; background: {muted}; color: {muted_fg}; border: 1px solid {border_subtle}; }}
.mdow-zoom {{ background: {popover}; border: 1px solid {border}; border-radius: 8px; padding: 4px 6px; box-shadow: 0 1px 3px {shadow}; font-size: {control_font}px; transition: opacity 150ms cubic-bezier(0.23, 1, 0.32, 1); }}
.mdow-zoom button {{ min-width: {button_xs}px; min-height: {button_xs}px; padding: 0; border-radius: 4px; }}
.mdow-zoom button image {{ -gtk-icon-size: 12px; }}
.mdow-zoom label {{ min-width: 40px; font-feature-settings: "tnum"; }}
.mdow-zoom.mdow-hidden {{ opacity: 0; }}

/* ── Welcome ── */
.mdow-welcome {{ color: {muted_fg}; }}
.mdow-welcome.mdow-drag-over {{ background: {drag_wash}; }}
.mdow-welcome-title {{ font-size: 24px; font-weight: 600; letter-spacing: -0.4px; color: {fg}; }}
.mdow-welcome-text {{ font-size: 14px; color: {muted_fg}; }}
.mdow-welcome button.mdow-outline {{ min-height: 28px; padding: 0 10px; border-radius: 6px; border: 1px solid {border}; background: {bg}; font-size: {control_font}px; font-weight: 500; box-shadow: 0 1px 2px rgba(0,0,0,0.04); }}
.mdow-welcome button.mdow-outline:hover {{ background: {muted}; }}
.mdow-dropcard {{ border: 1px dashed {dropcard_border}; border-radius: 10px; background: {dropcard_bg}; padding: 12px 16px; font-size: 12px; color: {muted_fg}; }}
.mdow-drag-over .mdow-dropcard {{ border: 1px solid {drag_ring}; background: {drag_card}; }}
.mdow-dropcard .mdow-dropcard-code {{ font-family: "Geist Mono", monospace; font-size: 11px; background: {muted}; color: {fg_80}; border-radius: 3px; padding: 0 4px; }}
.mdow-welcome-recent-label {{ font-family: "Geist Mono", monospace; font-size: 11px; letter-spacing: 2px; color: {subtle}; }}
.mdow-welcome button.mdow-recent {{ min-height: 32px; padding: 0 8px; border-radius: 6px; font-weight: 400; font-size: 13px; color: {fg}; }}
.mdow-welcome button.mdow-recent:hover {{ background: {muted}; }}
.mdow-welcome button.mdow-recent image {{ opacity: 0.6; }}
.mdow-logo {{ border-radius: 22%; box-shadow: 0 1px 2px rgba(0,0,0,0.06), 0 0 0 1px {logo_ring}; }}

/* ── Status pages (errors, empty panes) ── */
.mdow statuspage > scrolledwindow > viewport > box {{ margin: 24px; }}
.mdow statuspage .title {{ font-size: 16px; font-weight: 500; }}
.mdow statuspage .icon {{ color: {muted_fg}; -gtk-icon-size: 24px; }}
.mdow statuspage .description {{ color: {muted_fg}; font-size: 14px; }}
.mdow-error-path {{ font-family: "Geist Mono", monospace; font-size: 12px; color: {subtle}; }}
.mdow-error-icon {{ background: {muted}; border-radius: 999px; min-width: 48px; min-height: 48px; color: {muted_fg}; }}
.mdow-error-icon image {{ -gtk-icon-size: 24px; }}
.mdow-error-title {{ font-size: 16px; font-weight: 500; color: {fg}; }}
.mdow-error-body {{ font-size: 14px; color: {muted_fg}; }}
button.mdow-small {{ min-height: 28px; padding: 0 10px; border-radius: 6px; font-size: {control_font}px; font-weight: 500; }}
button.mdow-outline {{ border: 1px solid {border}; background: {bg}; }}
button.mdow-outline:hover {{ background: {muted}; }}

/* ── Command palette ── */
.mdow-palette {{ background: {popover}; }}
.mdow-palette .mdow-palette-entry {{ min-height: 44px; padding: 0 14px; font-size: 14px; border-bottom: 1px solid {border_subtle}; background: none; box-shadow: none; outline: none; border-radius: 0; }}
.mdow-palette .mdow-palette-entry > image {{ color: {muted_fg}; }}
.mdow-palette list {{ background: none; padding: 4px; }}
.mdow-palette list > row {{ border-radius: 6px; padding: 6px 8px; margin: 0; min-height: 0; font-size: 13px; }}
.mdow-palette list > row:selected {{ background: {muted}; color: {fg}; }}
.mdow-palette list > row image {{ color: {muted_fg}; -gtk-icon-size: 16px; }}
.mdow-palette .mdow-palette-hint {{ font-size: 11px; color: {subtle}; }}
.mdow-palette .mdow-palette-heading {{ font-size: 11px; font-weight: 500; color: {muted_fg}; padding: 8px 10px 4px 10px; }}
.mdow-palette .mdow-palette-empty {{ padding: 24px; font-size: 13px; color: {muted_fg}; }}
.mdow-palette .mdow-palette-footer {{ border-top: 1px solid {border_subtle}; background: {muted_soft}; padding: 6px 12px; font-size: 10px; color: {subtle}; }}

/* ── Settings ── */
.mdow-settings .mdow-field-label {{ font-size: 12px; font-weight: 500; color: {muted_fg}; }}
.mdow-settings .mdow-section-title {{ font-size: 14px; font-weight: 500; color: {fg}; }}
.mdow-settings .mdow-section-text {{ font-size: 12px; color: {muted_fg}; }}
.mdow-preview {{ border: 1px solid {border_subtle}; border-radius: 8px; background: {muted_soft}; padding: 14px 16px; }}
.mdow-preview-title {{ font-weight: 600; font-size: {preview_title:.2}px; color: {fg}; }}
.mdow-preview-text {{ font-size: {preview_text:.2}px; color: {fg_85}; }}
.mdow-segmented {{ background: {muted}; border-radius: 6px; padding: 2px; }}
.mdow-segmented > button {{ min-height: 24px; border-radius: 5px; font-size: {control_font}px; font-weight: 500; color: {muted_fg}; background: none; box-shadow: none; }}
.mdow-segmented > button:hover {{ color: {fg}; }}
.mdow-segmented > button:checked {{ background: {segment_on}; color: {fg}; box-shadow: 0 1px 2px rgba(0,0,0,0.08), 0 0 0 1px {segment_ring}; }}
.mdow-segmented separator {{ opacity: 0; min-width: 0; }}
button.mdow-tile {{ border-radius: 6px; border: 1px solid {border_subtle}; background: {bg}; padding: 6px; min-height: 0; box-shadow: none; }}
button.mdow-tile > overlay > box {{ margin: 4px 2px; }}
button.mdow-tile:hover {{ border-color: {border}; background: {muted_hover}; }}
button.mdow-tile:checked {{ border-color: {tile_on_border}; background: {tile_on_bg}; box-shadow: 0 0 0 1px {tile_on_ring}; }}
button.mdow-tile .mdow-tile-glyph {{ font-size: 18px; color: {fg_70}; }}
button.mdow-tile:checked .mdow-tile-glyph {{ color: {fg}; }}
button.mdow-tile .mdow-tile-glyph.mdow-mono-glyph {{ font-size: 13px; }}
button.mdow-tile .mdow-tile-label {{ font-size: 10px; font-weight: 500; letter-spacing: 0.3px; color: {muted_fg}; }}
button.mdow-tile:checked .mdow-tile-label {{ color: {fg}; }}
button.mdow-tile .mdow-tile-dot {{ min-width: 6px; min-height: 6px; border-radius: 99px; background: {primary}; }}
.mdow-settings separator.mdow-rule {{ background: {border_subtle}; min-height: 1px; }}

/* ── Shortcuts ── */
.mdow-shortcuts .mdow-shortcut-heading {{ font-size: 10px; font-weight: 500; letter-spacing: 1px; color: {subtle}; padding: 4px 16px 4px 16px; }}
.mdow-shortcuts .mdow-shortcut-row {{ padding: 6px 16px; }}
.mdow-shortcuts .mdow-shortcut-label {{ font-size: 14px; color: {muted_fg}; }}
.mdow-shortcuts separator {{ background: {border_subtle}; min-height: 1px; margin: 6px 0; }}
shortcut > .keycap, .mdow-keycap {{ font-family: "Geist Mono", monospace; font-size: 11px; min-width: 14px; min-height: 18px; padding: 0 5px; border-radius: 4px; background: {muted}; color: {muted_fg}; border: 1px solid {border_subtle}; box-shadow: none; }}
.mdow-cheatsheet {{ background: {popover}; border: 1px solid {border}; border-radius: 14px; padding: 20px 24px; box-shadow: 0 12px 40px {shadow}; }}
.mdow-cheatsheet .mdow-shortcut-heading {{ font-size: 10px; font-weight: 500; letter-spacing: 1px; color: {subtle}; padding: 0 0 6px 0; }}
.mdow-cheatsheet .mdow-shortcut-label {{ font-size: 13px; color: {muted_fg}; }}
"#,
        find_bg = p.background.alpha(0.94).css(),
        find_border = p.border.alpha(0.5).css(),
        control_xs = t.control_xs_font,
        control_font = t.control_font,
        button_xs = t.button_xs,
        drag_wash = p.primary.alpha(0.03).css(),
        dropcard_border = p.border.alpha(0.7).css(),
        dropcard_bg = p.muted.alpha(0.2).css(),
        drag_ring = p.primary.alpha(0.3).css(),
        drag_card = p.primary.alpha(0.05).css(),
        fg_80 = p.foreground.alpha(0.8).css(),
        logo_ring = p.border.alpha(0.4).css(),
        muted_soft = p.muted.alpha(0.4).css(),
        fg_85 = p.foreground.alpha(0.85).css(),
        preview_title = READER_FONT_SIZE * 1.25,
        preview_text = READER_FONT_SIZE,
        segment_on = if p.dark { p.input.css() } else { p.background.css() },
        segment_ring = p.foreground.alpha(if p.dark { 0.15 } else { 0.1 }).css(),
        muted_hover = p.muted.alpha(0.6).css(),
        tile_on_border = p.foreground.alpha(if p.dark { 0.3 } else { 0.25 }).css(),
        tile_on_bg = p.foreground.alpha(if p.dark { 0.06 } else { 0.04 }).css(),
        tile_on_ring = p.foreground.alpha(if p.dark { 0.2 } else { 0.1 }).css(),
        fg_70 = p.foreground.alpha(0.7).css(),
    );
    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklch_matches_known_srgb() {
        // oklch(1 0 0) is white and oklch(0 0 0) black.
        assert_eq!(oklch(1.0, 0.0, 0.0).hex(), "#ffffff");
        assert_eq!(oklch(0.0, 0.0, 0.0).hex(), "#000000");
        // The desktop's light primary renders as #2f6dd3-ish blue.
        let primary = oklch(0.55, 0.17, 260.0);
        assert!(primary.b > primary.r && primary.b > primary.g);
    }

    #[test]
    fn stylesheet_has_every_heading() {
        let css = stylesheet(&Palette::light(), &Prefs::default());
        for n in 1..=6 {
            assert!(css.contains(&format!(".md-h{n} ")));
        }
    }
}
