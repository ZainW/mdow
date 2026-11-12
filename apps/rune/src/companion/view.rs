//! The companion panel: setup states, the conversation and the composer. Ported from the Swift
//! app's `CompanionPanel`, `CompanionMessagesView`, `CompanionPartViews`, `CompanionComposer`
//! and `CompanionStyle`.

use super::icons as ci;
use super::logic::{self, DiffKind, InlineStyle, ReplyBlock, ReplyLine};
use super::picker::{self, PickerFor};
use super::types::*;
use crate::app::App;
use crate::icons;
use crate::theme::{self, GUTTER, Palette};
use rune::{
    Color, Cursor, Drag, El, Family, Icon, Key, KeyEvent, Link, LoopSpec, PopupItem, PopupMenu,
    ScrollSpec, Span, TextStyle, Tx, col, el, icon, rich, row, text,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Room the document card keeps when the panel is dragged wide.
const MIN_CARD: f32 = 360.0;
/// The widest a disclosure's row grows, inside the narrowest panel.
const ROW_MAX: f32 = super::MIN_WIDTH - 40.0;
/// Space between the panel's content and the document card (whose own gutter adds 8 points).
const LEADING: f32 = 4.0;
/// Space between the panel's content and the window's right edge, matching the card's gutter.
const TRAILING: f32 = GUTTER;
const INSTALL_COMMAND: &str = "curl -fsSL https://opencode.ai/install | bash";
const DOCS_URL: &str = "https://opencode.ai/docs";

// MARK: Style

/// Colours the companion adds to the window's palette.
struct Tone {
    /// Text and icons on an accent fill: white on the light theme's clay, ink on the dark one's.
    on_accent: Color,
    /// Additions in diffs and the "Applied" state.
    added: Color,
    /// Removals in diffs, errors and the "Failed" state.
    removed: Color,
}

fn hex(v: u32) -> Color {
    rune::color::hex(&format!("#{v:06x}")).unwrap_or(Color::BLACK)
}

fn tone(p: &Palette) -> Tone {
    if p.dark {
        Tone { on_accent: hex(0x1B1B1B), added: hex(0x74C48F), removed: hex(0xEC7A70) }
    } else {
        Tone { on_accent: Color::WHITE, added: hex(0x2F7D4F), removed: hex(0xB4413A) }
    }
}

/// Charter, for the assistant's prose and the empty state's greeting.
fn serif(size: f32, weight: f32) -> TextStyle {
    TextStyle { family: Family::Named("Charter"), ..TextStyle::new(size, weight) }
}

#[derive(Clone, Copy, PartialEq)]
enum Pill {
    /// Filled with the accent.
    Primary,
    /// A hairline outline.
    Secondary,
    /// Text that gains a soft fill on hover.
    Quiet,
}

/// A small pill button.
fn pill(
    key: impl Into<String>,
    glyph: Option<&'static Icon>,
    label: &str,
    kind: Pill,
    p: &Palette,
) -> El {
    let t = tone(p);
    let fg = match kind {
        Pill::Primary => t.on_accent,
        Pill::Secondary => p.text,
        Pill::Quiet => p.secondary,
    };
    let mut b = row()
        .key(key.into())
        .h(24.0)
        .px(10.0)
        .gap(4.0)
        .center_y()
        .radius(12.0)
        .role("button")
        .label(label.to_string())
        .focusable(true);
    if let Some(g) = glyph {
        b = b.child(icon(g, 11.0).color(fg));
    }
    b = b.child(text(label.to_string()).font(theme::ui(12.0, 500.0)).color(fg));
    match kind {
        Pill::Primary => b.fill(p.accent).hover(Color::WHITE.alpha(0.12)),
        Pill::Secondary => b.border(1.0, p.strong_line).hover(p.hover),
        Pill::Quiet => b.hover(p.hover),
    }
}

/// A square icon button with a soft rounded fill on hover, like the window's chrome buttons.
fn icon_button(
    key: impl Into<String>,
    glyph: &'static Icon,
    label: &str,
    side: f32,
    size: f32,
    p: &Palette,
    enabled: bool,
) -> El {
    let color = if enabled { p.secondary } else { p.tertiary.alpha(0.6) };
    let b = row()
        .key(key.into())
        .size(side, side)
        .center()
        .radius(7.0)
        .role("button")
        .label(label.to_string())
        .help(label.to_string())
        .child(icon(glyph, size).color(color));
    if enabled { b.hover(p.hover).focusable(true) } else { b.disabled(true) }
}

/// A flat card: paper fill and a hairline border.
fn card(radius: f32, fill: Color, border: Color) -> El {
    col().radius(radius).fill(fill).border(1.0, border)
}

/// Three dots that fade in turn (still under Reduce Motion).
fn activity_dots(p: &Palette) -> El {
    let mut r = row().gap(3.0).center_y().inert();
    for i in 0..3 {
        r = r.child(el().size(4.0, 4.0).radius(2.0).fill(p.accent).looping(LoopSpec::opacity(
            0.72,
            i as f64 * 0.13,
            0.25,
            1.0,
        )));
    }
    r
}

fn hairline(p: &Palette) -> El {
    el().w_full().h(1.0).fill(p.line)
}

/// A compact row that expands to show more: a label, a chevron, and the content below it. Used
/// for reasoning, tool calls and sources.
fn disclosure(
    id: String,
    expanded: bool,
    label: El,
    content: impl FnOnce() -> El,
    p: &Palette,
) -> El {
    let toggle = id.clone();
    let header = row()
        .key(format!("dis-{id}"))
        .self_start()
        .max_w(ROW_MAX)
        .min_w(0.0)
        .h(24.0)
        .px(6.0)
        .ml(-6.0)
        .gap(6.0)
        .center_y()
        .radius(6.0)
        .hover(p.hover)
        .role("button")
        .focusable(true)
        .checked(expanded)
        .on_press(move |s: &mut App| s.companion.toggle_expanded(&toggle))
        .child(label)
        .child(icon(&icons::CHEVRON_RIGHT, 10.0).color(p.tertiary).rotate(if expanded {
            std::f32::consts::FRAC_PI_2
        } else {
            0.0
        }));
    let mut c = col().w_full().gap(4.0).child(header);
    if expanded {
        c = c.child(content().transition(rune::Transition::opacity()));
    }
    c
}

// MARK: Panel

/// The width the panel draws at: the dragged width, clamped to Swift's range and to the room
/// the window leaves the document.
fn shown_width(s: &App) -> f32 {
    let mut width = s.companion.width.clamp(super::MIN_WIDTH, super::MAX_WIDTH);
    if let Some([body, _]) = rune::memo::layout_size("body") {
        let sidebar = if s.prefs.sidebar_open { s.prefs.sidebar_width } else { 0.0 };
        width = width.min((body - sidebar - MIN_CARD).max(super::MIN_WIDTH));
    }
    width.round()
}

/// The panel's leading edge: drag to resize, double-click for the default width.
fn resize_handle(width: f32) -> El {
    el().key("companion-resize")
        .absolute(-5.0, 0.0)
        .w(10.0)
        .h_full()
        .cursor(Cursor::ResizeColumn)
        .role("separator")
        .label("Companion width")
        .on_drag(move |s: &mut App, d: &mut Drag, _tx: &mut Tx| {
            if d.phase == rune::Phase::Down {
                d.data = width;
            }
            let dragged = (d.data - d.dx).clamp(super::MIN_WIDTH, super::MAX_WIDTH);
            s.companion.width = dragged;
            if d.phase == rune::Phase::Up {
                super::save_width(dragged);
            }
        })
        .on_double_press(|s: &mut App| {
            s.companion.width = super::DEFAULT_WIDTH;
            super::save_width(super::DEFAULT_WIDTH);
        })
}

pub fn panel(s: &App, p: &Palette, k: &rune_kit::Kit) -> El {
    let c = &s.companion;
    let ready = c.is_available();
    let no_models = ready && !c.models.stale && c.models.options.is_empty();
    let width = shown_width(s);
    let mut root = col()
        .key("companion")
        .w(width)
        .h_full()
        .min_h(0.0)
        .fill(p.chrome)
        .role("complementary")
        .label("Companion")
        .on_key(panel_key)
        .transition(rune::Transition::opacity())
        .child(header(s, p));
    if !ready {
        root = root.child(setup(c.status.as_ref(), p));
    } else if no_models && c.conversation.messages.is_empty() {
        let message = c
            .models
            .unavailable_reason
            .clone()
            .unwrap_or_else(|| "OpenCode has no models available yet.".into());
        root = root.child(setup_message(
            "Connect a Model Provider",
            &message,
            &["opencode auth login"],
            false,
            p,
        ));
    } else {
        root = root.child(messages(s, p, k));
        if let Some(change) = c.conversation.pending_change() {
            root = root.child(review_bar(s, change, p));
        }
        root = root.child(composer(s, p, k));
    }
    root = root.child(resize_handle(width));
    // The model picker draws in the panel's own tree: it comes after the card in the window,
    // so it sits above it, and nothing here clips it.
    if let Some(popover) = picker::popover(s, PickerFor::Composer, p, k) {
        root = root.child(popover);
    }
    root
}

/// ⌘↩ accepts the pending edit and ⌘⌫ declines it (unless the composer has the keys); ⌘.
/// stops the response.
pub fn panel_key(s: &mut App, ev: &KeyEvent, _tx: &mut Tx) -> bool {
    if !ev.mods.cmd {
        return false;
    }
    let pending = s.companion.conversation.pending_change().and_then(|c| c.permission_id.clone());
    match (&ev.key, pending) {
        (Key::Enter, Some(id)) => {
            s.companion.review(&id, Decision::Approve);
            true
        }
        (Key::Backspace, Some(id)) if !s.companion.composer.focused => {
            s.companion.review(&id, Decision::Reject);
            true
        }
        (Key::Command('.'), _) if s.companion.conversation.streaming => {
            s.companion.stop();
            true
        }
        _ => false,
    }
}

fn header(s: &App, p: &Palette) -> El {
    let c = &s.companion;
    let mut r = row()
        .key("companion-header")
        .w_full()
        .h(40.0)
        .gap(8.0)
        .center_y()
        .pl(LEADING + 2.0)
        .pr(TRAILING)
        .child(text("Companion").font(theme::ui(13.0, 600.0)).color(p.text));
    if let Some(v) = c
        .status
        .as_ref()
        .filter(|s| s.availability == Availability::Available)
        .and_then(|s| s.version.clone())
    {
        r = r.child(
            text(format!("OpenCode {v}"))
                .font(theme::mono(11.0, 400.0))
                .color(p.tertiary)
                .truncate()
                .pt(1.0),
        );
    }
    r.child(el().grow(1.0)).child(
        icon_button(
            "companion-new-chat",
            &ci::NEW_CHAT,
            "New Chat",
            28.0,
            15.0,
            p,
            !c.conversation.messages.is_empty(),
        )
        .on_press(|s: &mut App| s.companion.new_chat()),
    )
}

// MARK: Setup

fn setup(status: Option<&RuntimeStatus>, p: &Palette) -> El {
    let Some(status) = status else {
        return row()
            .key("companion-starting")
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .center()
            .gap(8.0)
            .child(activity_dots(p))
            .child(text("Starting OpenCode…").font(theme::ui(12.5, 400.0)).color(p.secondary));
    };
    match status.availability {
        Availability::Missing => setup_message(
            "Connect OpenCode",
            "The companion runs on OpenCode, using the models and subscriptions you already use there. Install OpenCode 2, then sign in to a provider:",
            &[INSTALL_COMMAND, "opencode auth login"],
            true,
            p,
        ),
        Availability::Outdated => setup_message(
            "Update OpenCode",
            &format!(
                "OpenCode {} is installed, but the companion needs OpenCode 2 or newer.",
                status.version.as_deref().unwrap_or("")
            ),
            &["opencode upgrade"],
            true,
            p,
        ),
        Availability::Failed | Availability::Available => setup_message(
            "OpenCode Didn’t Start",
            status.detail.as_deref().unwrap_or("Something went wrong while starting OpenCode."),
            &[],
            true,
            p,
        ),
    }
}

fn setup_message(title: &str, message: &str, commands: &[&str], docs: bool, p: &Palette) -> El {
    let mut c = col()
        .w_full()
        .gap(10.0)
        .pl(LEADING + 2.0)
        .pr(TRAILING + 2.0)
        .py(16.0)
        .child(text(title.to_string()).font(serif(19.0, 400.0)).color(p.text).wrap().pb(2.0))
        .child(
            text(message.to_string())
                .font(theme::ui(12.5, 400.0).lh(18.0))
                .color(p.secondary)
                .wrap()
                .selectable(true),
        );
    for (i, command) in commands.iter().enumerate() {
        c = c.child(command_line(i, command, p));
    }
    let mut buttons = row().gap(6.0).pt(4.0).child(
        pill("companion-check-again", None, "Check Again", Pill::Secondary, p)
            .on_press(|s: &mut App| s.companion.connect()),
    );
    if docs {
        buttons = buttons.child(
            pill("companion-docs", None, "OpenCode Docs", Pill::Quiet, p).on_press_tx(
                |_s: &mut App, tx: &mut Tx| {
                    let _ = tx.open_url(DOCS_URL);
                },
            ),
        );
    }
    col()
        .key("companion-setup")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(c.child(buttons))
}

fn command_line(i: usize, command: &str, p: &Palette) -> El {
    let copy = command.to_string();
    card(8.0, p.paper, p.line).key(format!("companion-command-{i}")).w_full().child(
        row()
            .w_full()
            .gap(6.0)
            .center_y()
            .pl(10.0)
            .pr(4.0)
            .py(4.0)
            .child(text("$").font(theme::mono(11.5, 400.0)).color(p.tertiary))
            .child(
                text(command.to_string())
                    .font(theme::mono(11.5, 400.0))
                    .color(p.text)
                    .wrap_anywhere()
                    .selectable(true)
                    .selection_color(p.selection)
                    .grow(1.0)
                    .w(0.0),
            )
            .child(
                icon_button(
                    format!("companion-copy-command-{i}"),
                    &icons::COPY,
                    "Copy",
                    22.0,
                    13.0,
                    p,
                    true,
                )
                .on_press_tx(move |_s: &mut App, tx: &mut Tx| tx.copy(copy.clone())),
            ),
    )
}

// MARK: Messages

fn messages(s: &App, p: &Palette, k: &rune_kit::Kit) -> El {
    let conversation = &s.companion.conversation;
    if conversation.messages.is_empty() {
        return empty_state(s, p);
    }
    let mut list = col()
        .key("companion-list")
        .w_full()
        .gap(18.0)
        .pl(LEADING + 2.0)
        .pr(TRAILING + 2.0)
        .pt(6.0)
        .pb(16.0);
    for m in &conversation.messages {
        list = list.child(match m.role {
            Role::User => user_message(m, p),
            Role::Assistant => assistant_message(s, m, p, k),
        });
    }
    if conversation.messages.last().is_some_and(|m| m.status == Some(MessageStatus::Error))
        && let Some(last_user) = conversation.messages.iter().rev().find(|m| m.role == Role::User)
    {
        list = list.child(turn_error(conversation.error.as_deref(), last_user.content.clone(), p));
    }
    for (i, warning) in conversation.warnings.iter().enumerate() {
        list = list.child(
            row()
                .key(format!("companion-warning-{i}"))
                .w_full()
                .gap(6.0)
                .items_start()
                .child(icon(&ci::WARNING, 12.0).color(p.tertiary).mt(1.0))
                .child(
                    text(warning.clone())
                        .font(theme::ui(11.5, 400.0))
                        .color(p.tertiary)
                        .wrap()
                        .grow(1.0)
                        .w(0.0),
                ),
        );
    }
    // Short conversations start at the top; long ones open at, and follow, the newest reply.
    col()
        .key("companion-scroll")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: true })
        .child(list)
}

