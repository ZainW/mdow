//! Mdow on Omarchy: the current theme's colours and shell tokens, Hyprland's corner radius, and
//! the system's own helpers for editors, terminals and sharing.
//!
//! Omarchy keeps the applied theme in `~/.local/state/omarchy/current/theme`. `colors.toml` holds
//! the palette (resolved with the same fallbacks as `omarchy-theme-color`) and the generated
//! `shell.toml` holds the state tokens every first-party surface uses. Mdow reads both rather than
//! shipping a copy, so any theme, stock or third-party, applies as soon as it is set.

use crate::theme::{Rgba, hex};
use gtk::{gio, glib, prelude::*};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub colors: HashMap<String, Rgba>,
    pub dark: bool,
    /// Hyprland's `decoration:rounding`, which the shell mirrors.
    pub rounding: u32,
    pub normal_border_alpha: f64,
    pub hover_fill: f64,
    pub hover_border_alpha: f64,
    pub selected_fill: f64,
    pub pressed_fill: f64,
    pub selection_fill: f64,
    /// The border on menus and other floating cards.
    pub menu_border: Rgba,
    /// The border on popups, which conventionally matches the active window's.
    pub popup_border: Rgba,
    pub scrim_alpha: f64,
    /// `[font] base-size`, the root of the shell's type scale.
    pub base_size: f64,
}

impl Theme {
    pub fn color(&self, key: &str) -> Rgba {
        self.colors.get(key).copied().unwrap_or_else(|| hex("#808080"))
    }

    /// A short name for this palette, for caches of things rendered in its colours.
    pub fn tag(&self) -> String {
        format!("{}{}", self.color("background").hex(), self.color("accent").hex())
    }
}

fn state_dir() -> PathBuf {
    glib::home_dir().join(".local/state/omarchy/current")
}

fn theme_dir() -> PathBuf {
    match std::env::var_os("MDOW_OMARCHY_THEME") {
        Some(dir) => PathBuf::from(dir),
        None => state_dir().join("theme"),
    }
}

/// The applied theme, or `None` away from Omarchy.
pub fn load() -> Option<Theme> {
    if std::env::var_os("MDOW_OMARCHY").is_some_and(|v| v == "off") {
        return None;
    }
    let dir = theme_dir();
    let colors = std::fs::read_to_string(dir.join("colors.toml")).ok()?;
    let shell = std::fs::read_to_string(dir.join("shell.toml")).unwrap_or_default();
    Some(build_theme(&colors, &shell, dir.join("light.mode").exists(), hyprland_rounding()))
}

fn hyprland_rounding() -> u32 {
    Command::new("hyprctl")
        .args(["getoption", "decoration:rounding", "-j"])
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| parse_rounding(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or(0)
}

fn parse_rounding(json: &str) -> Option<u32> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value.get("int")?.as_u64().map(|n| n as u32)
}

/// `key = value` pairs from a flat TOML file, ignoring comments. Section names prefix their keys,
/// as in `controls.hover-cursor-fill-alpha`.
fn parse_pairs(text: &str) -> HashMap<String, String> {
    let mut pairs = HashMap::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = format!("{name}.");
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
        pairs.insert(format!("{section}{key}"), value);
    }
    pairs
}

fn parse_color(value: &str) -> Option<Rgba> {
    let value = value.trim();
    let digits = value.strip_prefix('#')?;
    let full = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => digits.to_string(),
        _ => return None,
    };
    full.chars().all(|c| c.is_ascii_hexdigit()).then(|| hex(&full))
}

