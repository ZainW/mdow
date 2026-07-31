# Phase 6. Citations and setup polish

[Overview](./overview.md)

## Goal

Make answers actionable inside Mdow and make missing-provider setup clear and safe.

## Changes

- Validate citation source IDs against the last context packet. Drop or de-link invalid ones.
- Citation chips open the file and scroll to heading when present.
- Setup empty state lists opencode, Codex ACP, and custom command with install hints and retry. No `npx` install.
- Surface context truncation warnings under the composer summary (`Using README.md + 3 tagged + 14 docs`).
- Document that OpenCode Go is configured inside OpenCode, not as a Mdow provider row.

## Data structures

- No new core types. Citation render model: `{ sourceId, path, headingId?, label }`

## Verification

Static: citation validation tests. Setup state tests for missing/available/failed.
Runtime done predicate from overview: live opencode ACP question with tag + citation open. Cancel mid-stream. No-provider setup path.
