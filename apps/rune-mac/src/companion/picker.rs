//! The model picker: a searchable popover grouped by provider, as OpenCode can offer hundreds of
//! models. Ported from the Swift app's `ModelPicker` and `ModelList`. The composer's picker
//! switches the conversation's model; the one in Settings sets the default for new ones.

use super::service;
use super::types::*;
use crate::app::{App, Sheet};
use crate::icons;
use crate::theme::{self, Palette};
use rune::{El, Key, KeyEvent, TextEdit, Tx, col, el, icon, row, text};
use rune_kit::{Placement, popover};

/// Which picker is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerFor {
    /// The composer's: switches this conversation's model.
    Composer,
    /// Settings': the model new conversations start with.
    Settings,
}

impl PickerFor {
    /// The trigger the popover hangs from.
    pub fn anchor(self) -> &'static str {
        match self {
            PickerFor::Composer => "companion-model",
            PickerFor::Settings => "companion-default-model",
        }
    }
}

/// A choice in the list: a model, or (`value` None) OpenCode's own default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub value: Option<String>,
    pub name: String,
    pub provider: String,
}

/// The choices matching `query` (every word of it found in the model's name, provider or id),
/// in the list's order. With
/// `with_default`, "OpenCode’s default" leads while the query is empty.
pub fn choices(state: &ModelState, query: &str, with_default: bool) -> Vec<Choice> {
    let query = query.trim();
    let mut out = Vec::new();
    if with_default && query.is_empty() {
        out.push(Choice {
            value: None,
            name: "OpenCode’s default".into(),
            provider: String::new(),
        });
    }
    // Grouped by provider in the order providers first appear, as the options are sorted.
    let mut providers: Vec<&str> = Vec::new();
    for o in &state.options {
        if !providers.contains(&o.provider_id.as_str()) {
            providers.push(&o.provider_id);
        }
    }
    for provider in providers {
        for o in state.options.iter().filter(|o| o.provider_id == provider) {
            let haystack = format!("{} {} {}", o.name, o.provider_name, o.value).to_lowercase();
            if query.to_lowercase().split_whitespace().all(|word| haystack.contains(word)) {
                out.push(Choice {
                    value: Some(o.value.clone()),
                    name: o.name.clone(),
                    provider: o.provider_name.clone(),
                });
            }
        }
    }
    out
}

/// What the picker shows as chosen.
fn current(s: &App, target: PickerFor) -> Option<String> {
    match target {
        PickerFor::Composer => s.companion.models.current.clone(),
        PickerFor::Settings => s.prefs.companion_model.clone(),
    }
}

fn with_default(target: PickerFor) -> bool {
    target == PickerFor::Settings
}

fn row_key(i: usize) -> String {
    format!("companion-pick-{i}")
}

/// Opens `target`'s picker with an empty search and the current model highlighted.
pub fn open(s: &mut App, target: PickerFor, tx: &mut Tx) {
    let current = current(s, target);
    let list = choices(&s.companion.models, "", with_default(target));
    let c = &mut s.companion;
    c.picker = Some(target);
    c.picker_query = TextEdit::new("");
    c.picker_query.focused = true;
    c.picker_index = list.iter().position(|ch| ch.value == current).unwrap_or(0);
    tx.scroll_into_view(row_key(c.picker_index));
}

fn close(s: &mut App) {
    s.companion.picker = None;
}

fn choose(s: &mut App, value: Option<String>) {
    let Some(target) = s.companion.picker.take() else { return };
    match target {
        PickerFor::Composer => {
            if let Some(v) = value
                && s.companion.models.current.as_ref() != Some(&v)
            {
                s.companion.select_model(&v);
            }
        }
        PickerFor::Settings => service::set_default_model(value),
    }
}

fn key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let Some(target) = s.companion.picker else { return false };
    let list = choices(&s.companion.models, &s.companion.picker_query.text, with_default(target));
    let c = &mut s.companion;
    match ev.key {
        Key::Down if !list.is_empty() => {
            c.picker_index = (c.picker_index + 1).min(list.len() - 1);
            tx.scroll_into_view(row_key(c.picker_index));
            true
        }
        Key::Up if !list.is_empty() => {
            c.picker_index = c.picker_index.saturating_sub(1);
            tx.scroll_into_view(row_key(c.picker_index));
            true
        }
        Key::Enter => {
            if let Some(ch) = list.get(c.picker_index.min(list.len().saturating_sub(1))) {
                let value = ch.value.clone();
                choose(s, value);
            }
            true
        }
        Key::Escape => {
            close(s);
            true
        }
        _ => false,
    }
}

/// Pushes the open picker's scrim and popover into `layer`, when its trigger is on screen.
pub fn overlay(s: &App, target: PickerFor, p: &Palette, k: &rune_kit::Kit, layer: &mut Vec<El>) {
    if let Some(popover) = popover(s, target, p, k) {
        layer.push(rune_kit::scrim("companion-picker-scrim", close));
        layer.push(popover);
    }
}

