# Phase 5. Companion panel UI

[Overview](./overview.md)

## Goal

Ship the right-side companion and full-screen surfaces with composer, streaming messages, and setup empty state.

## Changes

- Add Zustand `companion-slice` for messages, open/fullscreen flags, streaming, provider selection, context summary, pending tags.
- Add renderer components: `CompanionPanel`, `CompanionFullscreen`, `CompanionMessages`, `CompanionComposer`, `CompanionSetup`, `CompanionStatus`.
- Mount the panel in `App.tsx` beside the document area without changing left sidebar modes.
- Add composer `@` mention picker over open-folder docs and open tabs.
- Copy and adapt needed AI Elements primitives into Mdow UI ownership.
- Settings dialog gets preferred provider and custom command fields.

## Data structures

- Companion slice mirrors session-only state from the June design. No history persistence.

## Verification

Static: renderer tests for open/close, fullscreen share-session, Enter/Shift+Enter, streaming cancel.
Runtime via control-ui: open panel, send with mock or live provider, confirm document pane stays usable and left sidebar mode is unchanged.
