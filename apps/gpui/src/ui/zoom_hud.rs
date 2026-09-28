//! Transient zoom feedback: a small pill at the bottom-right of the reader that appears after
//! ⌘+ / ⌘− / ⌘0 and fades out, like Electron's `ZoomIndicator`.

use crate::theme::{ColorScheme, Metrics, Theme};
use crate::ui::primitives::{compact_icon_button, tabular_sans};
use gpui::{
    Animation, AnimationExt, AnyElement, App, ClickEvent, FontWeight, IntoElement, Window, div,
    ease_out_quint, prelude::*, px,
};
use std::time::Duration;

/// How long the pill stays after the last zoom change.
pub const VISIBLE_FOR: Duration = Duration::from_millis(1500);
/// Grace period after the pointer leaves the pill (Electron uses 1s).
pub const HOVER_GRACE: Duration = Duration::from_millis(1000);
pub const FADE_IN: Duration = Duration::from_millis(150);
pub const FADE_OUT: Duration = Duration::from_millis(200);

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn mdow_reduce_motion() -> i32;
}

/// The macOS "Reduce motion" accessibility setting: the pill then appears and disappears
/// without fading.
pub fn prefers_reduced_motion() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { mdow_reduce_motion() != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudPhase {
    Hidden,
    Shown,
    Fading,
}

/// Timers are keyed by `generation`: any newer show/hover bumps it, so stale timers do nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoomHud {
    phase: HudPhase,
    generation: u64,
    /// Bumps only on a hidden → shown transition, so repeated ⌘+ doesn't replay the fade-in.
    appearance: u64,
    hovered: bool,
}

impl Default for ZoomHud {
    fn default() -> Self {
        Self {
            phase: HudPhase::Hidden,
            generation: 0,
            appearance: 0,
            hovered: false,
        }
    }
}

impl ZoomHud {
    pub fn phase(&self) -> HudPhase {
        self.phase
    }

    pub fn is_visible(&self) -> bool {
        self.phase != HudPhase::Hidden
    }

    /// Show (or keep showing) the pill; returns the generation to expire after [`VISIBLE_FOR`].
    pub fn show(&mut self) -> u64 {
        if self.phase == HudPhase::Hidden {
            self.appearance += 1;
        }
        self.phase = HudPhase::Shown;
        self.generation += 1;
        self.generation
    }

    /// The visible period for `generation` ended. Returns the generation to finish fading after
    /// [`FADE_OUT`], or `None` when nothing needs to happen later.
    pub fn expire(&mut self, generation: u64, reduce_motion: bool) -> Option<u64> {
        if generation != self.generation || self.hovered || self.phase != HudPhase::Shown {
            return None;
        }
        if reduce_motion {
            self.phase = HudPhase::Hidden;
            return None;
        }
        self.phase = HudPhase::Fading;
        self.generation += 1;
        Some(self.generation)
    }

    pub fn finish_fade(&mut self, generation: u64) -> bool {
        if generation != self.generation || self.phase != HudPhase::Fading {
            return false;
        }
        self.phase = HudPhase::Hidden;
        true
    }

    /// Hovering holds the pill; leaving returns the generation to expire after [`HOVER_GRACE`].
    pub fn hover(&mut self, hovered: bool) -> Option<u64> {
        if !self.is_visible() {
            return None;
        }
        self.hovered = hovered;
        self.generation += 1;
        if hovered {
            self.phase = HudPhase::Shown;
            None
        } else {
            Some(self.generation)
        }
    }
}

pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub type HoverHandler = Box<dyn Fn(&bool, &mut Window, &mut App)>;

pub struct ZoomHudHandlers {
    pub zoom_out: ClickHandler,
    pub zoom_in: ClickHandler,
    pub reset: ClickHandler,
    pub hover: HoverHandler,
}