struct Suggestion {
    glyph: &'static Icon,
    label: &'static str,
    prompt: String,
}

fn suggestions(document: Option<&str>) -> Vec<Suggestion> {
    match document {
        Some(name) => vec![
            Suggestion {
                glyph: &ci::LIST,
                label: "Summarize this document",
                prompt: format!("Summarize {name} in a few bullet points."),
            },
            Suggestion {
                glyph: &ci::BULB,
                label: "Explain the key ideas",
                prompt: format!("Explain the key ideas in {name} as if I'm new to the topic."),
            },
            Suggestion {
                glyph: &ci::TEXT_CHECK,
                label: "Fix spelling and grammar",
                prompt: format!(
                    "Fix spelling and grammar mistakes in {name}. Don't change the meaning or style."
                ),
            },
            Suggestion {
                glyph: &ci::SCISSORS,
                label: "Make it more concise",
                prompt: format!(
                    "Tighten the writing in {name}: cut repetition and filler, keep every point."
                ),
            },
        ],
        None => vec![
            Suggestion {
                glyph: &icons::FOLDER,
                label: "What’s in this folder?",
                prompt: "Give me a quick tour of the documents in this folder.".into(),
            },
            Suggestion {
                glyph: &ci::QUESTION,
                label: "Find open questions",
                prompt: "Find open questions and TODOs across these documents.".into(),
            },
        ],
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn empty_state(s: &App, p: &Palette) -> El {
    let (doc, folder) = super::context(s);
    let doc_name = doc.as_deref().map(file_name);
    let scope = doc_name.clone().or_else(|| folder.as_deref().map(file_name));
    let mut grid = el().w_full().equal_columns(2).gaps(8.0, 8.0).pt(20.0);
    for (i, sug) in suggestions(doc_name.as_deref()).into_iter().enumerate() {
        let prompt = sug.prompt.clone();
        grid = grid.child(
            col()
                .key(format!("companion-suggestion-{i}"))
                .min_h(68.0)
                .gap(8.0)
                .px(11.0)
                .py(10.0)
                .radius(10.0)
                .fill(p.paper)
                .border(1.0, p.line)
                .hover(p.hover)
                .hover_group()
                .role("button")
                .label(sug.label)
                .focusable(true)
                .on_press(move |s: &mut App| {
                    let (doc, folder) = super::context(s);
                    s.companion.send(&prompt, doc.as_deref(), folder.as_deref());
                })
                .child(icon(sug.glyph, 14.0).color(p.tertiary).group_color(p.accent))
                .child(text(sug.label).font(theme::ui(12.5, 500.0)).color(p.text).wrap()),
        );
    }
    let content = col()
        .w_full()
        .pl(LEADING + 2.0)
        .pr(TRAILING + 2.0)
        .py(20.0)
        .child(icon(&icons::SPARKLE, 19.0).color(p.accent).mb(12.0))
        .child(
            text(scope.map(|s| format!("Ask about {s}")).unwrap_or_else(|| "Ask about your documents".into()))
                .font(serif(22.0, 400.0))
                .color(p.text)
                .wrap(),
        )
        .child(
            text("Get answers, summaries and edits. Every change comes back as a diff for you to approve.")
                .font(theme::ui(12.5, 400.0).lh(18.0))
                .color(p.secondary)
                .wrap()
                .pt(6.0),
        )
        .child(grid);
    col().key("companion-empty").w_full().grow(1.0).min_h(0.0).justify_center().child(content)
}

fn user_message(m: &Message, p: &Palette) -> El {
    row().key(format!("companion-m-{}", m.id)).w_full().justify_end().pl(36.0).child(
        card(14.0, p.elevated, p.line).shrink().min_w(0.0).px(12.0).py(8.0).child(
            text(m.content.clone())
                .font(theme::ui(13.0, 400.0).lh(18.5))
                .color(p.text)
                .wrap()
                .selectable(true)
                .selection_color(p.selection),
        ),
    )
}

/// Nothing is streaming in right now, so show that OpenCode is still working.
fn is_waiting(m: &Message) -> bool {
    match m.parts.last() {
        None => true,
        Some(Part::Tool(t)) => t.state == ToolState::Completed,
        Some(Part::Thinking { done, .. }) => *done,
        Some(Part::Change(c)) => c.status == ChangeStatus::Applied,
        _ => false,
    }
}

fn assistant_message(s: &App, m: &Message, p: &Palette, k: &rune_kit::Kit) -> El {
    let c = &s.companion;
    let streaming = m.status == Some(MessageStatus::Streaming);
    let mut out = col()
        .key(format!("companion-m-{}", m.id))
        .w_full()
        .gap(10.0)
        .hover_group()
        .selection_group(true);
    for (i, part) in m.parts.iter().enumerate() {
        let id = format!("{}/{i}", m.id);
        out = out.child(match part {
            Part::Text(t) => reply_text(t, p),
            Part::Thinking { text: t, done } => {
                reasoning(id.clone(), t, !done && streaming, c.expanded.contains(&id), p)
            }
            Part::Tool(tool) => tool_call(id.clone(), tool, c.expanded.contains(&id), p, k),
            Part::Change(change) => change_card(s, change, p),
            Part::Status(t) => text(t.clone()).font(theme::ui(11.5, 400.0)).color(p.tertiary),
        });
    }
    if streaming && is_waiting(m) {
        out = out.child(
            row().h(20.0).gap(8.0).center_y().child(activity_dots(p)).child(
                text(if m.parts.is_empty() { "Thinking…" } else { "Working…" })
                    .font(theme::ui(12.0, 400.0))
                    .color(p.secondary),
            ),
        );
    }
    if m.status == Some(MessageStatus::Cancelled) {
        out = out.child(text("Stopped").font(theme::ui(11.5, 400.0)).color(p.tertiary));
    }
    if !m.citations.is_empty() {
        let id = format!("{}/sources", m.id);
        out = out.child(sources(id.clone(), &m.citations, c.expanded.contains(&id), p));
    }
    if m.status == Some(MessageStatus::Complete) && !m.content.trim().is_empty() {
        let copied = c.copied.as_ref().is_some_and(|(id, _)| *id == m.id);
        let content = m.content.clone();
        let id = m.id.clone();
        let label = if copied { "Copied" } else { "Copy Response" };
        out = out.child(
            icon_button(
                format!("companion-copy-{}", m.id),
                if copied { &icons::CHECK } else { &icons::COPY },
                label,
                24.0,
                13.0,
                p,
                true,
            )
            .ml(-5.0)
            .mt(-4.0)
            .group_opacity(if copied { 1.0 } else { 0.0 }, 1.0)
            .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                tx.copy(content.clone());
                s.companion.copied = Some((id.clone(), Instant::now()));
                tx.wake_after(Duration::from_millis(1500));
            }),
        );
    }
    out
}

