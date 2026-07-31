# Phase 2. ACP client and provider detection

[Overview](./overview.md)

## Goal

Talk to a real local ACP agent over stdio, and detect whether built-in candidates exist without installing anything.

## Changes

- Add `apps/desktop/src/main/companion/acp-client.ts` for JSON-RPC initialize, session/new, session/prompt, session/cancel, streaming session/update, and shutdown.
- Add `apps/desktop/src/main/companion/provider-detection.ts` for `opencode acp`, Codex ACP adapter (installed binary or package only, never `npx` install), and a user custom command.
- Unit-test the client against a mocked stdio agent.
- Refuse `fs/write_text_file`, terminal, and unsupported tools at the client boundary for read-only mode.

## Data structures

- `AcpSession`: process handle, session id, pending request map, abort controller
- Detection result list reused from phase 1 provider status types

## Verification

Static: companion unit tests for initialize/prompt/cancel/malformed events and detection cases.
Runtime: with local `opencode` present, detection reports available; a mocked prompt round-trip streams text.