pub fn render_zoom_hud(
    hud: &ZoomHud,
    percent: u16,
    reduce_motion: bool,
    theme: Theme,
    handlers: ZoomHudHandlers,
) -> Option<AnyElement> {
    if !hud.is_visible() {
        return None;
    }
    let ZoomHudHandlers {
        zoom_out,
        zoom_in,
        reset,
        hover,
    } = handlers;
    let at_default = percent == 100;
    let pill = div()
        .id("zoom-hud")
        .debug_selector(|| "zoom-hud".into())
        .absolute()
        .right(px(16.0))
        .bottom(px(16.0))
        .flex()
        .items_center()
        .p(px(3.0))
        .rounded(px(Metrics::RADIUS))
        .bg(theme.surface_raised)
        .map(|pill| match theme.color_scheme {
            ColorScheme::Light => pill
                .border_1()
                .border_color(theme.border_subtle)
                .shadow_md(),
            ColorScheme::Dark => pill.border_1().border_color(theme.border),
        })
        .occlude()
        .font_family(Metrics::FONT_SANS)
        .text_size(px(12.0))
        .text_color(theme.foreground)
        .on_hover(hover)
        .child(compact_icon_button(
            "zoom-hud-out",
            "icons/minus.svg",
            26.0,
            13.0,
            theme,
            zoom_out,
        ))
        .child(
            div()
                .w(px(46.0))
                .text_center()
                .font(tabular_sans(FontWeight::MEDIUM))
                .child(format!("{percent}%")),
        )
        .child(compact_icon_button(
            "zoom-hud-in",
            "icons/plus.svg",
            26.0,
            13.0,
            theme,
            zoom_in,
        ))
        .child(
            div()
                .w(px(1.0))
                .h(px(16.0))
                .mx(px(3.0))
                .bg(theme.border_subtle),
        )
        .child(
            div()
                .id("zoom-hud-reset")
                .debug_selector(|| "zoom-hud-reset".into())
                .flex()
                .items_center()
                .h(px(26.0))
                .px(px(8.0))
                .rounded(px(5.0))
                .text_size(px(11.5))
                .text_color(if at_default {
                    theme.muted_foreground.opacity(0.5)
                } else {
                    theme.muted_foreground
                })
                .when(!at_default, |button| {
                    button
                        .tab_index(0)
                        .focusable()
                        .cursor_pointer()
                        .hover(move |style| style.bg(theme.muted).text_color(theme.foreground))
                        .focus(move |style| style.border_1().border_color(theme.primary))
                        .on_click(reset)
                })
                .child("Reset"),
        );

    Some(match (reduce_motion, hud.phase) {
        (true, _) => pill.into_any_element(),
        (false, HudPhase::Fading) => pill
            .with_animation(
                ("zoom-hud-fade-out", hud.generation),
                Animation::new(FADE_OUT).with_easing(ease_out_quint()),
                |pill, delta| pill.opacity(1.0 - delta),
            )
            .into_any_element(),
        (false, _) => pill
            .with_animation(
                ("zoom-hud-fade-in", hud.appearance),
                Animation::new(FADE_IN).with_easing(ease_out_quint()),
                |pill, delta| pill.opacity(delta),
            )
            .into_any_element(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_then_fades_then_hides() {
        let mut hud = ZoomHud::default();
        let shown = hud.show();
        assert_eq!(hud.phase(), HudPhase::Shown);
        let fading = hud.expire(shown, false).expect("fade scheduled");
        assert_eq!(hud.phase(), HudPhase::Fading);
        assert!(hud.finish_fade(fading));
        assert!(!hud.is_visible());
    }

    #[test]
    fn a_newer_zoom_change_outlives_the_older_timer() {
        let mut hud = ZoomHud::default();
        let first = hud.show();
        let second = hud.show();
        assert_eq!(hud.expire(first, false), None);
        assert_eq!(hud.phase(), HudPhase::Shown);
        assert!(hud.expire(second, false).is_some());
    }

    #[test]
    fn repeated_zooms_while_visible_do_not_replay_the_entrance() {
        let mut hud = ZoomHud::default();
        hud.show();
        let appearance = hud.appearance;
        hud.show();
        assert_eq!(hud.appearance, appearance);
    }

    #[test]
    fn zooming_during_the_fade_brings_the_pill_back() {
        let mut hud = ZoomHud::default();
        let shown = hud.show();
        let fading = hud.expire(shown, false).unwrap();
        let again = hud.show();
        assert!(!hud.finish_fade(fading));
        assert_eq!(hud.phase(), HudPhase::Shown);
        assert!(hud.expire(again, false).is_some());
    }

    #[test]
    fn reduced_motion_hides_immediately() {
        let mut hud = ZoomHud::default();
        let shown = hud.show();
        assert_eq!(hud.expire(shown, true), None);
        assert!(!hud.is_visible());
    }

    #[test]
    fn hovering_holds_the_pill_and_leaving_restarts_a_grace_period() {
        let mut hud = ZoomHud::default();
        let shown = hud.show();
        assert_eq!(hud.hover(true), None);
        assert_eq!(hud.expire(shown, false), None);
        assert!(hud.is_visible());
        let grace = hud.hover(false).expect("grace timer");
        assert!(hud.expire(grace, false).is_some());
    }

    #[test]
    fn hover_on_a_hidden_pill_is_ignored() {
        let mut hud = ZoomHud::default();
        assert_eq!(hud.hover(true), None);
        assert_eq!(hud.hover(false), None);
        assert!(!hud.is_visible());
    }
}