fn turn_error(message: Option<&str>, retry: String, p: &Palette) -> El {
    let t = tone(p);
    card(10.0, t.removed.alpha(0.07), t.removed.alpha(0.25)).key("companion-error").w_full().child(
        row()
            .w_full()
            .gap(8.0)
            .items_start()
            .px(12.0)
            .py(10.0)
            .child(icon(&ci::ERROR, 14.0).color(t.removed).mt(1.0))
            .child(
                text(message.unwrap_or("Something went wrong.").to_string())
                    .font(theme::ui(12.5, 400.0).lh(18.0))
                    .color(p.text)
                    .wrap()
                    .selectable(true)
                    .grow(1.0)
                    .w(0.0),
            )
            .child(pill("companion-retry", None, "Retry", Pill::Secondary, p).on_press(
                move |s: &mut App| {
                    let (doc, folder) = super::context(s);
                    s.companion.send(&retry, doc.as_deref(), folder.as_deref());
                },
            )),
    )
}

fn sources(id: String, citations: &[Citation], expanded: bool, p: &Palette) -> El {
    let label = text(if citations.len() == 1 {
        "Used 1 source".to_string()
    } else {
        format!("Used {} sources", citations.len())
    })
    .font(theme::ui(11.5, 400.0))
    .color(p.tertiary);
    disclosure(
        id,
        expanded,
        label,
        || {
            let mut c = col().gap(2.0);
            for (i, citation) in citations.iter().enumerate() {
                let path = PathBuf::from(&citation.path);
                c = c.child(
                    row()
                        .key(format!("companion-source-{i}-{}", citation.source_id))
                        .gap(6.0)
                        .center_y()
                        .hover_group()
                        .role("link")
                        .label(citation.label.clone())
                        .help(citation.path.clone())
                        .focusable(true)
                        .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                            s.open_paths(std::slice::from_ref(&path), tx)
                        })
                        .child(icon(&icons::DOC, 12.0).color(p.tertiary))
                        .child(
                            text(citation.label.clone())
                                .font(theme::mono(11.0, 400.0))
                                .color(p.secondary)
                                .group_color(p.accent)
                                .truncate_middle(),
                        ),
                );
            }
            c
        },
        p,
    )
}

