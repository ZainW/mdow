//! The Companion section of Settings: what it runs on, whether it's connected, and the model new
//! conversations start with. Ported from the Swift app's `CompanionSettingsSection.swift`, in the
//! look of `crate::sheets`' groups.

use super::agents::Agent;
use super::picker::{self, PickerFor};
use super::service;
use super::types::Availability;
use crate::app::App;
use crate::icons;
use crate::theme::{self, Palette};
use rune::{El, Tx, col, el, row, text};

/// The section, with the searchable model picker mounted in `layer`. Push `layer` after the
/// Settings sheet, so the picker sits above it: inside the sheet its scroll view would clip it.
pub fn settings_group_layered(s: &App, p: &Palette, k: &rune_kit::Kit, layer: &mut Vec<El>) -> El {
    picker::overlay(s, PickerFor::Settings, p, k, layer);
    group(s, p, true)
}

fn group(s: &App, p: &Palette, layered: bool) -> El {
    let c = &s.companion;
    let agent = c.agent;
    let saved = s.prefs.companion_model.clone();
    let available = c.status.as_ref().is_some_and(|st| st.availability == Availability::Available);
    let status_text = match &c.status {
        None => "Starts when you open the companion".to_string(),
        Some(st) if st.availability == Availability::Available => {
            format!("Connected to {} {}", agent.name(), st.version.clone().unwrap_or_default())
                .trim()
                .to_string()
        }
        Some(st) => st.detail.clone().unwrap_or_else(|| "Not connected".into()),
    };
    let status = row()
        .gap(7.0)
        .center_y()
        .max_w(280.0)
        .child(el().size(6.0, 6.0).radius(3.0).fill(if available { p.accent } else { p.tertiary }))
        .child(
            text(status_text)
                .font(theme::ui(12.5, 400.0))
                .color(p.secondary)
                .wrap()
                .align(rune::Align::End)
                .shrink(),
        );

    let model_name = match &saved {
        None => "OpenCode’s default".to_string(),
        Some(v) => c
            .models
            .options
            .iter()
            .find(|o| &o.value == v)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| v.clone()),
    };
    let model = if c.models.options.is_empty() {
        text(model_name).font(theme::mono(11.5, 400.0)).color(p.secondary)
    } else {
        let menu = super::view::model_menu(&c.models, true);
        let trigger = row()
            .key("companion-default-model")
            .h(28.0)
            .gap(6.0)
            .center_y()
            .px(10.0)
            .radius(6.0)
            .fill(p.well)
            .border(1.0, p.line)
            .hover(p.hover)
            .role("combobox")
            .label(format!("Default model: {model_name}"))
            .focusable(true)
            .child(text(model_name).font(theme::ui(12.5, 500.0)).color(p.text).truncate())
            .child(rune::icon(&icons::CHEVRON_DOWN, 12.0).color(p.tertiary))
            .expanded(c.picker == Some(PickerFor::Settings));
        if layered {
            trigger.on_press_tx(|s: &mut App, tx: &mut Tx| picker::open(s, PickerFor::Settings, tx))
        } else {
            trigger.on_press_tx(move |_s: &mut App, tx: &mut Tx| {
                tx.popup_menu(
                    menu.clone(),
                    "companion-default-model",
                    |_s: &mut App, picked: Option<&str>, _tx: &mut Tx| {
                        if let Some(v) = picked {
                            service::set_default_model((!v.is_empty()).then(|| v.to_string()));
                        }
                    },
                );
            })
        }
    };

    let agent_menu = row()
        .key("companion-settings-agent")
        .h(28.0)
        .gap(6.0)
        .center_y()
        .px(10.0)
        .radius(6.0)
        .fill(p.well)
        .border(1.0, p.line)
        .hover(p.hover)
        .role("combobox")
        .label(format!("Agent: {}", agent.name()))
        .focusable(true)
        .child(text(agent.name()).font(theme::ui(12.5, 500.0)).color(p.text))
        .child(rune::icon(&icons::CHEVRON_DOWN, 12.0).color(p.tertiary))
        .on_press_tx(|s: &mut App, tx: &mut Tx| {
            let current = s.companion.agent;
            let mut menu = rune::PopupMenu::new();
            for a in Agent::ALL {
                menu = menu.item(
                    rune::PopupItem::new(a.raw(), format!("{} — {}", a.name(), a.blurb()))
                        .checked(a == current),
                );
            }
            tx.popup_menu(
                menu,
                "companion-settings-agent",
                |s: &mut App, picked: Option<&str>, _tx: &mut Tx| {
                    if let Some(a) = picked.and_then(Agent::from_raw) {
                        s.companion.choose_agent(a);
                    }
                },
            );
        });
    let mut rows = vec![
        setting(
            "Agent",
            Some("What the companion runs on. Another agent starts a new chat."),
            agent_menu,
            p,
        ),
        setting("Status", None, status, p),
    ];
    if !agent.is_acp() {
        rows.push(setting(
            "Default model",
            Some("New conversations start with this model."),
            model,
            p,
        ));
    }
    let mut box_ = col().w_full().radius(10.0).fill(p.paper).border(1.0, p.line);
    let n = rows.len();
    for (i, r) in rows.into_iter().enumerate() {
        let r = r.px(14.0).py(10.0);
        box_ = box_.child(if i + 1 < n { r.border_sides([0.0, 0.0, 1.0, 0.0], p.line) } else { r });
    }
    col()
        .key("companion-settings")
        .w_full()
        .gap(8.0)
        .child(text("COMPANION").font(rune::TextStyle { tracking: 0.5, ..theme::ui(10.5, 600.0) }).color(p.tertiary).px(4.0))
        .child(box_)
        .child(
            text(match agent {
                Agent::OpenCode => "OpenCode 2 runs on this Mac and uses the providers you have signed in to there. You can also choose a model from the companion’s message box.",
                _ => "Claude, Codex and Cursor run through the Agent Client Protocol with your own plans. Choose the model and effort from the companion’s message box; each agent keeps its own.",
            })
                .font(theme::ui(12.0, 400.0).lh(17.0))
                .color(p.tertiary)
                .wrap()
                .px(4.0)
                .pt(2.0),
        )
}

fn setting(label: &str, detail: Option<&str>, control: El, p: &Palette) -> El {
    let mut left = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(2.0)
        .child(text(label.to_string()).font(theme::ui(13.0, 500.0)).color(p.text));
    if let Some(d) = detail {
        left =
            left.child(text(d.to_string()).font(theme::ui(11.5, 400.0)).color(p.tertiary).wrap());
    }
    row().w_full().gap(16.0).center_y().child(left).child(control)
}
