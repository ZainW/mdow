//! The AI Companion: a right-side panel that asks a locally installed agent CLI about the open
//! documents over the Agent Client Protocol. Ported from Electron's `main/companion` service
//! and `renderer/components/companion` UI.
//!
//! - [`acp`]: JSON-RPC wire format and ACP message parsing
//! - [`provider`]: detecting installed agent CLIs
//! - [`context`]: which documents and sections go with each question
//! - [`service`]: the agent subprocess state machine, run off the UI thread
//! - [`state`]: the conversation model the panel renders
//! - [`ui`]: the panel, composer and answer rendering

pub mod acp;
pub mod composer;
pub mod context;
pub mod markdown;
pub mod provider;
pub mod service;
pub mod settings;
pub mod state;
pub mod types;
pub mod ui;

pub use types::ProviderId;
pub use ui::{CompanionEvent, CompanionPanel};

use gpui::KeyBinding;

gpui::actions!(
    companion,
    [
        /// Show or hide the companion panel.
        ToggleCompanion,
        /// Start a new companion conversation.
        NewCompanionChat
    ]
);

/// ⌘I: "ask about this document". Electron has no shortcut for the drawer.
pub const TOGGLE_KEYS: &str = "cmd-i";
pub const TOGGLE_KEYS_LABEL: &str = "⌘I";

/// Every companion key binding; `main.rs` appends these after the app's own bindings.
pub fn key_bindings() -> Vec<KeyBinding> {
    let mut bindings = vec![KeyBinding::new(TOGGLE_KEYS, ToggleCompanion, None)];
    bindings.extend(composer::key_bindings());
    bindings
}