// MARK: Reply text

/// Assistant markdown: paragraphs and headings in Charter with inline styling, list items that
/// hang their wrapped lines, and code blocks in Geist Mono on a card.
fn reply_text(markdown: &str, p: &Palette) -> El {
    let body = serif(14.5, 400.0).lh(21.5);
    let mut out = col().w_full().gap(9.0);
    for block in logic::split_reply(markdown) {
        out = out.child(match block {
            ReplyBlock::Paragraph(paragraph) => {
                let mut c = col().w_full().gap(5.0);
                for line in logic::split_lines(&paragraph) {
                    c = c.child(match line {
                        ReplyLine::Text(t) => inline_text(&t, body, p).w_full(),
                        ReplyLine::Item { marker, depth, text: t } => row()
                            .w_full()
                            .items_start()
                            .pl(depth as f32 * 16.0)
                            .child(text(marker).font(body).color(p.secondary).w(18.0))
                            .child(inline_text(&t, body, p).grow(1.0).w(0.0)),
                    });
                }
                c
            }
            ReplyBlock::Heading { level, text: t } => {
                let style = if level <= 2 { serif(16.0, 600.0) } else { theme::ui(13.0, 600.0) };
                inline_text(&t, style, p).w_full().pt(4.0)
            }
            ReplyBlock::Code { text: code, .. } => {
                card(8.0, p.paper, p.line).w_full().clip_rounded(8.0).child(
                    col().w_full().scroll_x().child(
                        text(code)
                            .font(theme::mono(11.5, 400.0).lh(16.0))
                            .color(p.text)
                            .selectable(true)
                            .selection_color(p.selection)
                            .px(11.0)
                            .py(9.0),
                    ),
                )
            }
        });
    }
    out
}

/// A line of inline markdown as rich text: strong, emphasis, code in the accent, links that
/// open in the browser.
fn inline_text(source: &str, style: TextStyle, p: &Palette) -> El {
    let parsed = logic::inline(source);
    let mut spans: Vec<Span> = Vec::new();
    for (range, s) in &parsed.styles {
        let span = Span::new(range.start, range.end);
        spans.push(match s {
            InlineStyle::Strong => span.bold(),
            InlineStyle::Emphasis => span.italic(),
            // Geist Mono a touch smaller, so it sits level with the serif.
            InlineStyle::Code => span.mono().size(style.size - 2.0).color(p.accent),
            InlineStyle::Strike => span.color(p.tertiary),
        });
    }
    for (range, _) in &parsed.links {
        spans
            .push(Span::new(range.start, range.end).color(p.accent).underline(p.accent.alpha(0.4)));
    }
    spans.sort_by_key(|s| (s.start, s.end));
    let mut t = rich(parsed.text, spans)
        .font(style)
        .color(p.text)
        .selectable(true)
        .selection_color(p.selection);
    if !parsed.links.is_empty() {
        for (range, url) in parsed.links {
            t = t.link(Link::new(range, url));
        }
        t = t.on_link(|_s: &mut App, url: &str, tx: &mut Tx| {
            let _ = tx.open_url(url);
        });
    }
    t
}

// MARK: Reasoning and tools

