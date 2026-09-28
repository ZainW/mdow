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
pub mod context;
pub mod provider;
pub mod service;
pub mod settings;
pub mod state;
pub mod types;

pub use types::ProviderId;
