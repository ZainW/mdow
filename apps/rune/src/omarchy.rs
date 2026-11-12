//! Mdow on Omarchy: the current theme's colours, as the app's palette.
//!
//! Omarchy keeps the applied theme in `~/.local/state/omarchy/current/theme`. Its `colors.toml`
//! holds the palette, resolved here with the same fallbacks as `omarchy-theme-color`, so any
//! theme works, stock or third-party, and `omarchy theme set` recolours open windows at once.
//! Settings › Appearance › Light or Dark goes back to Mdow's own colours, `MDOW_OMARCHY=off`
//! turns this off, and `MDOW_OMARCHY_THEME=<dir>` reads a theme from somewhere else.

use crate::theme::Palette;
use rune::Color;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static CURRENT: Mutex<Option<Palette>> = Mutex::new(None);
static REVISION: AtomicU64 = AtomicU64::new(0);

/// The Omarchy theme's palette, when there is one.
pub fn palette() -> Option<Palette> {
    *CURRENT.lock().unwrap()
}

/// Bumped each time the theme changes.
pub fn revision() -> u64 {
    REVISION.load(Ordering::Relaxed)
}

fn state_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    Some(home.join(".local/state/omarchy/current"))
}

fn theme_dir() -> Option<PathBuf> {
    match std::env::var_os("MDOW_OMARCHY_THEME") {
        Some(dir) => Some(PathBuf::from(dir)),
        None => Some(state_dir()?.join("theme")),
    }
}

fn load() -> Option<Palette> {
    let dir = theme_dir()?;
    let colors = std::fs::read_to_string(dir.join("colors.toml")).ok()?;
    Some(build(&colors, dir.join("light.mode").exists(), hyprland_rounding()))
}

/// Reads the theme and follows it from then on. Does nothing off Linux, in headless runs
/// (which must be repeatable) and with `MDOW_OMARCHY=off`.
pub fn start() {
    if !cfg!(target_os = "linux")
        || rune::is_headless()
        || std::env::var_os("MDOW_OMARCHY").is_some_and(|v| v == "off")
    {
        return;
    }
    *CURRENT.lock().unwrap() = load();
    if palette().is_none() || std::env::var_os("MDOW_OMARCHY_THEME").is_some() {
        return;
    }
    let Some(dir) = state_dir() else { return };
    std::thread::spawn(move || {
        use notify::Watcher as _;
        let (tx, rx) = std::sync::mpsc::channel();
        let Ok(mut watcher) = notify::recommended_watcher(move |_| {
            let _ = tx.send(());
        }) else {
            return;
        };
        // The theme directory is swapped as a whole, so watch its parent.
        if watcher.watch(&dir, notify::RecursiveMode::NonRecursive).is_err() {
            return;
        }
        while rx.recv().is_ok() {
            // Wait for the swap (and Hyprland's reload) to finish.
            while rx.recv_timeout(std::time::Duration::from_millis(350)).is_ok() {}
            let next = load();
            let mut current = CURRENT.lock().unwrap();
            if *current != next {
                *current = next;
                drop(current);
                REVISION.fetch_add(1, Ordering::Relaxed);
                crate::windows::wake_all();
            }
        }
    });
}

/// Hyprland's `decoration:rounding`, which Omarchy's own surfaces mirror.
fn hyprland_rounding() -> f32 {
    std::process::Command::new("hyprctl")
        .args(["getoption", "decoration:rounding", "-j"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .and_then(|output| {
            let value: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
            value.get("int")?.as_f64()
        })
        .map_or(0.0, |n| n as f32)
}

/// `key = value` pairs from a flat TOML file, ignoring comments and sections.
fn parse_pairs(text: &str) -> HashMap<String, String> {
    let mut pairs = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim().trim_matches(['"', '\'']);
        let value = value.trim();
        let value = match value.chars().next() {
            Some(quote @ ('"' | '\'')) => {
                value[1..].split(quote).next().unwrap_or_default().to_string()
            }
            _ => value.split('#').next().unwrap_or_default().trim().to_string(),
        };
        pairs.insert(key.to_string(), value);
    }
    pairs
}

fn parse_color(value: &str) -> Option<Color> {
    let digits = value.trim().strip_prefix('#')?;
    let full = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => digits.to_string(),
        _ => return None,
    };
    rune::color::hex(&format!("#{full}"))
}