fn reasoning(id: String, body: &str, active: bool, expanded: bool, p: &Palette) -> El {
    let mut label =
        row().gap(7.0).center_y().child(icon(&ci::THOUGHT, 13.0).color(p.tertiary)).child(
            text(if active { "Thinking…" } else { "Thought" })
                .font(theme::ui(12.0, 400.0))
                .color(p.secondary),
        );
    if active {
        label = label.child(activity_dots(p));
    }
    let body = body.trim().to_string();
    disclosure(
        id,
        expanded,
        label,
        move || {
            row().w_full().pl(6.0).child(
                text(body)
                    .font(theme::ui(12.5, 400.0).lh(18.0))
                    .color(p.secondary)
                    .wrap()
                    .selectable(true)
                    .selection_color(p.selection)
                    .pl(10.0)
                    .border_sides([0.0, 0.0, 0.0, 1.5], p.strong_line)
                    .grow(1.0)
                    .w(0.0),
            )
        },
        p,
    )
}

fn tool_call(id: String, tool: &ToolPart, expanded: bool, p: &Palette, k: &rune_kit::Kit) -> El {
    let t = tone(p);
    let state = match tool.state {
        ToolState::Pending | ToolState::Running => {
            rune_kit::Spinner::new().size(12.0).color(p.secondary).build(k)
        }
        ToolState::Completed => icon(&icons::CHECK, 12.0).color(p.tertiary),
        ToolState::Error => icon(&ci::BANG, 12.0).color(t.removed),
        ToolState::Cancelled => icon(&ci::MINUS, 12.0).color(p.tertiary),
    };
    let label = row()
        .gap(7.0)
        .center_y()
        .shrink()
        .min_w(0.0)
        .help(tool.name.clone())
        .child(row().w(14.0).center().child(state))
        .child(
            text(tool.name.clone())
                .font(theme::mono(11.5, 400.0))
                .color(if tool.state == ToolState::Error { t.removed } else { p.secondary })
                .truncate_middle(),
        );
    disclosure(
        id,
        expanded,
        label,
        || {
            let mut c = col().w_full().gap(6.0);
            if let Some(input) = tool.input.as_deref().filter(|i| !i.is_empty()) {
                c = c.child(detail_box("Input", input, false, p));
            }
            if let Some(error) = &tool.error {
                c = c.child(detail_box("Error", error, true, p));
            } else if let Some(output) = tool.output.as_deref().filter(|o| !o.is_empty()) {
                c = c.child(detail_box("Output", output, false, p));
            }
            c
        },
        p,
    )
}

fn detail_box(title: &str, body: &str, error: bool, p: &Palette) -> El {
    let t = tone(p);
    card(8.0, p.well, p.line)
        .w_full()
        .clip_rounded(8.0)
        .child(
            text(title.to_lowercase())
                .font(theme::mono(10.0, 500.0))
                .color(p.tertiary)
                .px(9.0)
                .pt(6.0),
        )
        .child(
            col().w_full().max_h(160.0).scroll_xy().child(
                text(body.to_string())
                    .font(theme::mono(11.0, 400.0).lh(15.0))
                    .color(if error { t.removed } else { p.text })
                    .selectable(true)
                    .selection_color(p.selection)
                    .px(9.0)
                    .pt(3.0)
                    .pb(7.0),
            ),
        )
}

// MARK: Change cards

fn change_card(s: &App, change: &ChangePart, p: &Palette) -> El {
    let t = tone(p);
    let settled = matches!(change.status, ChangeStatus::Rejected | ChangeStatus::Failed);
    let additions: i64 = change.files.iter().map(|f| f.additions).sum();
    let deletions: i64 = change.files.iter().map(|f| f.deletions).sum();
    let title = match change.files.len() {
        0 => "Proposed change".to_string(),
        1 => change.files[0].display_path.clone(),
        n => format!("{n} documents"),
    };
    let added =
        !change.files.is_empty() && change.files.iter().all(|f| f.status == FileStatus::Added);

    let mut head = row()
        .w_full()
        .h(34.0)
        .px(10.0)
        .gap(7.0)
        .center_y()
        .child(icon(if added { &ci::DOC_PLUS } else { &ci::PENCIL }, 13.0).color(p.tertiary))
        .child(
            text(title.clone())
                .font(theme::mono(11.5, 500.0))
                .color(p.text)
                .truncate_middle()
                .help(title.clone())
                .grow(1.0)
                .w(0.0),
        );
    if additions > 0 || deletions > 0 {
        head = head.child(
            row()
                .gap(5.0)
                .child(text(format!("+{additions}")).font(theme::mono(11.0, 400.0)).color(t.added))
                .child(
                    text(format!("−{deletions}")).font(theme::mono(11.0, 400.0)).color(t.removed),
                ),
        );
    }
    head = head.child(status_pill(change, p, &t));

    let mut c = col()
        .key(format!("companion-change-{}", change.tool_call_id))
        .w_full()
        .radius(10.0)
        .fill(p.paper)
        .border(1.0, if change.awaiting_review() { p.accent.alpha(0.55) } else { p.line })
        .clip_rounded(10.0)
        .opacity(if settled { 0.7 } else { 1.0 })
        .role("group")
        .label(format!("Change to {title}"))
        .child(head);
    for file in &change.files {
        c = c.child(hairline(p));
        if change.files.len() > 1 {
            c = c.child(
                text(file.display_path.clone())
                    .font(theme::mono(10.5, 400.0))
                    .color(p.tertiary)
                    .truncate_middle()
                    .px(10.0)
                    .py(4.0),
            );
        }
        let key = format!("{}:{}", change.tool_call_id, file.path);
        c = c.child(diff(s, &key, file, settled, p, &t));
    }
    if let Some(error) = change.error.as_ref().filter(|_| change.status == ChangeStatus::Failed) {
        c = c.child(hairline(p)).child(
            text(error.clone()).font(theme::ui(11.5, 400.0)).color(t.removed).wrap().padding(10.0),
        );
    }
    if change.awaiting_review()
        || (change.status == ChangeStatus::Applied && !change.files.is_empty())
    {
        let mut actions = row().w_full().gap(6.0).center_y().px(8.0).py(7.0);
        if change.files.len() == 1 && change.files[0].status != FileStatus::Deleted {
            let path = PathBuf::from(&change.files[0].path);
            actions = actions.child(
                pill(
                    format!("companion-open-{}", change.tool_call_id),
                    None,
                    "Open Document",
                    Pill::Quiet,
                    p,
                )
                .ml(-4.0)
                .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                    s.open_paths(std::slice::from_ref(&path), tx)
                }),
            );
        }
        actions = actions.child(el().grow(1.0));
        if let Some(permission) = change.permission_id.clone().filter(|_| change.awaiting_review())
        {
            let decline = permission.clone();
            actions = actions
                .child(
                    pill(
                        format!("companion-decline-{}", change.tool_call_id),
                        None,
                        "Decline",
                        Pill::Secondary,
                        p,
                    )
                    .on_press(move |s: &mut App| s.companion.review(&decline, Decision::Reject)),
                )
                .child(
                    pill(
                        format!("companion-accept-{}", change.tool_call_id),
                        Some(&icons::CHECK),
                        "Accept",
                        Pill::Primary,
                        p,
                    )
                    .on_press(move |s: &mut App| {
                        s.companion.review(&permission, Decision::Approve)
                    }),
                );
        }
        c = c.child(hairline(p)).child(actions);
    }
    c
}

