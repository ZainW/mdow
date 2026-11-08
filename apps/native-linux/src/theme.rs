//! Mdow's design tokens, converted from OKLCH, and the GTK stylesheet built from them.

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

impl Rgba {
    /// WCAG relative luminance.
    fn luminance(self) -> f64 {
        let linear = |c: f64| {
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }
}

/// WCAG contrast ratio between two opaque colours.
pub fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (a, b) = (a.luminance(), b.luminance());
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The nearest colour to `color` that reaches `target` against every surface it sits on. Colours
/// that already pass are untouched, so a theme only changes where it would be hard to read.
pub fn legible(color: Rgba, surfaces: &[Rgba], target: f64) -> Rgba {
    let passes = |c: Rgba| surfaces.iter().all(|s| contrast(c, *s) >= target);
    if passes(color) {
        return color;
    }
    let darkest = surfaces.iter().map(|s| s.luminance()).fold(1.0, f64::min);
    let lightest = surfaces.iter().map(|s| s.luminance()).fold(0.0, f64::max);
    let goal = if lightest + darkest > 0.36 { hex("#000000") } else { hex("#ffffff") };
    (1..=20).map(|step| color.mix(goal, step as f64 / 20.0)).find(|c| passes(*c)).unwrap_or(goal)
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

/// What the stylesheet takes from Omarchy besides colours.
#[derive(Clone, Debug)]
pub struct Omarchy {
    pub radius: u32,
    pub base_size: f64,
    pub tag: String,
    pub menu_border: Rgba,
    pub popup_border: Rgba,
    pub normal_border: Rgba,
    pub hover_border: Rgba,
    pub pressed: Rgba,
    pub scrim: Rgba,
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
    /// The window around the document: header, sidebar and gutter.
    pub chrome: Rgba,
    /// Row and button fills.
    pub hover: Rgba,
    pub selected: Rgba,
    pub text_selection: Rgba,
    /// Additions in diffs and the "Applied" state.
    pub added: Rgba,
    /// Removals in diffs, errors and the "Failed" state.
    pub removed: Rgba,
    /// Set when the colours come from the current Omarchy theme.
    pub omarchy: Option<Omarchy>,
}

impl Palette {
    /// Warm paper with one clay accent, the same colours as Mdow for Mac.
    pub fn light() -> Palette {
        let paper = hex("#fdfcf9");
        let ink = |alpha: f64| hex("#3c2d1e").alpha(alpha).over(paper);
        let chrome = hex("#f3f1eb");
        let well = hex("#f5f3ed");
        let accent = legible(hex("#b95a33"), &[paper, chrome], 4.5);
        Palette {
            dark: false,
            background: paper,
            foreground: hex("#1e1c19"),
            card: paper,
            popover: hex("#ffffff"),
            primary: accent,
            primary_fg: hex("#ffffff"),
            secondary: hex("#f5f3ed"),
            muted: hex("#f5f3ed"),
            muted_fg: hex("#6b665e"),
            muted_fg_subtle: legible(hex("#9e998f"), &[paper, chrome], 4.5),
            accent,
            destructive: hex("#b3412e"),
            border: ink(0.16),
            border_subtle: ink(0.1),
            input: ink(0.16),
            ring: accent,
            sidebar: hex("#f3f1eb"),
            sidebar_accent: hex("#3c2d1e").alpha(0.085).over(hex("#f3f1eb")),
            alerts: [
                hex("#2c6c8c"),
                hex("#3c7a36"),
                hex("#6249b0"),
                hex("#86670f"),
                hex("#b3412e"),
            ],
            chart: [
                accent,
                hex("#3c7a36"),
                hex("#2c6c8c"),
                hex("#6249b0"),
                hex("#9e3f6c"),
                hex("#0f8080"),
            ],
            find_match: oklch(0.93, 0.08, 85.0),
            find_match_fg: None,
            find_active: hex("#d97757"),
            find_active_fg: hex("#ffffff"),
            scrollbar: hex("#3c2d1e").alpha(0.2),
            scrollbar_hover: hex("#3c2d1e").alpha(0.4),
            syntax: Syntax {
                fg: hex("#1e1c19"),
                comment: legible(hex("#99948a"), &[well], 4.5),
                keyword: hex("#9e3f6c"),
                string: hex("#3c7a36"),
                entity: hex("#6249b0"),
                constant: hex("#ad531c"),
            },
            chrome: hex("#f3f1eb"),
            hover: hex("#3c2d1e").alpha(0.05),
            selected: hex("#3c2d1e").alpha(0.085),
            text_selection: hex("#d97757").alpha(0.24),
            added: hex("#2f7d4f"),
            removed: hex("#b4413a"),
            omarchy: None,
        }
    }

    /// Neutral ink with the clay accent, the same colours as Mdow for Mac.
    pub fn dark() -> Palette {
        let paper = hex("#1b1b1b");
        let glow = |alpha: f64| hex("#ffffff").alpha(alpha).over(paper);
        let chrome = hex("#131313");
        let well = hex("#161616");
        let accent = hex("#e08a66");
        Palette {
            dark: true,
            background: paper,
            foreground: hex("#ececec"),
            card: paper,
            popover: hex("#242424"),
            primary: accent,
            primary_fg: hex("#131313"),
            secondary: hex("#161616"),
            muted: hex("#161616"),
            muted_fg: hex("#a3a3a3"),
            muted_fg_subtle: legible(hex("#6e6e6e"), &[paper, chrome], 4.5),
            accent,
            destructive: hex("#f08a7a"),
            border: glow(0.14),
            border_subtle: glow(0.08),
            input: glow(0.14),
            ring: accent,
            sidebar: hex("#131313"),
            sidebar_accent: hex("#ffffff").alpha(0.085).over(hex("#131313")),
            alerts: [
                hex("#7cc4e4"),
                hex("#a6d28f"),
                hex("#b9a5f5"),
                hex("#e3c178"),
                hex("#f08a7a"),
            ],
            chart: [
                accent,
                hex("#a6d28f"),
                hex("#7cc4e4"),
                hex("#b9a5f5"),
                hex("#e58ab4"),
                hex("#5a9e98"),
            ],
            find_match: oklch(0.45, 0.08, 85.0),
            find_match_fg: Some(hex("#ececec")),
            find_active: accent,
            find_active_fg: hex("#131313"),
            scrollbar: hex("#ffffff").alpha(0.2),
            scrollbar_hover: hex("#ffffff").alpha(0.4),
            syntax: Syntax {
                fg: hex("#ececec"),
                comment: legible(hex("#727272"), &[well], 4.5),
                keyword: hex("#e58ab4"),
                string: hex("#a6d28f"),
                entity: hex("#b9a5f5"),
                constant: hex("#f0a46c"),
            },
            chrome: hex("#131313"),
            hover: hex("#ffffff").alpha(0.05),
            selected: hex("#ffffff").alpha(0.085),
            text_selection: accent.alpha(0.3),
            added: hex("#74c48f"),
            removed: hex("#ec7a70"),
            omarchy: None,
        }
    }

    pub fn for_dark(dark: bool) -> Palette {
        if dark { Palette::dark() } else { Palette::light() }
    }

    /// The current Omarchy theme, mapped as Omarchy's own templates map it: `background` for the
    /// page, `dark_background` for the surfaces around it, the shell's state tokens for hover and
    /// selection, and the VS Code template's choices for syntax.
    pub fn from_omarchy(t: &crate::omarchy::Theme) -> Palette {
        let bg = t.color("background");
        let fg = t.color("foreground");
        let chrome = t.color("dark_background");
        let muted = t.color("muted");
        let yellow = t.color("yellow");
        let fg_at = |alpha: f64| fg.alpha(alpha);
        let selected = fg_at(t.selected_fill);
        // Several themes make dark_background the same as background, which would lose code
        // blocks and table headers against the page.
        let well = if contrast(chrome, bg) < 1.04 { bg.mix(fg, 0.05) } else { chrome };
        let on_page = [bg, chrome, selected.over(chrome)];
        let text = |color: Rgba| legible(color, &on_page, 4.5);
        let code = |color: Rgba| legible(color, &[well, bg], 4.5);
        let accent = text(t.color("accent"));
        let on_accent = legible(bg, &[accent], 4.5);
        let bar = |color: Rgba| legible(color, &[well], 3.0);
        let find_match = yellow.mix(bg, 0.6);
        Palette {
            dark: t.dark,
            background: bg,
            foreground: fg,
            card: bg,
            popover: bg,
            primary: accent,
            primary_fg: on_accent,
            secondary: well,
            muted: well,
            muted_fg: text(fg.mix(bg, 0.3)),
            muted_fg_subtle: text(fg.mix(bg, 0.45)),
            accent,
            destructive: text(t.color("red")),
            border: muted,
            border_subtle: muted.mix(bg, 0.45),
            input: muted,
            ring: accent,
            sidebar: chrome,
            sidebar_accent: selected.over(chrome),
            alerts: [
                bar(t.color("blue")),
                bar(t.color("green")),
                bar(t.color("magenta")),
                bar(yellow),
                bar(t.color("red")),
            ],
            chart: [
                accent,
                t.color("green"),
                t.color("blue"),
                t.color("magenta"),
                t.color("orange"),
                t.color("cyan"),
            ],
            find_match,
            find_match_fg: Some(legible(t.color("bright_foreground"), &[find_match], 4.5)),
            find_active: accent,
            find_active_fg: on_accent,
            scrollbar: fg_at(0.25),
            scrollbar_hover: fg_at(0.45),
            syntax: Syntax {
                fg,
                comment: code(t.color("dark_foreground")),
                keyword: code(t.color("bright_magenta")),
                string: code(t.color("green")),
                entity: code(t.color("blue")),
                constant: code(t.color("orange")),
            },
            chrome,
            hover: fg_at(t.hover_fill),
            selected,
            text_selection: t.color("selection").alpha(t.selection_fill.max(0.6)),
            added: text(t.color("green")),
            removed: text(t.color("red")),
            omarchy: Some(Omarchy {
                radius: t.rounding,
                base_size: t.base_size,
                tag: t.tag(),
                menu_border: t.menu_border,
                popup_border: t.popup_border,
                normal_border: fg_at(t.normal_border_alpha),
                hover_border: fg_at(t.hover_border_alpha),
                pressed: fg_at(t.pressed_fill),
                scrim: bg.alpha(t.scrim_alpha),
            }),
        }
    }

    /// Identifies these colours in caches of things rendered with them.
    pub fn cache_tag(&self) -> String {
        match &self.omarchy {
            Some(o) => o.tag.clone(),
            None => self.dark.to_string(),
        }
    }
}

/// Reader typography: 16px, scaled by zoom. The line height is a setting.
pub const READER_FONT_SIZE: f64 = 16.0;
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
    pane_header: f64,
    pane_header_text: f64,
}

fn scale_tokens(scale: InterfaceScale) -> ScaleTokens {
    match scale {
        InterfaceScale::Compact => ScaleTokens {
            control_font: 12.0,
            control_xs_font: 11.0,
            button: 28.0,
            button_xs: 24.0,
            sidebar_width: 244.0,
            sidebar_title: 11.0,
            sidebar_text: 12.0,
            row_pad_x: 6.0,
            row_pad_y: 4.0,
            tabbar: 40.0,
            tab: 28.0,
            pane_header: 28.0,
            pane_header_text: 11.0,
        },
        InterfaceScale::Comfortable => ScaleTokens {
            control_font: 13.0,
            control_xs_font: 11.0,
            button: 32.0,
            button_xs: 26.0,
            sidebar_width: 264.0,
            sidebar_title: 12.0,
            sidebar_text: 13.0,
            row_pad_x: 8.0,
            row_pad_y: 5.0,
            tabbar: 44.0,
            tab: 32.0,
            pane_header: 32.0,
            pane_header_text: 12.0,
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
            tabbar: 48.0,
            tab: 36.0,
            pane_header: 36.0,
            pane_header_text: 13.0,
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
    let line_height = prefs.line_spacing.height();
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
    let sidebar_accent = p.sidebar_accent.css();
    let popover = p.popover.css();
    let destructive = p.destructive.css();
    let hover = p.muted.alpha(0.6).css();
    let shadow = if p.dark { "rgba(0,0,0,0.5)" } else { "rgba(0,0,0,0.08)" };

    let _ = write!(
        css,
        r#"
window.mdow {{
  font-family: "Inter Variable", "Inter", sans-serif;
  font-size: 13px;
  font-feature-settings: "cv02", "cv03", "cv04", "cv11";
}}
window.mdow {{ background: {chrome}; color: {fg}; }}
.mdow-dim {{ color: {muted_fg}; }}
.mdow-subtle {{ color: {subtle}; }}

/* Scrollbars: thin, rounded, quiet. */
.mdow scrollbar slider {{ background-color: {scroll}; min-width: 6px; min-height: 6px; border-radius: 999px; }}
.mdow scrollbar slider:hover {{ background-color: {scroll_hover}; }}
.mdow-sidebar scrollbar slider {{ min-width: 4px; min-height: 4px; }}

/* ── Sidebar ── */
.mdow-sidebar {{ background: {chrome}; color: {fg}; }}
.mdow-sidebar-header {{ padding: 0 8px 8px 8px; }}
.mdow-sidebar-footer {{ padding: 6px 8px 8px 8px; }}
.mdow-mode-tabs {{ background: none; padding: 0; border-spacing: 2px; }}
.mdow-mode-tabs > button {{ font-weight: 500; min-height: 28px; padding: 0 4px; border-radius: 6px; font-size: {sidebar_title}px; color: {muted_fg}; background: none; box-shadow: none; }}
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
.mdow-row-icon {{ opacity: 0.4; }}
.mdow-row-dir {{ font-size: 11px; color: {subtle}; }}
.mdow-outline row .mdow-row {{ color: {sidebar_fg_dim}; }}
.mdow-outline row.mdow-active .mdow-row {{ color: {fg}; }}
.mdow-tree row .mdow-row {{ padding-top: 3px; padding-bottom: 3px; font-size: 13px; }}
.mdow-tree row .mdow-row-icon {{ opacity: 0.55; }}
.mdow-tree treeexpander indent {{ -gtk-icon-size: 12px; }}
.mdow-sidebar-footer button {{ min-height: 24px; padding: 2px 8px; font-size: {control_font}px; color: {muted_fg}; }}
.mdow-sidebar-footer button:hover {{ color: {fg}; }}
.mdow-filter {{ margin: 0 8px 8px 8px; min-height: 28px; font-size: {control_font}px; }}
.mdow-note {{ margin: 0 8px 4px 8px; padding: 6px 8px; border-radius: 6px; border: 1px solid {border_subtle}; background: {muted_soft}; font-size: 11px; color: {muted_fg}; }}
.mdow-match-count {{ margin: -4px 12px 6px 12px; font-size: 11px; color: {subtle}; }}
.mdow-empty {{ padding: 24px 12px; }}
.mdow-empty image {{ color: {empty_icon}; -gtk-icon-size: 20px; }}
.mdow-empty .mdow-empty-title {{ font-size: 12px; color: {sidebar_fg_dim}; }}
.mdow-empty .mdow-empty-hint {{ font-size: 11px; color: {subtle}; }}

/* ── Tab bar ── */
.mdow-tabbar {{ background: {chrome}; min-height: {tabbar}px; padding: 0 8px; }}
.mdow-card {{ background: {bg}; border: 1px solid {border_subtle}; border-radius: 12px; margin: 0 8px 8px 8px; }}
.mdow-card.mdow-card-flush {{ margin-left: 0; }}
.mdow-card.mdow-card-flush-end {{ margin-right: 0; }}
.mdow-companion-paned > separator {{ background: none; min-width: 1px; }}
button.mdow-search {{ min-height: {button_sm}px; min-width: 170px; padding: 0 8px; border-radius: 6px; border: 1px solid {border_subtle}; background: {bg}; color: {muted_fg}; font-size: {control_font}px; font-weight: 400; }}
button.mdow-search:hover {{ background: {hover_fill}; color: {fg}; }}
.mdow-tabbar tabbar, .mdow-tabbar tabbar > revealer > box {{ background: none; box-shadow: none; border: none; padding: 0; min-height: {tabbar}px; }}
.mdow-tabbar tabbar tabbox {{ padding: 0 2px; }}
.mdow-tabbar tabbar tab {{
  min-height: {tab}px; margin: 4px 1px; padding: 0 2px 0 6px; border-radius: 6px;
  font-size: {control_font}px; color: {muted_fg}; background: none; box-shadow: none;
  transition: background-color 140ms cubic-bezier(0.23, 1, 0.32, 1), color 140ms cubic-bezier(0.23, 1, 0.32, 1);
}}
.mdow-tabbar tabbar tab:hover {{ background-color: {hover}; color: {fg}; }}
.mdow-tabbar tabbar tab:selected {{
  background-color: {popover}; color: {fg};
  box-shadow: 0 0 0 1px {border_subtle};
}}
.mdow-tabbar tabbar tab .tab-close-button {{ min-width: 24px; min-height: 24px; padding: 0; border-radius: 4px; opacity: 0.5; }}
.mdow-tabbar tabbar tab .tab-close-button:hover {{ opacity: 1; background: {muted_strong}; }}
.mdow-tabbar tabbar tab image {{ -gtk-icon-size: 14px; color: {muted_fg}; }}
.mdow-tabbar tabbar tab:selected image {{ color: {muted_fg}; }}
.mdow-tabbar tabbar tabbox > separator {{ background: {border_subtle}; min-width: 1px; margin: 11px 0; opacity: 1; }}
.mdow-tabbar tabbar .needs-attention {{ color: {accent}; }}
.mdow-tabbar .mdow-tab-actions {{ padding: 0 0 0 6px; }}
.mdow-icon-button {{ min-width: {button_sm}px; min-height: {button_sm}px; padding: 0; border-radius: 6px; color: {muted_fg}; }}
.mdow-icon-button:hover {{ color: {fg}; }}
.mdow-icon-button:checked {{ background: {secondary}; color: {fg}; }}
.mdow-icon-button.mdow-quiet:checked {{ background: none; color: {muted_fg}; }}
.mdow-icon-button.mdow-quiet:checked:hover {{ background: {hover}; color: {fg}; }}
.mdow-icon-button image {{ -gtk-icon-size: 14px; }}

.mdow-pane-header {{ min-height: {pane_header}px; padding: 0 12px; border-bottom: 1px solid {border_subtle}; font-size: {pane_header_text}px; }}
.mdow-pane-header .mdow-pane-mark {{ min-width: 2px; min-height: 14px; border-radius: 99px; background: transparent; }}
.mdow-pane-active .mdow-pane-header .mdow-pane-mark {{ background: {primary}; }}
.mdow-pane-badge {{ padding: 2px 6px; border-radius: 4px; background: {muted}; font-size: 11px; font-weight: 500; color: {muted_fg}; }}
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
        button_sm = t.button,
        pane_header = t.pane_header,
        pane_header_text = t.pane_header_text,
        content_font = content_font,
        fs = fs,
        lh = fs * line_height,
        selection = p.text_selection.css(),
        chrome = p.chrome.css(),
        hover_fill = p.hover.css(),
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
.md-alert {{ border-left: 4px solid transparent; border-radius: 8px; background: {muted}; padding: {ay:.1}px {ax:.1}px; }}
.md label.md-alert-title {{ font-size: {th_size:.2}px; font-weight: 600; }}
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
.md-image {{ border-radius: 8px; outline: 1px solid {image_edge}; outline-offset: -1px; }}
.md-images, .md-images > flowboxchild {{ padding: 0; background: none; }}
.md-image-missing {{ color: {muted_fg}; font-size: {small:.1}px; padding: 8px 12px; border-radius: 8px; border: 1px dashed {border}; }}
.md-mermaid-error {{ color: {destructive}; font-family: {code_font}; font-size: {code_size:.2}px; padding: 16px; background: {muted}; border-radius: 8px; }}
.md-math-text, .md-math-text text {{ background: none; color: {fg}; }}
.md-math {{ font-family: {code_font}; font-size: {code_size:.2}px; color: {fg}; }}
.md-footnotes label {{ font-size: {small:.1}px; color: {muted_fg}; line-height: {small_lh:.2}px; }}
.md-footnotes-rule {{ background: {border}; min-height: 1px; }}
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
        th_lh = fs * 0.925 * 0.8 * line_height,
        td_lh = fs * 0.925 * line_height,
        small_lh = fs * 0.875 * line_height,
        code_fg = p.foreground.css(),
        copy_hover = p.foreground.alpha(0.08).css(),
        th_size = fs * 0.925 * 0.8,
        image_edge = if p.dark { "rgba(255,255,255,0.1)" } else { "rgba(0,0,0,0.1)" },
        th_ls = fs * 0.925 * 0.8 * 0.04,
        td_size = fs * 0.925,
        small = fs * 0.875,
    );
    for (name, color) in alert_names.iter().zip(p.alerts.iter()) {
        let _ = writeln!(
            css,
            ".md-alert-{name} {{ border-left-color: {c}; }}\n.md-alert-{name} .md-alert-title {{ color: {c}; }}",
            c = color.css()
        );
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
.mdow-kbd {{ font-family: "Geist Mono", monospace; font-size: 11px; padding: 0 4px; min-width: 12px; border-radius: 4px; background: {muted}; color: {muted_fg}; border: 1px solid {border_subtle}; }}
.mdow-zoom {{ background: {popover}; border: 1px solid {border}; border-radius: 8px; padding: 4px 6px; box-shadow: 0 1px 3px {shadow}; font-size: {control_font}px; transition: opacity 150ms cubic-bezier(0.23, 1, 0.32, 1); }}
.mdow-zoom button {{ min-width: {button_xs}px; min-height: {button_xs}px; padding: 0; border-radius: 4px; }}
.mdow-zoom button image {{ -gtk-icon-size: 12px; }}
.mdow-zoom label {{ min-width: 40px; font-feature-settings: "tnum"; }}
.mdow-zoom.mdow-hidden {{ opacity: 0; }}

/* ── Welcome ── */
.mdow-welcome.mdow-drag-over {{ background: {drag_wash}; }}
.mdow-welcome-title {{ font-family: {content_font}; font-size: 34px; color: {fg}; }}
.mdow-welcome-text {{ font-size: 14px; color: {muted_fg}; }}
.mdow-welcome-hint {{ font-size: 12px; color: {subtle}; }}
.mdow-welcome-section {{ font-size: 12px; font-weight: 500; color: {subtle}; }}
button.mdow-action-card {{ min-height: 0; padding: 14px; border-radius: 12px; border: 1px solid {border_subtle}; background: {popover}; color: {fg}; font-weight: 400; }}
button.mdow-action-card:hover {{ background: {hover_fill}; }}
button.mdow-action-card .mdow-action-keys {{ color: {subtle}; font-size: 11px; }}
.mdow-recent-list {{ border: 1px solid {border_subtle}; border-radius: 12px; padding: 4px; }}
.mdow-recent-list button {{ min-height: 0; padding: 6px 8px; border-radius: 8px; font-weight: 400; color: {fg}; }}
.mdow-recent-list button:hover {{ background: {hover_fill}; }}
.mdow-recent-list .mdow-recent-dir {{ font-size: 11px; color: {subtle}; }}
.mdow-logo {{ border-radius: 22%; }}

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
.mdow-palette sheet {{ background: {popover}; }}
.mdow-palette .mdow-palette-entry {{ min-height: 44px; padding: 0 14px; font-size: 14px; border-bottom: 1px solid {border_subtle}; background: none; box-shadow: none; outline: none; border-radius: 0; }}
.mdow-palette .mdow-palette-entry > image {{ color: {muted_fg}; }}
.mdow-palette list {{ background: none; padding: 4px; }}
.mdow-palette list > row {{ border-radius: 6px; padding: 6px 8px; margin: 0; min-height: 0; font-size: 13px; }}
.mdow-palette list > row:selected {{ background: {muted}; color: {fg}; }}
.mdow-palette list > row image {{ color: {muted_fg}; -gtk-icon-size: 16px; }}
.mdow-palette .mdow-palette-hint {{ font-size: 11px; color: {subtle}; }}
.mdow-palette .mdow-palette-heading {{ font-size: 11px; font-weight: 500; color: {muted_fg}; padding: 8px 10px 4px 10px; }}
.mdow-palette .mdow-palette-empty {{ padding: 24px; font-size: 13px; color: {muted_fg}; }}
.mdow-palette .mdow-palette-footer {{ border-top: 1px solid {border_subtle}; background: {muted_soft}; padding: 6px 12px; font-size: 11px; color: {subtle}; }}

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
button.mdow-tile > box {{ margin: 4px 2px; }}
button.mdow-tile:hover {{ border-color: {border}; background: {muted_hover}; }}
button.mdow-tile:checked {{ border-color: {tile_on_border}; background: {tile_on_bg}; box-shadow: 0 0 0 1px {tile_on_ring}; }}
button.mdow-tile .mdow-tile-glyph {{ font-size: 18px; color: {fg_70}; }}
button.mdow-tile:checked .mdow-tile-glyph {{ color: {fg}; }}
button.mdow-tile .mdow-tile-glyph.mdow-mono-glyph {{ font-size: 13px; }}
button.mdow-tile .mdow-tile-label {{ font-size: 11px; font-weight: 500; letter-spacing: 0.3px; color: {muted_fg}; }}
button.mdow-tile:checked .mdow-tile-label {{ color: {fg}; }}
.mdow-settings separator.mdow-rule {{ background: {border_subtle}; min-height: 1px; }}

/* ── Shortcuts ── */
.mdow-shortcuts .mdow-shortcut-heading {{ font-size: 11px; font-weight: 500; letter-spacing: 1px; color: {subtle}; padding: 4px 16px 4px 16px; }}
.mdow-shortcuts .mdow-shortcut-row {{ padding: 6px 16px; }}
.mdow-shortcuts .mdow-shortcut-label {{ font-size: 14px; color: {muted_fg}; }}
.mdow-shortcuts separator {{ background: {border_subtle}; min-height: 1px; margin: 6px 0; }}
shortcut > .keycap, .mdow-keycap {{ font-family: "Geist Mono", monospace; font-size: 11px; min-width: 14px; min-height: 18px; padding: 0 5px; border-radius: 4px; background: {muted}; color: {muted_fg}; border: 1px solid {border_subtle}; box-shadow: none; }}
.mdow-cheatsheet {{ background: {popover}; border: 1px solid {border}; border-radius: 14px; padding: 20px 24px; box-shadow: 0 12px 40px {shadow}; }}
.mdow-cheatsheet .mdow-shortcut-heading {{ font-size: 11px; font-weight: 500; letter-spacing: 1px; color: {subtle}; padding: 0 0 6px 0; }}
.mdow-cheatsheet .mdow-shortcut-label {{ font-size: 13px; color: {muted_fg}; }}
"#,
        find_bg = p.background.alpha(0.94).css(),
        find_border = p.border.alpha(0.5).css(),
        control_xs = t.control_xs_font,
        control_font = t.control_font,
        button_xs = t.button_xs,
        drag_wash = p.primary.alpha(0.03).css(),
        hover_fill = p.hover.css(),
        content_font = content_font,
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
    css.push_str(&companion_css(p, prefs));
    match &p.omarchy {
        Some(omarchy) => {
            let mut css = clamp_radii(&css, omarchy.radius);
            css.push_str(&omarchy_layer(p, omarchy));
            css
        }
        None => css,
    }
}

/// The companion panel's styles; `omarchy_layer` squares its corners and sets its interface font
/// like the rest of the window's.
fn companion_css(p: &Palette, prefs: &Prefs) -> String {
    let fg = p.foreground.css();
    let bg = p.background.css();
    let chrome = p.chrome.css();
    let well = p.muted.css();
    let secondary = p.muted_fg.css();
    let tertiary = p.muted_fg_subtle.css();
    let line = p.border_subtle.css();
    let strong = p.border.css();
    let hover = p.hover.css();
    let selected = p.selected.css();
    let accent = p.accent.css();
    let on_accent = p.primary_fg.css();
    let added = p.added.css();
    let removed = p.removed.css();
    let code_font = prefs.code_font.family();
    let content_font = prefs.content_font.family();
    format!(
        r#"
/* ── Companion ── */
.mdow-companion {{ background: {chrome}; color: {fg}; }}
.mdow-companion-header {{ min-height: 40px; padding: 0 8px 0 14px; }}
.mdow-companion-title {{ font-weight: 600; }}
.mdow-companion-secondary {{ color: {secondary}; }}
.mdow-companion-tertiary {{ color: {tertiary}; }}
.mdow-companion-accent {{ color: {accent}; }}
.mdow-companion-added {{ color: {added}; }}
.mdow-companion-removed {{ color: {removed}; }}
.mdow-companion-strong {{ font-weight: 500; }}
.mdow-companion-small {{ font-size: 11px; }}
.mdow-companion-mono {{ font-family: {code_font}; font-size: 12px; }}
.mdow-companion-small-mono {{ font-family: {code_font}; font-size: 11px; }}
.mdow-companion-pad {{ padding: 8px 10px; }}
.mdow-companion-display {{ font-family: {content_font}; font-size: 21px; color: {fg}; }}
.mdow-companion-setup {{ padding: 16px 14px; }}
.mdow-companion-empty {{ padding: 20px 14px; }}
.mdow-companion-list {{ padding: 6px 12px 16px 14px; }}
.mdow-companion-card {{ background: {bg}; border: 1px solid {line}; border-radius: 8px; }}
.mdow-companion-command {{ padding: 2px 4px 2px 10px; }}
.mdow-companion-hairline {{ background: {line}; min-height: 1px; }}
.mdow-reply {{ font-family: {content_font}; font-size: 14.5px; color: {fg}; }}
.mdow-reply-title {{ font-family: {content_font}; font-size: 16px; font-weight: 600; margin-top: 4px; }}
.mdow-reply-heading {{ font-size: 13px; font-weight: 600; margin-top: 4px; }}
.mdow-companion-user {{ background: {bg}; border: 1px solid {line}; border-radius: 14px; padding: 8px 12px; }}
button.mdow-companion-suggestion {{ background: {bg}; border: 1px solid {line}; border-radius: 8px; padding: 8px 11px; font-weight: 400; }}
button.mdow-companion-suggestion:hover {{ background: {hover}; border-color: {strong}; }}
button.mdow-pill {{ min-height: 24px; padding: 0 10px; border-radius: 999px; font-size: 12px; font-weight: 500; }}
button.mdow-pill-quiet {{ background: none; color: {secondary}; }}
button.mdow-pill-quiet:hover {{ background: {hover}; color: {fg}; }}
button.mdow-pill-secondary {{ background: {hover}; color: {fg}; border: 1px solid {line}; }}
button.mdow-pill-secondary:hover {{ background: {selected}; }}
button.mdow-pill-primary {{ background: {accent}; color: {on_accent}; }}
button.mdow-pill-primary:hover {{ background: {accent}; opacity: 0.9; }}
button.mdow-companion-disclosure {{ min-height: 22px; padding: 0 4px; margin-left: -4px; font-weight: 400; border-radius: 6px; }}
.mdow-companion-reasoning {{ margin-left: 6px; padding-left: 10px; border-left: 2px solid {strong}; }}
.mdow-companion-detail {{ background: {well}; border: 1px solid {line}; border-radius: 8px; padding: 6px 9px; }}
.mdow-companion-change {{ background: {bg}; border: 1px solid {line}; border-radius: 10px; }}
.mdow-companion-change-pending {{ border-color: {accent_soft}; }}
.mdow-companion-settled {{ opacity: 0.7; }}
.mdow-companion-change-top {{ min-height: 34px; padding: 0 10px; }}
.mdow-companion-change-actions {{ padding: 7px 8px; }}
.mdow-companion-pill {{ font-size: 11px; font-weight: 500; padding: 2px 7px; border-radius: 999px; }}
.mdow-companion-pill-review {{ color: {accent}; background: {accent_wash}; }}
.mdow-companion-pill-applied {{ color: {added}; background: {added_wash}; }}
.mdow-companion-pill-declined {{ color: {secondary}; background: {hover}; }}
.mdow-companion-pill-failed {{ color: {removed}; background: {removed_wash}; }}
.mdow-companion-diff {{ background: {well}; padding: 4px 0; }}
.mdow-diff-row {{ padding: 1px 8px 1px 0; }}
.mdow-diff-context .mdow-diff-text {{ color: {secondary}; }}
.mdow-diff-add {{ background: {added_wash}; box-shadow: inset 2px 0 0 {added}; }}
.mdow-diff-add .mdow-diff-marker {{ color: {added}; }}
.mdow-diff-remove {{ background: {removed_wash}; box-shadow: inset 2px 0 0 {removed}; }}
.mdow-diff-remove .mdow-diff-marker {{ color: {removed}; }}
button.mdow-companion-show-more {{ min-height: 24px; font-size: 11px; font-weight: 500; color: {secondary}; }}
.mdow-companion-error {{ background: {removed_wash}; border: 1px solid {removed_soft}; border-radius: 10px; padding: 10px 12px; }}
button.mdow-companion-source {{ min-height: 20px; padding: 0 4px; font-weight: 400; color: {secondary}; }}
button.mdow-companion-source:hover {{ color: {accent}; }}
.mdow-companion-message .mdow-companion-copy {{ opacity: 0; }}
.mdow-companion-message:hover .mdow-companion-copy, .mdow-companion-copy:focus-visible {{ opacity: 1; }}
.mdow-companion-review {{ background: {bg}; border: 1px solid {accent_soft}; border-radius: 12px; margin: 6px 8px 0 12px; padding: 8px 8px 8px 10px; }}
.mdow-companion-dot {{ min-width: 6px; min-height: 6px; border-radius: 99px; background: {accent}; }}
.mdow-companion-composer {{ background: {bg}; border: 1px solid {line}; border-radius: 12px; margin: 8px 8px 8px 12px; padding: 10px 6px 7px 10px; }}
.mdow-companion-input, .mdow-companion-input text {{ background: none; color: {fg}; }}
.mdow-companion-chip {{ background: {hover}; border: 1px solid {line}; border-radius: 999px; min-height: 18px; padding: 0 7px; }}
.mdow-companion-chip-secondary {{ color: {secondary}; }}
button.mdow-companion-chip-remove {{ min-width: 14px; min-height: 14px; padding: 0; }}
button.mdow-companion-chip-remove image {{ -gtk-icon-size: 8px; }}
button.mdow-companion-model {{ min-height: 24px; }}
.mdow-companion-model > button {{ min-height: 24px; padding: 0 8px; border-radius: 999px; font-weight: 400; color: {secondary}; }}
.mdow-companion-model > button:checked {{ background: {selected}; color: {fg}; }}
button.mdow-companion-send, button.mdow-companion-stop {{ min-width: 26px; min-height: 26px; padding: 0; border-radius: 999px; }}
button.mdow-companion-send {{ background: {accent}; color: {on_accent}; }}
button.mdow-companion-send:disabled {{ background: {hover}; color: {tertiary}; }}
button.mdow-companion-stop {{ background: {fg}; color: {bg}; }}
.mdow-companion-mentions {{ background: {bg}; border: 1px solid {line}; border-radius: 10px; margin: 6px 8px 0 12px; padding: 4px; }}
.mdow-companion-mentions > row {{ padding: 4px 8px; border-radius: 6px; }}
.mdow-companion-mentions > row:hover {{ background: {hover}; }}
.mdow-companion-mentions > row:selected {{ background: {selected}; color: {fg}; }}
.mdow-companion-group {{ font-size: 11px; font-weight: 500; color: {tertiary}; padding: 10px 8px 3px 8px; }}
button.mdow-companion-model-row {{ min-height: 26px; padding: 0 8px; font-weight: 400; border-radius: 6px; }}
"#,
        accent_soft = p.accent.alpha(0.45).css(),
        accent_wash = p.accent.alpha(0.12).css(),
        added_wash = p.added.alpha(0.11).css(),
        removed_wash = p.removed.alpha(0.1).css(),
        removed_soft = p.removed.alpha(0.25).css(),
    )
}

/// Caps every `border-radius` at Hyprland's rounding, so Mdow's corners match its window's.
fn clamp_radii(css: &str, max: u32) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(found) = rest.find("radius: ") {
        let start = found + "radius: ".len();
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let digits = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        if let (Ok(n), true) = (rest[..digits].parse::<u32>(), rest[digits..].starts_with("px")) {
            let _ = write!(out, "{}", n.min(max));
            rest = &rest[digits..];
        }
    }
    out.push_str(rest);
    out
}

/// What makes Mdow look like the rest of Omarchy, on top of the base stylesheet: the system's
/// monospace font (`omarchy font set`), flat surfaces, and the state tokens from the shell's
/// `[controls]` and `[menu]` sections.
fn omarchy_layer(p: &Palette, o: &Omarchy) -> String {
    let radius = o.radius;
    let base = o.base_size;
    let bg = p.background.css();
    let fg = p.foreground.css();
    let chrome = p.chrome.css();
    let accent = p.accent.css();
    let hover = p.hover.css();
    let selected = p.selected.css();
    let pressed = o.pressed.css();
    let hover_border = o.hover_border.css();
    let normal_border = o.normal_border.css();
    let menu_border = o.menu_border.css();
    let popup_border = o.popup_border.css();
    let border = p.border.css();
    let border_subtle = p.border_subtle.css();
    let muted_fg = p.muted_fg.css();
    let scrim = o.scrim.css();
    let on_accent = p.primary_fg.css();
    format!(
        r#"
/* ── Omarchy ── */
window.mdow, dialog-host {{
  font-family: monospace; font-size: {base}px; font-feature-settings: normal;
}}
.mdow-welcome-title {{ font-family: monospace; font-size: 28px; font-weight: 700; letter-spacing: -0.5px; }}
.mdow-kbd, .mdow-keycap, shortcut > .keycap {{ font-family: monospace; }}

/* Controls: the shell's Button states. */
window.mdow button, window.mdow entry, window.mdow searchbar, dialog-host button, dialog-host entry {{
  border-radius: {radius}px; box-shadow: none;
}}
.mdow button.flat:hover, .mdow-icon-button:hover, .mdow-mode-tabs > button:hover,
.mdow-tabbar tabbar tab:hover, .mdow-recent-list button:hover, button.mdow-search:hover,
button.mdow-action-card:hover, .mdow-segmented > button:hover {{
  background: {hover}; box-shadow: inset 0 0 0 1px {hover_border}; color: {fg};
}}
.mdow button:active, .mdow-tabbar tabbar tab:active {{ background: {pressed}; }}
.mdow-icon-button:checked, .mdow-mode-tabs > button:checked, .mdow-segmented > button:checked,
button.mdow-tile:checked {{
  background: {selected}; color: {fg}; box-shadow: none; border-color: transparent;
}}
button.mdow-search, button.mdow-action-card, .mdow-recent-list, button.mdow-outline,
button.mdow-tile {{ border: 1px solid {normal_border}; background: transparent; box-shadow: none; }}
.mdow-segmented {{ background: none; box-shadow: inset 0 0 0 1px {normal_border}; }}
/* GTK marks the focused widget's ancestors focus-visible too, so this names the controls. */
button:focus-visible, row:focus-visible, switch:focus-visible, checkbutton:focus-visible,
tabbar tab:focus-visible, entry:focus-within {{ outline: 2px solid {focus}; outline-offset: -2px; }}
entry text:focus-visible, .mdow-palette entry:focus-within {{ outline: none; }}
.mdow-card:focus-visible {{ border-color: {focus}; }}

/* Tabs: square and flat; the chosen one takes the selected fill. */
.mdow-tabbar tabbar tab {{ margin: 6px 1px; }}
.mdow-tabbar tabbar tab:selected {{ background: {selected}; box-shadow: none; color: {fg}; }}
.mdow-tabbar tabbar tabbox > separator {{ opacity: 0; }}
.mdow-tabbar tabbar tab .tab-close-button:hover {{ background: {hover}; }}

/* The document: a flat sheet with a hairline, like a tiled window inside the window. */
.mdow-card {{ border-color: {border_subtle}; box-shadow: none; }}
.mdow-sidebar row:hover {{ background: {hover}; box-shadow: inset 0 0 0 1px {hover_border}; }}
.mdow-sidebar row:selected, .mdow-sidebar row.mdow-active {{ background: {selected}; }}
.mdow-sidebar row.mdow-active .mdow-row, .mdow-sidebar row:selected .mdow-row {{ color: {accent}; }}
.mdow-outline row.mdow-active .mdow-row {{ color: {accent}; }}
.md-code, .md-table, .md-alert, .mdow-preview {{ box-shadow: none; }}
.md-code {{ border-color: {border_subtle}; }}
.md-th {{ background: {chrome}; }}
.md-task check {{ border-radius: {radius}px; }}
.md-task check:checked {{ color: {on_accent}; }}

/* Floating cards: the shell's menu. No shadows and no blur, one strong border. */
popover > contents {{
  background: {bg}; color: {fg}; border-radius: {radius}px; box-shadow: none;
  border: 2px solid {menu_border}; padding: 4px;
}}
popover > arrow {{ background: none; border: none; }}
popover, popover label, popover accelerator, popover modelbutton {{ font-family: monospace; font-size: {base}px; }}
popover modelbutton {{ border-radius: {radius}px; min-height: 26px; padding: 0 10px; }}
popover modelbutton:hover, popover modelbutton:focus {{
  background: {hover}; box-shadow: inset 0 0 0 1px {hover_border}; color: {accent};
}}
popover separator {{ background: {border_subtle}; margin: 4px 0; }}
tooltip > contents, tooltip.background {{
  background: {bg}; color: {fg}; border-radius: {radius}px; box-shadow: none;
}}
tooltip {{ border: 1px solid {menu_border}; }}
windowcontrols button, windowcontrols button image, dialog-host headerbar button {{ border-radius: {radius}px; }}
floating-sheet > dimming {{ background-color: {scrim}; }}
floating-sheet > sheet {{
  border-radius: {radius}px; box-shadow: none; background: {bg};
  outline: 2px solid {menu_border}; outline-offset: -2px;
}}
dialog-host > dialog.background sheet {{ background-color: {bg}; }}
toast {{
  background: {bg}; color: {fg}; border-radius: {radius}px; box-shadow: none;
  border: 2px solid {popup_border}; margin: 8px;
}}
.mdow-find {{
  background: {bg}; border-radius: 0; box-shadow: none;
  border: none; border-left: 2px solid {menu_border}; border-bottom: 2px solid {menu_border};
}}
.mdow-zoom, .mdow-cheatsheet {{ box-shadow: none; border: 2px solid {menu_border}; }}

/* The companion: same surfaces and states as the rest of the window. */
.mdow-companion-title, .mdow-companion-display {{ font-family: monospace; }}
.mdow-companion-display {{ font-size: 18px; font-weight: 700; }}
.mdow-companion-card, .mdow-companion-change, .mdow-companion-composer, .mdow-companion-review,
button.mdow-companion-suggestion, button.mdow-pill-secondary {{ border-color: {normal_border}; }}
button.mdow-pill-quiet:hover, button.mdow-pill-secondary:hover, button.mdow-companion-suggestion:hover {{
  background: {hover}; box-shadow: inset 0 0 0 1px {hover_border}; color: {fg};
}}
.mdow-companion-change-pending, .mdow-companion-review {{ border-color: {accent}; }}
.mdow-companion-mentions > row:selected {{ background: {hover}; box-shadow: inset 0 0 0 1px {hover_border}; color: {accent}; }}

/* Quick Open: rows like the Omarchy menu's. */
.mdow-palette .mdow-palette-entry {{ min-height: 48px; font-size: {title}px; border-bottom: 1px solid {border}; }}
.mdow-palette list > row {{ padding: 8px 12px; }}
.mdow-palette list > row:selected {{ background: {hover}; box-shadow: inset 0 0 0 1px {hover_border}; color: {accent}; }}
.mdow-palette list > row:selected image {{ color: {accent}; }}
.mdow-palette .mdow-palette-heading {{ color: {muted_fg}; }}
.mdow-palette .mdow-palette-footer {{ background: none; border-top: 1px solid {border_subtle}; }}
"#,
        title = base + 2.0,
        focus = p.foreground.alpha(0.7).css(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklch_matches_known_srgb() {
        // oklch(1 0 0) is white and oklch(0 0 0) black.
        assert_eq!(oklch(1.0, 0.0, 0.0).hex(), "#ffffff");
        assert_eq!(oklch(0.0, 0.0, 0.0).hex(), "#000000");
        let blue = oklch(0.55, 0.17, 260.0);
        assert!(blue.b > blue.r && blue.b > blue.g);
    }

    #[test]
    fn legible_leaves_good_colours_alone_and_fixes_bad_ones() {
        let page = hex("#fffcf0");
        assert_eq!(legible(hex("#100f0f"), &[page], 4.5), hex("#100f0f"));
        let faint = hex("#d0a215");
        let fixed = legible(faint, &[page], 4.5);
        assert!(contrast(fixed, page) >= 4.5);
        assert!(fixed.r < faint.r, "light surfaces push text darker");
        let night = hex("#1a1b26");
        let dim = legible(hex("#414868"), &[night], 4.5);
        assert!(contrast(dim, night) >= 4.5);
        assert!(dim.b > hex("#414868").b, "dark surfaces push text lighter");
    }

    fn assert_readable(name: &str, palette: &Palette) {
        let page = palette.background;
        let code = palette.muted;
        for (role, color, surface) in [
            ("foreground", palette.foreground, page),
            ("secondary text", palette.muted_fg, page),
            ("secondary text on chrome", palette.muted_fg, palette.chrome),
            ("tertiary text", palette.muted_fg_subtle, page),
            ("tertiary text on chrome", palette.muted_fg_subtle, palette.chrome),
            ("links", palette.primary, page),
            ("text on accent", palette.primary_fg, palette.primary),
            ("comments", palette.syntax.comment, code),
            ("keywords", palette.syntax.keyword, code),
            ("strings", palette.syntax.string, code),
            ("functions", palette.syntax.entity, code),
            ("constants", palette.syntax.constant, code),
        ] {
            let ratio = contrast(color, surface);
            assert!(ratio >= 4.5, "{name}: {role} is {ratio:.2}:1");
        }
    }

    #[test]
    fn stock_palettes_are_readable() {
        assert_readable("light", &Palette::light());
        assert_readable("dark", &Palette::dark());
    }

    /// Runs against whatever themes this machine has, so it does nothing on CI.
    #[test]
    fn installed_omarchy_themes_are_readable() {
        let Ok(themes) = std::fs::read_dir("/usr/share/omarchy/themes") else { return };
        for theme in themes.flatten() {
            let Ok(colors) = std::fs::read_to_string(theme.path().join("colors.toml")) else {
                continue;
            };
            let parsed = crate::omarchy::tests::parse_theme(&colors);
            let name = theme.file_name().to_string_lossy().into_owned();
            assert_readable(&name, &Palette::from_omarchy(&parsed));
        }
    }

    #[test]
    fn radii_are_clamped_but_percentages_are_kept() {
        let css = ".a { border-radius: 12px; } .b { border-bottom-left-radius: 3px; } .c { border-radius: 22%; }";
        assert_eq!(
            clamp_radii(css, 4),
            ".a { border-radius: 4px; } .b { border-bottom-left-radius: 3px; } .c { border-radius: 22%; }"
        );
    }

    #[test]
    fn omarchy_palette_comes_from_the_theme() {
        let theme = crate::omarchy::tests::sample();
        let palette = Palette::from_omarchy(&theme);
        assert_eq!(palette.background.hex(), "#1a1b26");
        assert_eq!(palette.chrome.hex(), "#13141c");
        assert_eq!(palette.primary.hex(), "#7aa2f7");
        assert!(palette.dark);
        let css = stylesheet(&palette, &Prefs::default());
        assert!(css.contains("font-family: monospace"));
        assert!(!css.contains("border-radius: 12px"), "corners follow Hyprland's rounding");
    }

    #[test]
    fn stylesheet_has_every_heading() {
        let css = stylesheet(&Palette::light(), &Prefs::default());
        for n in 1..=6 {
            assert!(css.contains(&format!(".md-h{n} ")));
        }
    }
}
