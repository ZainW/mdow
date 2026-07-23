# Phase 4. Companion service

[Overview](./overview.md)

## Goal

Orchestrate provider lifecycle, context assembly, and streaming updates behind the real IPC handlers.

## Changes

- Add `apps/desktop/src/main/companion/service.ts`.
- Wire detect/settings/start/send/cancel/shutdown handlers in `ipc.ts`.
- On send: build context, start or reuse ACP session, prompt with read-only instructions, stream `companion:update` events.
- Map ACP updates into companion update types. Preserve user messages on error.
- Shut down the child process on window close / shutdown IPC.

## Data structures

- Service holds optional active `AcpSession`, selected provider, last context packet for citation validation

## Verification

Static: service tests with mocked ACP client and context builder.
Runtime: end-to-end IPC send from a small script or integration test streams done/error events.