fn status_pill(change: &ChangePart, p: &Palette, t: &Tone) -> El {
    if change.applying() {
        return row()
            .gap(5.0)
            .center_y()
            .child(activity_dots(p))
            .child(text("Applying").font(theme::ui(11.0, 500.0)).color(p.secondary));
    }
    let (label, glyph, color): (&str, Option<&'static Icon>, Color) = match change.status {
        ChangeStatus::Pending => ("Review", None, p.accent),
        ChangeStatus::Applied => ("Applied", Some(&icons::CHECK), t.added),
        ChangeStatus::Rejected => ("Declined", Some(&ci::UNDO), p.secondary),
        ChangeStatus::Failed => ("Failed", Some(&ci::XMARK), t.removed),
    };
    let mut r = row().h(20.0).px(7.0).gap(4.0).center_y().radius(10.0).fill(color.alpha(0.12));
    r = match glyph {
        Some(g) => r.child(icon(g, 10.0).color(color)),
        None => r.child(el().size(5.0, 5.0).radius(2.5).fill(color)),
    };
    r.child(text(label).font(theme::ui(11.0, 500.0)).color(color))
}

/// Lines a long diff shows before "Show All".
const PREVIEW_LINES: usize = 14;

/// A unified diff, long ones cut to a preview so the conversation stays readable.
fn diff(
    s: &App,
    key: &str,
    file: &FileChange,
    collapsed_by_default: bool,
    p: &Palette,
    t: &Tone,
) -> El {
    let hunks = logic::parse_diff(&file.patch);
    if hunks.is_empty() {
        let note = if file.status == FileStatus::Deleted {
            "Deletes this document."
        } else {
            "No preview available."
        };
        return col()
            .w_full()
            .fill(p.well)
            .child(text(note).font(theme::ui(11.5, 400.0)).color(p.secondary).padding(10.0));
    }
    let total: usize = hunks.iter().map(|h| h.lines.len()).sum();
    let show_all = s.companion.diff_open.get(key).copied().unwrap_or(!collapsed_by_default)
        || total <= PREVIEW_LINES;
    let font = theme::mono(11.0, 400.0).lh(16.0);
    let mut c = col().w_full().fill(p.well).py(4.0).selection_group(true);
    let mut remaining = if show_all { usize::MAX } else { PREVIEW_LINES };
    for (hi, hunk) in hunks.iter().enumerate() {
        if remaining == 0 {
            break;
        }
        if hi > 0 {
            c = c.child(text("⋯").font(font).color(p.tertiary).pl(34.0).py(1.0));
        }
        for line in hunk.lines.iter().take(remaining) {
            let (marker, mark_color, fill) = match line.kind {
                DiffKind::Add => ("+", t.added, Some(t.added.alpha(0.11))),
                DiffKind::Remove => ("−", t.removed, Some(t.removed.alpha(0.10))),
                // An empty text has no line height.
                DiffKind::Context => (" ", p.tertiary, None),
            };
            let number = if line.kind == DiffKind::Remove { line.old_line } else { line.new_line };
            let mut r = row()
                .w_full()
                .items_start()
                .py(1.5)
                .child(
                    text(number.map(|n| n.to_string()).unwrap_or_default())
                        .font(font)
                        .color(p.tertiary)
                        .w(28.0)
                        .align(rune::Align::End)
                        .mr(6.0),
                )
                .child(text(marker).font(font).color(mark_color).w(12.0))
                .child(
                    text(if line.text.is_empty() { " ".to_string() } else { line.text.clone() })
                        .font(font)
                        .color(if line.kind == DiffKind::Context { p.secondary } else { p.text })
                        .wrap_anywhere()
                        .selectable(true)
                        .selection_color(p.selection)
                        .grow(1.0)
                        .w(0.0)
                        .pr(8.0),
                );
            if let Some(fill) = fill {
                r = r.fill(fill).border_sides([0.0, 0.0, 0.0, 2.0], mark_color);
            }
            c = c.child(r);
        }
        remaining = remaining.saturating_sub(hunk.lines.len());
    }
    if total > PREVIEW_LINES {
        let key = key.to_string();
        c = c.child(
            row()
                .key(format!("companion-diff-more-{key}"))
                .w_full()
                .h(24.0)
                .center()
                .gap(4.0)
                .hover_group()
                .role("button")
                .focusable(true)
                .on_press(move |s: &mut App| {
                    s.companion.diff_open.insert(key.clone(), !show_all);
                })
                .child(
                    icon(if show_all { &icons::CHEVRON_UP } else { &icons::CHEVRON_DOWN }, 10.0)
                        .color(p.secondary)
                        .group_color(p.text),
                )
                .child(
                    text(if show_all {
                        "Show Less".to_string()
                    } else {
                        format!("Show All {total} Lines")
                    })
                    .font(theme::ui(11.0, 500.0))
                    .color(p.secondary)
                    .group_color(p.text),
                ),
        );
    }
    c
}

// MARK: Pending review

/// Stays above the composer while OpenCode waits for a decision on an edit, wherever the change
/// card has scrolled to. ⌘↩ accepts and ⌘⌫ declines.
fn review_bar(s: &App, change: &ChangePart, p: &Palette) -> El {
    let first = change.files.first();
    let label = match change.files.len() {
        0 => "Suggested edit".to_string(),
        1 => first.unwrap().display_path.clone(),
        n => format!("{} and {} more", first.unwrap().display_path, n - 1),
    };
    let (doc, _) = super::context(s);
    let elsewhere = first
        .filter(|f| doc.as_deref() != Some(Path::new(&f.path)) && f.status != FileStatus::Deleted);
    let mut r = card(12.0, p.paper, p.accent.alpha(0.35)).key("companion-review").w_full().child(
        row()
            .w_full()
            .gap(8.0)
            .center_y()
            .pl(10.0)
            .pr(8.0)
            .py(8.0)
            .child(el().size(6.0, 6.0).radius(3.0).fill(p.accent).ml(2.0))
            .child(
                col()
                    .gap(1.0)
                    .grow(1.0)
                    .w(0.0)
                    .help(label.clone())
                    .child(text("Suggested edit").font(theme::ui(12.5, 500.0)).color(p.text))
                    .child(
                        text(label)
                            .font(theme::mono(11.0, 400.0))
                            .color(p.tertiary)
                            .truncate_middle(),
                    ),
            )
            .child({
                let mut buttons = row().gap(6.0).center_y();
                if let Some(f) = elsewhere {
                    let path = PathBuf::from(&f.path);
                    buttons = buttons.child(
                        pill("companion-review-show", None, "Show", Pill::Quiet, p)
                            .help(format!("Open {}", f.display_path))
                            .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                                s.open_paths(std::slice::from_ref(&path), tx)
                            }),
                    );
                }
                if let Some(permission) = change.permission_id.clone() {
                    let decline = permission.clone();
                    buttons = buttons
                        .child(
                            pill("companion-review-decline", None, "Decline", Pill::Secondary, p)
                                .help(crate::platform::label("Decline (⌘⌫)"))
                                .on_press(move |s: &mut App| {
                                    s.companion.review(&decline, Decision::Reject)
                                }),
                        )
                        .child(
                            pill("companion-review-accept", None, "Accept", Pill::Primary, p)
                                .help(crate::platform::label("Accept (⌘↩)"))
                                .on_press(move |s: &mut App| {
                                    s.companion.review(&permission, Decision::Approve)
                                }),
                        );
                }
                buttons
            }),
    );
    r = r.transition(rune::Transition::opacity());
    row().w_full().pl(LEADING).pr(TRAILING).pt(6.0).child(r)
}