/// A colour's red, green and blue, 0 to 1.
fn rgb(c: Color) -> [f64; 3] {
    let hex = c.hex();
    let part = |i: usize| {
        u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as f64 / 255.0
    };
    [part(1), part(3), part(5)]
}

/// WCAG relative luminance.
fn luminance(c: Color) -> f64 {
    let lin = |v: f64| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    let [r, g, b] = rgb(c);
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

fn contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The nearest colour to `color` that reaches `target` against every surface it sits on.
/// Colours that already pass are untouched, so a theme only changes where it would be hard to
/// read.
fn legible(color: Color, surfaces: &[Color], target: f64) -> Color {
    let passes = |c: Color| surfaces.iter().all(|s| contrast(c, *s) >= target);
    if passes(color) {
        return color;
    }
    let darkest = surfaces.iter().map(|s| luminance(*s)).fold(1.0, f64::min);
    let lightest = surfaces.iter().map(|s| luminance(*s)).fold(0.0, f64::max);
    let goal = if lightest + darkest > 0.36 { Color::BLACK } else { Color::WHITE };
    (1..=20).map(|step| color.mix(goal, step as f32 / 20.0)).find(|c| passes(*c)).unwrap_or(goal)
}

fn build(colors: &str, light_file: bool, rounding: f32) -> Palette {
    let raw = parse_pairs(colors);
    let get = |key: &str| raw.get(key).and_then(|v| parse_color(v));
    let first = |keys: &[&str]| keys.iter().find_map(|key| get(key));
    let fallback = |hex: &str| rune::color::hex(hex).unwrap_or(Color::BLACK);

    // The cascade from `omarchy-theme-color`: legacy short names, then ANSI `colorN` names,
    // then shades derived from the base colours.
    let bg = first(&["background", "bg", "color0"]).unwrap_or_else(|| fallback("#1a1b26"));
    let fg = first(&["foreground", "fg", "color7"]).unwrap_or_else(|| fallback("#c0caf5"));
    let red = first(&["red", "color1"]).unwrap_or(fg);
    let green = first(&["green", "color2"]).unwrap_or(fg);
    let yellow = first(&["yellow", "color3"]).unwrap_or(fg);
    let blue = first(&["blue", "color4"]).unwrap_or(fg);
    let magenta = first(&["magenta", "purple", "color5"]).unwrap_or(fg);
    let orange = first(&["orange"]).unwrap_or(yellow);
    let dark_fg = first(&["dark_foreground", "dark_fg", "color8"]).unwrap_or(fg);
    let muted = first(&["muted", "color8"]).unwrap_or(dark_fg);
    let accent = first(&["accent"]).unwrap_or(blue);
    let selection = first(&["selection", "selection_background", "color8"]).unwrap_or(bg);
    let chrome =
        first(&["dark_background", "dark_bg"]).unwrap_or_else(|| bg.mix(Color::BLACK, 0.25));
    let bright_magenta = first(&["bright_magenta", "bright_purple", "color13"])
        .unwrap_or_else(|| magenta.mix(Color::WHITE, 0.2));

    // Theme mode, in `omarchy-theme-color`'s order: `mode`, legacy `theme_type`, a `light.mode`
    // file, then the background's brightness.
    let dark = match raw.get("mode").or_else(|| raw.get("theme_type")).map(String::as_str) {
        Some("light") => false,
        Some("dark") => true,
        _ if light_file => false,
        _ => rgb(bg).iter().sum::<f64>() <= 1.5,
    };

    // Several themes make dark_background the same as background, which would lose code blocks
    // and table headers against the page.
    let well = if contrast(chrome, bg) < 1.04 { bg.mix(fg, 0.05) } else { chrome };
    let text = |color: Color| legible(color, &[bg, chrome], 4.5);
    let code = |color: Color| legible(color, &[well, bg], 4.5);
    let bar = |color: Color| legible(color, &[well], 3.0);
    let accent = text(accent);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&(bg.hex(), fg.hex(), accent.hex(), chrome.hex()), &mut hasher);
    // Ids 0 and 1 are Mdow's own light and dark.
    let id = (std::hash::Hasher::finish(&hasher) as u32).max(2);

    Palette {
        id,
        dark,
        // Omarchy's surfaces are flat and square, or as round as Hyprland's windows.
        radius: rounding.clamp(0.0, 16.0),
        chrome,
        paper: bg,
        elevated: bg,
        well,
        text: fg,
        secondary: text(fg.mix(bg, 0.3)),
        tertiary: text(fg.mix(bg, 0.45)),
        line: muted.mix(bg, 0.45),
        strong_line: muted,
        hover: fg.alpha(0.08),
        selected: fg.alpha(0.18),
        accent,
        accent_soft: accent.alpha(0.14),
        selection: selection.alpha(0.6),
        inline_code_bg: fg.alpha(0.08),
        quote_bar: fg.alpha(0.25),
        mark: yellow.alpha(0.4),
        find_match: yellow.alpha(0.4),
        find_current: accent.alpha(0.5),
        shadow: Color::BLACK.alpha(0.0),
        syntax: [code(dark_fg), code(bright_magenta), code(green), code(blue), code(orange)],
        alerts: [bar(blue), bar(green), bar(magenta), bar(yellow), bar(red)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLEXOKI: &str = "mode = \"light\"\naccent = \"#205EA6\"\nbackground = \"#FFFCF0\"\n\
        dark_background = \"#f2efe4\"\nforeground = \"#100F0F\"\ndark_foreground = \"#878580\"\n\
        muted = \"#B7B5AC\"\nselection = \"#CECDC3\"\nyellow = \"#D0A215\"\ngreen = \"#879A39\"\n";

    #[test]
    fn reads_colours_and_mode() {
        let p = build(FLEXOKI, false, 0.0);
        assert!(!p.dark);
        assert_eq!(p.paper.hex()[..7].to_lowercase(), "#fffcf0");
        assert_eq!(p.chrome.hex()[..7].to_lowercase(), "#f2efe4");
        assert_eq!(p.radius, 0.0);
        assert!(p.id >= 2);
    }

    #[test]
    fn follows_the_legacy_cascade() {
        let p = build("bg = \"#101010\"\nfg = \"#eeeeee\"\ncolor4 = \"#3366ff\"\n", false, 24.0);
        assert!(p.dark, "a dark background with no mode is a dark theme");
        assert_eq!(p.paper.hex()[..7], *"#101010");
        assert_eq!(p.radius, 16.0, "rounding is capped");
        assert!(!build("bg = \"#101010\"\n", true, 0.0).dark, "light.mode wins");
    }

    #[test]
    fn text_stays_readable_on_every_surface() {
        let p = build(FLEXOKI, false, 0.0);
        for (role, color) in
            [("secondary", p.secondary), ("tertiary", p.tertiary), ("accent", p.accent)]
        {
            for surface in [p.paper, p.chrome] {
                assert!(contrast(color, surface) >= 4.5, "{role} on {}", surface.hex());
            }
        }
        // The theme's own yellow is too faint for code on paper, so it is nudged darker.
        for token in p.syntax {
            assert!(contrast(token, p.well) >= 4.5 && contrast(token, p.paper) >= 4.5);
        }
    }
}
