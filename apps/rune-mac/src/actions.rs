//! `MDOW_ACTIONS` runs steps once a window has launched, for checking things in the real app
//! without clicking, as the Swift app's variable of the same name does. Steps are separated by
//! `;`: `export=/tmp/out.pdf`, `find=text`, `palette`, `settings`, `split`, `appearance=dark`,
//! `newtab`, `open=/path`, `zoom=in|out`, `companion`, `selectall`.

use crate::app::{App, Cmd, QuickOpen};
use crate::prefs::{self, Appearance};
use rune::{TextEdit, Tx};

pub fn steps() -> Vec<(String, String)> {
    let Ok(raw) = std::env::var("MDOW_ACTIONS") else { return Vec::new() };
    raw.split(';')
        .filter(|s| !s.trim().is_empty())
        .map(|s| match s.split_once('=') {
            Some((k, v)) => (k.trim().to_string(), v.trim().to_string()),
            None => (s.trim().to_string(), String::new()),
        })
        .collect()
}

pub fn run(s: &mut App, step: &str, arg: &str, tx: &mut Tx) {
    match step {
        "export" => {
            let title = s.window_title();
            if let Some(d) = s.tab().pane().doc.as_ref() {
                let page = if d.html {
                    None
                } else {
                    crate::export::write_temp(&d.doc, &d.path, s.prefs.body_font.family())
                };
                crate::webview::print(&s.key, s.tab().pane().id, page, Some(arg.into()), &title);
            }
        }
        "find" => {
            s.run(Cmd::Find, tx);
            if let Some(f) = s.find.as_mut() {
                f.query.replace_text(arg);
            }
            s.find_changed(tx);
        }
        "palette" => {
            let mut query = TextEdit::new(arg);
            query.focused = true;
            s.quick_open = Some(QuickOpen { query, selected: 0 });
        }
        "settings" => s.run(Cmd::Settings, tx),
        "shortcuts" => s.run(Cmd::Shortcuts, tx),
        "split" => s.run(Cmd::ToggleSplit, tx),
        "tab" => s.run(Cmd::SelectTab(arg.parse::<usize>().unwrap_or(1).saturating_sub(1)), tx),
        "quit" => tx.quit(),
        // Glides down the document and logs the frames drawn on the way.
        "glide" => {
            // `distance` or `distance,seconds`.
            let mut parts = arg.split(',');
            let distance: f32 = parts.next().and_then(|d| d.parse().ok()).unwrap_or(12_600.0);
            let seconds: f32 = parts.next().and_then(|d| d.parse().ok()).unwrap_or(3.0);
            let pane = s.tab().pane().id;
            crate::perf::bench(seconds as f64 + 0.2);
            tx.scroll_to_with(
                crate::reader::scroll_key(pane),
                0.0,
                distance,
                rune::Spring::new(seconds, 0.0),
            );
        }
        "newtab" => s.run(Cmd::NewTab, tx),
        "companion" => s.run(Cmd::ToggleCompanion, tx),
        "selectall" => {
            s.select_all(tx);
        }
        "open" => s.run(Cmd::Open(vec![arg.into()]), tx),
        "zoom" => s.run(if arg == "out" { Cmd::ZoomOut } else { Cmd::ZoomIn }, tx),
        "appearance" => {
            let a = match arg {
                "dark" => Appearance::Dark,
                "light" => Appearance::Light,
                _ => Appearance::System,
            };
            s.prefs = prefs::update(|p| p.appearance = a);
        }
        _ => eprintln!("MDOW_ACTIONS: unknown step {step}"),
    }
}