// MARK: Composer

/// Documents the user can attach with @: the sidebar's folder.
fn folder_documents(s: &App) -> Vec<PathBuf> {
    let Some(scan) = &s.tab().scan else { return Vec::new() };
    let mut out = Vec::new();
    crate::files::flatten_files(&scan.tree, &mut out);
    out.retain(|p| crate::files::is_document(p));
    out
}

fn relative(path: &Path, root: Option<&Path>) -> String {
    match root.and_then(|r| path.strip_prefix(r).ok()) {
        Some(rel) => rel.to_string_lossy().into_owned(),
        None => file_name(path),
    }
}

fn mention_results(s: &App) -> Vec<PathBuf> {
    let Some(query) = &s.companion.mention else { return Vec::new() };
    let (doc, folder) = super::context(s);
    let candidates: Vec<PathBuf> =
        folder_documents(s).into_iter().filter(|p| Some(p) != doc.as_ref()).collect();
    if query.is_empty() {
        return candidates.into_iter().take(8).collect();
    }
    let mut scored: Vec<(f64, PathBuf)> = candidates
        .into_iter()
        .filter_map(|p| {
            let score = crate::files::fuzzy_score(query, &file_name(&p))
                .max(crate::files::fuzzy_score(query, &relative(&p, folder.as_deref())));
            (score > 0.0).then_some((score, p))
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(8).map(|(_, p)| p).collect()
}

/// The `@query` at the end of the message, if one is being typed.
fn mention_query(text: &str) -> Option<String> {
    let at = text.rfind('@')?;
    let query = &text[at + 1..];
    let starts = at == 0 || text[..at].chars().next_back().is_some_and(char::is_whitespace);
    (starts && !query.contains(char::is_whitespace)).then(|| query.to_string())
}

impl super::Companion {
    fn update_mention(&mut self) {
        let query = mention_query(&self.composer.text);
        if query != self.mention {
            self.mention_index = 0;
        }
        self.mention = query;
    }

    fn start_mention(&mut self) {
        let mut text = self.composer.text.clone();
        if !text.is_empty() && !text.ends_with(' ') {
            text.push(' ');
        }
        text.push('@');
        self.composer.replace_text(text);
        self.mention = Some(String::new());
        self.mention_index = 0;
        self.composer.focused = true;
    }

    fn pick_mention(&mut self, path: &Path) {
        self.conversation.add_tag(ContextTag::new(path.to_string_lossy()));
        let text = &self.composer.text;
        if let Some(at) = text.rfind('@') {
            let kept = text[..at].to_string();
            self.composer.replace_text(kept);
        }
        self.mention = None;
        self.composer.focused = true;
    }
}

fn submit(s: &mut App) {
    let text = s.companion.composer.text.clone();
    if text.trim().is_empty() || s.companion.conversation.streaming {
        return;
    }
    let (doc, folder) = super::context(s);
    s.companion.send(&text, doc.as_deref(), folder.as_deref());
    s.companion.composer.clear();
    s.companion.mention = None;
}

fn composer_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let results = if s.companion.mention.is_some() { mention_results(s) } else { Vec::new() };
    if !results.is_empty() {
        let c = &mut s.companion;
        match ev.key {
            Key::Down => {
                c.mention_index = (c.mention_index + 1) % results.len();
                return true;
            }
            Key::Up => {
                c.mention_index =
                    if c.mention_index == 0 { results.len() - 1 } else { c.mention_index - 1 };
                return true;
            }
            Key::Enter | Key::Tab if !ev.mods.shift && !ev.mods.alt => {
                let pick = results[c.mention_index.min(results.len() - 1)].clone();
                c.pick_mention(&pick);
                return true;
            }
            Key::Escape => {
                c.mention = None;
                return true;
            }
            _ => {}
        }
    }
    if ev.mods.cmd && matches!(ev.key, Key::Enter | Key::Command('.')) {
        return panel_key(s, ev, tx);
    }
    false
}

fn composer(s: &App, p: &Palette, k: &rune_kit::Kit) -> El {
    let c = &s.companion;
    let t = tone(p);
    let (doc, folder) = super::context(s);
    let streaming = c.conversation.streaming;
    let mut outer =
        col().key("companion-composer").w_full().pl(LEADING).pr(TRAILING).pb(GUTTER).pt(8.0);

    let results = mention_results(s);
    if c.mention.is_some() && !results.is_empty() {
        let mut list = col()
            .key("companion-mentions")
            .w_full()
            .gap(1.0)
            .padding(4.0)
            .radius(10.0)
            .fill(p.elevated)
            .border(1.0, p.line)
            .shadow(3.0, 20.0, p.shadow)
            .role("listbox")
            .mb(6.0);
        for (i, path) in results.iter().enumerate() {
            let parent = path.parent().map(Path::to_path_buf);
            let folder_label = match (parent.as_deref(), folder.as_deref()) {
                (Some(parent), Some(root)) if parent == root => String::new(),
                (Some(parent), root) => relative(parent, root),
                _ => String::new(),
            };
            let pick = path.clone();
            let mut r = row()
                .key(format!("companion-mention-{i}"))
                .w_full()
                .h(26.0)
                .px(8.0)
                .gap(7.0)
                .center_y()
                .radius(6.0)
                .hover(p.hover)
                .role("option")
                .on_press(move |s: &mut App| s.companion.pick_mention(&pick))
                .child(icon(&icons::DOC, 12.0).color(p.tertiary))
                .child(text(file_name(path)).font(theme::ui(12.5, 400.0)).color(p.text).truncate())
                .child(el().grow(1.0))
                .child(
                    text(folder_label)
                        .font(theme::mono(10.5, 400.0))
                        .color(p.tertiary)
                        .truncate_head(),
                );
            if i == c.mention_index {
                r = r.fill(p.selected);
            }
            list = list.child(r);
        }
        outer = outer.child(list);
    }

    let mut card_col = col()
        .w_full()
        .gap(8.0)
        .pt(10.0)
        .px(10.0)
        .pb(7.0)
        .radius(p.radius)
        .fill(p.paper)
        .border(1.0, if c.composer.focused { p.strong_line } else { p.line })
        .shadow(2.0, 12.0, Color::BLACK.alpha(0.05))
        // A press anywhere on the card puts the caret in the message field.
        .on_press(|s: &mut App| s.companion.composer.focused = true);

    if doc.is_some() || !c.conversation.tags.is_empty() {
        let mut chips = row().gap(4.0).center_y();
        if let Some(d) = &doc {
            chips = chips.child(
                chip("companion-chip-doc".into(), &icons::DOC, &file_name(d), true, None, p)
                    .help(format!("OpenCode sees that you are viewing {}", d.display())),
            );
        }
        for tag in &c.conversation.tags {
            let id = tag.source_id.clone();
            chips = chips.child(
                chip(
                    format!("companion-chip-{}", tag.source_id),
                    &ci::AT,
                    &logic_basename(&tag.path),
                    false,
                    Some(id),
                    p,
                )
                .help(tag.path.clone()),
            );
        }
        card_col = card_col.child(col().w_full().scroll_x().child(chips));
    }

    let placeholder = match &doc {
        Some(d) => format!("Ask about {}, or ask for an edit…", file_name(d)),
        None => "Ask about your documents…".to_string(),
    };
    let field = rune::text_input(&c.composer, theme::ui(13.0, 400.0).lh(18.0))
        .key("companion-input")
        .w_full()
        .color(p.text)
        .placeholder(placeholder, p.tertiary)
        .caret(p.text, p.selection)
        .label("Message the companion")
        .on_edit(|s: &mut App| &mut s.companion.composer)
        .on_submit(|s: &mut App, _tx: &mut Tx| submit(s))
        .on_input_change(|s: &mut App, _tx: &mut Tx| s.companion.update_mention())
        .on_input_key(composer_key);
    card_col = card_col.child(rune::auto_grow(field, 1, 8).px(2.0));

    let has_documents = s.tab().scan.as_ref().is_some_and(|sc| !sc.tree.is_empty());
    let mut bar = row()
        .w_full()
        .gap(2.0)
        .center_y()
        .ml(-4.0)
        .mr(-4.0)
        .child(
            icon_button(
                "companion-mention",
                &ci::AT,
                "Mention a Document",
                26.0,
                14.0,
                p,
                has_documents,
            )
            .on_press(|s: &mut App| s.companion.start_mention()),
        )
        .child(model_picker(c, p))
        .child(el().grow(1.0));
    if streaming {
        bar = bar.child(
            row()
                .key("companion-stop")
                .size(26.0, 26.0)
                .center()
                .radius(13.0)
                .fill(p.text)
                .role("button")
                .label("Stop")
                .help(crate::platform::label("Stop (⌘.)"))
                .focusable(true)
                .hover(p.paper.alpha(0.12))
                .on_press(|s: &mut App| s.companion.stop())
                .child(icon(&icons::STOP, 12.0).color(p.paper)),
        );
    } else {
        let can_send = !c.composer.text.trim().is_empty();
        let mut send = row()
            .key("companion-send")
            .size(26.0, 26.0)
            .center()
            .radius(13.0)
            .fill(if can_send { p.accent } else { p.hover })
            .role("button")
            .label("Send")
            .help("Send (↩)")
            .child(icon(&icons::SEND, 14.0).color(if can_send { t.on_accent } else { p.tertiary }));
        send = if can_send {
            send.focusable(true).hover(Color::WHITE.alpha(0.12)).on_press(|s: &mut App| submit(s))
        } else {
            send.disabled(true)
        };
        bar = bar.child(send);
    }
    card_col = card_col.child(bar);
    let _ = k;
    outer.child(card_col)
}

fn logic_basename(path: &str) -> String {
    super::events::basename(path)
}

/// A document the message refers to, as a small mono pill.
fn chip(
    key: String,
    glyph: &'static Icon,
    title: &str,
    secondary: bool,
    remove: Option<String>,
    p: &Palette,
) -> El {
    let mut c = row()
        .key(key.clone())
        .h(20.0)
        .max_w(180.0)
        .pl(7.0)
        .pr(if remove.is_some() { 3.0 } else { 8.0 })
        .gap(4.0)
        .center_y()
        .radius(10.0)
        .fill(p.hover)
        .border(1.0, p.line)
        .child(icon(glyph, 11.0).color(p.tertiary))
        .child(
            text(title.to_string())
                .font(theme::mono(11.0, 400.0))
                .color(if secondary { p.secondary } else { p.text })
                .truncate_middle(),
        );
    if let Some(id) = remove {
        c = c.child(
            row()
                .key(format!("{key}-remove"))
                .size(14.0, 14.0)
                .center()
                .radius(7.0)
                .hover(p.hover)
                .role("button")
                .label(format!("Remove {title}"))
                .focusable(true)
                .on_press(move |s: &mut App| s.companion.conversation.remove_tag(&id))
                .child(icon(&ci::XMARK, 8.0).color(p.tertiary)),
        );
    }
    c
}

/// The model button, which opens the searchable picker.
fn model_picker(c: &super::Companion, p: &Palette) -> El {
    let state = &c.models;
    let unavailable = state.stale || state.options.is_empty();
    let label = match state.current_option() {
        Some(o) => o.name.clone(),
        None if state.stale => "Loading models…".into(),
        None => "Choose a model".into(),
    };
    let help = if unavailable {
        state.unavailable_reason.clone().unwrap_or_default()
    } else {
        state
            .current_option()
            .map(|o| o.provider_name.clone())
            .unwrap_or_else(|| "Choose a model".into())
    };
    let b = row()
        .key("companion-model")
        .h(24.0)
        .px(8.0)
        .gap(4.0)
        .center_y()
        .shrink()
        .min_w(0.0)
        .radius(12.0)
        .role("combobox")
        .label(format!("Model: {label}"))
        .help(help)
        .opacity(if unavailable { 0.6 } else { 1.0 })
        .child(text(label).font(theme::mono(11.0, 400.0)).color(p.secondary).truncate())
        .child(icon(&icons::CHEVRON_DOWN, 10.0).color(p.tertiary));
    if unavailable {
        return b.disabled(true);
    }
    b.hover(p.hover)
        .focusable(true)
        .expanded(c.picker == Some(PickerFor::Composer))
        .on_press_tx(|s: &mut App, tx: &mut Tx| picker::open(s, PickerFor::Composer, tx))
}

/// The models as a native menu: flat for one provider, a submenu each for several. With
/// `default_item`, "OpenCode’s default" (id "") leads it.
pub(super) fn model_menu(state: &ModelState, default_item: bool) -> PopupMenu {
    let current =
        if default_item { crate::prefs::get().companion_model } else { state.current.clone() };
    let mut menu = PopupMenu::new();
    if default_item {
        menu = menu
            .item(PopupItem::new("", "OpenCode’s default").checked(current.is_none()))
            .separator();
    }
    let item = |o: &ModelOption| {
        PopupItem::new(o.value.clone(), o.name.clone())
            .checked(current.as_deref() == Some(o.value.as_str()))
    };
    let providers = state.providers();
    if providers.len() <= 1 {
        menu = menu.items(state.options.iter().map(item));
        if let Some(c) = &current {
            menu = menu.selected(c.clone());
        }
    } else {
        for provider in providers {
            let sub = PopupMenu::new()
                .items(state.options.iter().filter(|o| o.provider_name == provider).map(item));
            menu = menu.submenu(provider, sub);
        }
    }
    menu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_mention_being_typed() {
        assert_eq!(mention_query("@"), Some(String::new()));
        assert_eq!(mention_query("look at @no"), Some("no".into()));
        assert_eq!(mention_query("mail me@x"), None);
        assert_eq!(mention_query("@notes done"), None);
        assert_eq!(mention_query("plain"), None);
    }
}
