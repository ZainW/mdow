# Phase 1. Domain types and IPC

[Overview](./overview.md)

## Goal

Make the companion domain representable and wire empty typed IPC channels end to end, with no agent process yet.

## Changes

- Add companion types to `apps/desktop/src/shared/types.ts` (provider status, settings, session message, stream update, citation, context summary, tag refs).
- Add IPC channel constants for detect, settings, start, send, cancel, shutdown, and update events.
- Extend preload `window.api` with typed companion methods and a stream subscription.
- Register stub main handlers that return safe empty/not-started responses.
- Persist preferred provider and custom command fields on `AppState` / electron-store.

## Data structures

- `CompanionProviderId`: `'opencode' | 'codex-acp' | 'custom'`
- `CompanionProviderStatus`: id, label, command display, availability (`available` | `missing` | `failed`)
- `CompanionMessage`: id, role, content, citations?, status?
- `CompanionContextTag`: `{ kind: 'file' | 'folder', path, sourceId }`
- `CompanionUpdate`: discriminated union for delta, status, citation, warning, error, done

## Verification

Static: typecheck, lint, existing IPC channel parity test still passes with new channels.
Runtime: renderer can call detect/settings stubs without crashing.