/// The open picker's popover, anchored to its trigger; it closes on a press outside it.
pub fn popover(s: &App, target: PickerFor, p: &Palette, k: &rune_kit::Kit) -> Option<El> {
    if s.companion.picker != Some(target) {
        return None;
    }
    let shown = match target {
        PickerFor::Composer => s.companion.open,
        PickerFor::Settings => s.sheet == Some(Sheet::Settings),
    };
    if !shown {
        return None;
    }
    let c = &s.companion;
    let chosen = current(s, target);
    let list = choices(&c.models, &c.picker_query.text, with_default(target));
    let highlighted = c.picker_index.min(list.len().saturating_sub(1));

    let field = rune::text_input(&c.picker_query, theme::ui(12.5, 400.0))
        .key("companion-pick-search")
        .single_line()
        .w_full()
        .color(p.text)
        .placeholder("Search models", p.tertiary)
        .caret(p.text, p.selection)
        .label("Search models")
        .on_edit(|s: &mut App| &mut s.companion.picker_query)
        .on_input_change(|s: &mut App, _tx: &mut Tx| s.companion.picker_index = 0)
        .on_input_key(key);
    let search = row()
        .w_full()
        .h(28.0)
        .px(9.0)
        .gap(7.0)
        .center_y()
        .radius(7.0)
        .fill(p.hover)
        .child(icon(&icons::SEARCH, 12.0).color(p.tertiary))
        .child(col().grow(1.0).w(0.0).child(field));

    let body = if list.is_empty() {
        col()
            .w_full()
            .h(120.0)
            .center()
            .child(text("No matching models.").font(theme::ui(12.5, 400.0)).color(p.secondary))
    } else {
        let mut rows = col().w_full().gap(1.0).px(6.0).pb(6.0).role("listbox");
        let mut last_provider: Option<&str> = None;
        for (i, ch) in list.iter().enumerate() {
            if ch.value.is_some() && last_provider != Some(ch.provider.as_str()) {
                last_provider = Some(&ch.provider);
                rows = rows.child(
                    text(ch.provider.clone())
                        .font(theme::ui(11.0, 500.0))
                        .color(p.tertiary)
                        .px(8.0)
                        .pt(10.0)
                        .pb(3.0),
                );
            }
            let is_current = ch.value == chosen;
            let value = ch.value.clone();
            let mut r = row()
                .key(row_key(i))
                .w_full()
                .h(26.0)
                .px(8.0)
                .gap(8.0)
                .center_y()
                .radius(6.0)
                .hover(p.hover)
                .role("option")
                .checked(is_current)
                .label(ch.name.clone())
                .on_press(move |s: &mut App| choose(s, value.clone()))
                .child(
                    text(ch.name.clone())
                        .font(theme::ui(12.5, 400.0))
                        .color(p.text)
                        .truncate()
                        .grow(1.0)
                        .w(0.0),
                );
            if is_current {
                r = r.child(icon(&icons::CHECK, 12.0).color(p.accent));
            }
            if i == highlighted {
                r = r.fill(p.selected);
            }
            rows = rows.child(r);
            if ch.value.is_none() {
                // A rule under "OpenCode’s default", as the menu had.
                rows = rows.child(el().w_full().h(1.0).fill(p.line).mx(2.0).my(4.0));
            }
        }
        col().w_full().min_h(0.0).child(popover::fit_scroll(rows))
    };

    let surface = col()
        .key("companion-picker")
        .w(300.0)
        .max_h(380.0)
        .min_h(0.0)
        .radius(10.0)
        .fill(p.elevated)
        .border(1.0, p.line)
        .shadow(6.0, 24.0, p.shadow)
        .clip_rounded(10.0)
        .role("dialog")
        .label("Models")
        .modal(true, close)
        .child(col().w_full().padding(8.0).child(search))
        .child(el().w_full().h(1.0).fill(p.line))
        .child(body);
    let placement = match target {
        PickerFor::Composer => Placement::TopStart,
        PickerFor::Settings => Placement::BottomEnd,
    };
    Some(popover::anchored(k, surface, target.anchor(), placement, 6.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ModelState {
        let o = |provider: &str, provider_name: &str, id: &str, name: &str| ModelOption {
            value: format!("{provider}/{id}"),
            name: name.into(),
            provider_id: provider.into(),
            provider_name: provider_name.into(),
        };
        ModelState {
            options: vec![
                o("opencode", "OpenCode Zen", "sonnet", "Claude Sonnet 5.5"),
                o("opencode", "OpenCode Zen", "gpt", "GPT-6"),
                o("openrouter", "OpenRouter", "ling", "Ling"),
            ],
            current: None,
            stale: false,
            unavailable_reason: None,
            efforts: Vec::new(),
            effort: None,
        }
    }

    #[test]
    fn lists_every_model_by_provider_and_the_default_in_settings() {
        let all = choices(&state(), "", true);
        let names: Vec<&str> = all.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["OpenCode’s default", "Claude Sonnet 5.5", "GPT-6", "Ling"]);
        assert_eq!(all[0].value, None);
        assert_eq!(choices(&state(), "", false).len(), 3);
    }

    #[test]
    fn searches_names_and_providers() {
        let by_name = choices(&state(), "sonnet", true);
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].value.as_deref(), Some("opencode/sonnet"));
        let by_provider: Vec<String> =
            choices(&state(), "router", false).into_iter().filter_map(|c| c.value).collect();
        assert_eq!(by_provider, ["openrouter/ling"]);
        assert!(choices(&state(), "zzz", false).is_empty());
        // Letters scattered through a name don't match.
        assert!(choices(&state(), "cnt", false).is_empty());
        assert_eq!(choices(&state(), "zen gpt", false).len(), 1);
    }
}
