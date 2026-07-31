# Phase 3. Context builder and file tags

[Overview](./overview.md)

## Goal

Assemble a bounded read-only context packet from the active document, explicit `@` tags, and open-folder markdown.

## Changes

- Add `apps/desktop/src/main/companion/context-builder.ts`.
- Priority order: explicit file tags, active tab, then ranked open-folder markdown snippets.
- Assign stable `sourceId` values per included file/heading snippet for citation validation.
- Enforce size/depth limits and emit truncation warnings.
- Reuse `validateDocumentPath` and markdown extension rules. Exclude non-docs.
- Define the composer tag grammar the renderer will parse (`@filename`, path chips) and the IPC payload shape for tags.

## Data structures

- `ContextPacket`: `{ sources: ContextSource[], warnings: string[], summary: string }`
- `ContextSource`: `{ sourceId, path, headingId?, excerpt, bytes }`
- Tag parse result on the renderer becomes `CompanionContextTag[]` before send

## Verification

Static: tests for active-first ordering, tag inclusion, non-markdown exclusion, path rejection, truncation warnings.
Runtime: sending with a tagged file includes that file's sourceId even when it is not the active tab.