/// The cascade from `omarchy-theme-color`, for themes that don't define every key: legacy short
/// names, then ANSI `colorN` names, then shades derived from the base colours.
fn resolve_colors(raw: &HashMap<String, String>) -> HashMap<String, Rgba> {
    let get = |key: &str| raw.get(key).and_then(|v| parse_color(v));
    let first = |keys: &[&str]| keys.iter().find_map(|key| get(key));
    let background = first(&["background", "bg", "color0"]).unwrap_or_else(|| hex("#1a1b26"));
    let foreground = first(&["foreground", "fg", "color7"]).unwrap_or_else(|| hex("#c0caf5"));
    let black = hex("#000000");
    let white = hex("#ffffff");

    let red = first(&["red", "color1"]).unwrap_or(foreground);
    let green = first(&["green", "color2"]).unwrap_or(foreground);
    let yellow = first(&["yellow", "color3"]).unwrap_or(foreground);
    let blue = first(&["blue", "color4"]).unwrap_or(foreground);
    let magenta = first(&["magenta", "purple", "color5"]).unwrap_or(foreground);
    let cyan = first(&["cyan", "color6"]).unwrap_or(foreground);
    let orange = first(&["orange"]).unwrap_or(yellow);
    let dark_foreground = first(&["dark_foreground", "dark_fg", "color8"]).unwrap_or(foreground);

    let mut colors = HashMap::new();
    let mut put = |key: &str, color: Rgba| {
        colors.insert(key.to_string(), color);
    };
    put("background", background);
    put("foreground", foreground);
    put("red", red);
    put("green", green);
    put("yellow", yellow);
    put("blue", blue);
    put("magenta", magenta);
    put("cyan", cyan);
    put("orange", orange);
    put("dark_foreground", dark_foreground);
    put("accent", first(&["accent"]).unwrap_or(blue));
    put("muted", first(&["muted", "color8"]).unwrap_or(dark_foreground));
    put("selection", first(&["selection", "selection_background", "color8"]).unwrap_or(background));
    put(
        "dark_background",
        first(&["dark_background", "dark_bg"]).unwrap_or_else(|| background.mix(black, 0.25)),
    );
    put(
        "darker_background",
        first(&["darker_background", "darker_bg"]).unwrap_or_else(|| background.mix(black, 0.5)),
    );
    put("lighter_background", first(&["lighter_background", "lighter_bg"]).unwrap_or(background));
    put(
        "bright_foreground",
        first(&["bright_foreground", "bright_fg", "color15"]).unwrap_or(foreground),
    );
    put(
        "bright_magenta",
        first(&["bright_magenta", "bright_purple", "color13"])
            .unwrap_or_else(|| magenta.mix(white, 0.2)),
    );
    put("bright_blue", first(&["bright_blue", "color12"]).unwrap_or_else(|| blue.mix(white, 0.2)));
    colors
}

/// Theme mode, in `omarchy-theme-color`'s order: `mode`, legacy `theme_type`, a `light.mode` file,
/// then the background's brightness.
fn resolve_dark(raw: &HashMap<String, String>, light_file: bool, background: Rgba) -> bool {
    match raw.get("mode").or_else(|| raw.get("theme_type")).map(String::as_str) {
        Some("light") => false,
        Some("dark") => true,
        _ if light_file => false,
        _ => background.r + background.g + background.b <= 1.5,
    }
}

/// A shell colour token: a colour, a reference like `hyprland.active-border`, or a Hyprland-style
/// gradient, of which GTK borders can only show the first colour.
fn shell_color(shell: &HashMap<String, String>, key: &str, fallback: Rgba) -> Rgba {
    let Some(value) = shell.get(key) else { return fallback };
    if let Some(color) = parse_color(value) {
        return color;
    }
    if shell.contains_key(value) {
        return shell_color(shell, value, fallback);
    }
    first_gradient_color(value).unwrap_or(fallback)
}

fn first_gradient_color(value: &str) -> Option<Rgba> {
    let token = value.split_whitespace().next()?;
    if let Some(color) = parse_color(token) {
        return Some(color);
    }
    let digits = token.strip_prefix("rgba(").or_else(|| token.strip_prefix("rgb("))?;
    let digits = digits.trim_end_matches(')');
    parse_color(&format!("#{}", digits.get(..6)?))
}

fn build_theme(colors: &str, shell: &str, light_file: bool, rounding: u32) -> Theme {
    let raw = parse_pairs(colors);
    let shell = parse_pairs(shell);
    let colors = resolve_colors(&raw);
    let dark = resolve_dark(&raw, light_file, colors["background"]);
    let num = |key: &str, fallback: f64| {
        shell.get(key).and_then(|v| v.parse::<f64>().ok()).unwrap_or(fallback)
    };
    let foreground = colors["foreground"];
    let accent = colors["accent"];
    Theme {
        dark,
        rounding,
        normal_border_alpha: num("controls.normal-border-alpha", 0.4),
        hover_fill: num("controls.hover-cursor-fill-alpha", 0.08),
        hover_border_alpha: num("controls.hover-cursor-border-alpha", 0.25),
        selected_fill: num("controls.selected-fill-alpha", 0.18),
        pressed_fill: num("controls.pressed-fill-alpha", 0.22),
        selection_fill: num("controls.selection-fill-alpha", 0.35),
        menu_border: shell_color(&shell, "menu.border", foreground),
        popup_border: shell_color(&shell, "popups.border", accent),
        scrim_alpha: num("menu.scrim-alpha", 0.5),
        base_size: num("font.base-size", 12.0),
        colors,
    }
}

