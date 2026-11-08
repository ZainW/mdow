//! The OpenCode companion: chat about the document and its folder, with every edit shown as a
//! diff to accept or decline. It runs a private `opencode serve` shared by all windows.

pub mod client;
pub mod conversation;
pub mod events;
pub mod logic;
pub mod panel;
pub mod process;
pub mod service;
pub mod types;