/// Calls `changed` after `omarchy theme set`. The theme directory is swapped as a whole, so this
/// watches its parent, and waits for the swap (and Hyprland's reload) to finish.
pub fn watch(changed: impl Fn() + 'static) -> Option<gio::FileMonitor> {
    let dir = match std::env::var_os("MDOW_OMARCHY_THEME") {
        Some(_) => return None,
        None => state_dir(),
    };
    let monitor = gio::File::for_path(dir)
        .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        .ok()?;
    let pending = std::cell::RefCell::new(None::<glib::JoinHandle<()>>);
    let changed = std::rc::Rc::new(changed);
    monitor.connect_changed(move |_, _, _, _| {
        if let Some(handle) = pending.borrow_mut().take() {
            handle.abort();
        }
        let changed = changed.clone();
        let handle = glib::spawn_future_local(async move {
            glib::timeout_future(Duration::from_millis(350)).await;
            changed();
        });
        pending.replace(Some(handle));
    });
    Some(monitor)
}

fn spawn(program: &str, args: &[&str], cwd: Option<&Path>) -> bool {
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    match command.spawn() {
        Ok(mut child) => {
            // Reap it off the main thread: these helpers run for as long as the app they launch.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            true
        }
        Err(_) => false,
    }
}

/// Opens a file in the editor chosen with `omarchy default editor`.
pub fn open_in_editor(path: &Path) -> bool {
    spawn("omarchy-launch-editor", &[&path.to_string_lossy()], path.parent())
}

/// Opens the default terminal in a folder.
pub fn open_terminal(dir: &Path) -> bool {
    spawn("xdg-terminal-exec", &[&format!("--dir={}", dir.display())], Some(dir))
}

/// Hands a file to Omarchy's share menu (LocalSend).
pub fn share_file(path: &Path) -> bool {
    spawn("omarchy-menu-share", &["file", &path.to_string_lossy()], path.parent())
}

pub fn open_theme_switcher() -> bool {
    spawn("omarchy-theme-switcher", &[], None)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn parse_theme(colors: &str) -> Theme {
        build_theme(colors, "", false, 0)
    }

    pub fn sample() -> Theme {
        build_theme(TOKYO, "", false, 0)
    }

    const TOKYO: &str = r##"
mode = "dark"
accent = "#7aa2f7"
background = "#1a1b26"
dark_background = "#13141c"
foreground = "#a9b1d6"
# a comment
blue = "#7aa2f7"
red = '#f7768e' # inline
"##;

    #[test]
    fn reads_colours_and_mode() {
        let theme = build_theme(TOKYO, "", false, 8);
        assert!(theme.dark);
        assert_eq!(theme.rounding, 8);
        assert_eq!(theme.color("accent").hex(), "#7aa2f7");
        assert_eq!(theme.color("dark_background").hex(), "#13141c");
        assert_eq!(theme.color("red").hex(), "#f7768e");
    }

    #[test]
    fn follows_the_legacy_cascade() {
        let legacy = r##"
color0 = "#ffffff"
color1 = "#aa0000"
color4 = "#0000aa"
color7 = "#111111"
color8 = "#777777"
"##;
        let theme = build_theme(legacy, "", false, 0);
        assert!(!theme.dark, "a bright background with no mode reads as light");
        assert_eq!(theme.color("background").hex(), "#ffffff");
        assert_eq!(theme.color("red").hex(), "#aa0000");
        assert_eq!(theme.color("accent").hex(), "#0000aa", "accent falls back to blue");
        assert_eq!(theme.color("muted").hex(), "#777777");
        assert_eq!(theme.color("dark_background").hex(), "#bfbfbf", "25% toward black");
    }

    #[test]
    fn light_mode_marker_wins_over_a_dark_background() {
        let theme = build_theme("background = \"#101010\"", "", true, 0);
        assert!(!theme.dark);
    }

    #[test]
    fn reads_shell_tokens_and_follows_references() {
        let shell = r##"
[hyprland]
active-border = "rgba(33ccffee) rgba(00ff99ee) 45deg"
active-border-foreground = "#100F0F"

[controls]
hover-cursor-fill-alpha = 0.12

[menu]
border = "hyprland.active-border-foreground"

[popups]
border = "hyprland.active-border"

[font]
base-size = 14
"##;
        let theme = build_theme(TOKYO, shell, false, 0);
        assert_eq!(theme.hover_fill, 0.12);
        assert_eq!(theme.selected_fill, 0.18, "unset tokens keep the shell's defaults");
        assert_eq!(theme.menu_border.hex(), "#100f0f");
        assert_eq!(theme.popup_border.hex(), "#33ccff", "a gradient shows its first colour");
        assert_eq!(theme.base_size, 14.0);
    }

    #[test]
    fn reads_hyprctl_rounding() {
        let json = r#"{"option": "decoration:rounding", "int": 6, "set": true }"#;
        assert_eq!(parse_rounding(json), Some(6));
        assert_eq!(parse_rounding("not json"), None);
    }
}
