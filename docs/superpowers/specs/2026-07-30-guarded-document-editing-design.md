# Guarded Document Editing Design

## Summary

Mdow Companion will support document editing through proposals rather than direct agent writes.
The agent may read an allowed markdown document and propose a structured patch. Mdow validates and
renders the patch, and only an explicit user action applies it.

The provider remains responsible for reasoning. Mdow remains the authority over paths, document
versions, writes, and undo.

## Goals

- Support focused edits, rewrites, link fixes, and coordinated markdown changes.
- Show the exact proposed change before any file is modified.
- Require explicit acceptance for every proposal in the first release.
- Restrict all operations to markdown files inside the open folder.
- Detect stale proposals and preserve user edits.
- Apply accepted changes atomically and provide one-click undo.
- Keep the editing interface provider-independent.

## Non-goals

- Unrestricted filesystem or terminal access.
- Automatic edit approval.
- Editing binary, source-code, or non-markdown files.
- Git commits, pushes, or release operations.
- Resolving arbitrary merge conflicts automatically.
- Persisting pending proposals across app restarts.

## Mdow Document Tools

Mdow will expose a narrow, session-scoped tool surface:

### `read_document`

Accepts a folder-relative markdown path and optional line or heading range. Mdow validates the path,
reads the current content, and returns content plus a version hash.

### `propose_document_patch`

Accepts:

- Folder-relative markdown path.
- Base version hash from `read_document`.
- A bounded patch representation.
- A short user-facing rationale.

This tool never writes. Mdow validates the request, computes the resulting content in memory, and
creates a proposal ID. The proposal is emitted to the renderer as a dedicated Companion update.

The editing tool bridge will be registered only when an open folder exists. It will use a
session-specific endpoint and credential, bind to loopback only, and shut down with the ACP
session. FFF remains a separate read-only search server.

## Validation

Before accepting a proposal, the main process must:

- Canonicalize the open-folder root and requested target.
- Reject traversal, symlink escape, missing files, and non-markdown extensions.
- Enforce patch and resulting-document size limits.
- Require the supplied base hash to match the current file.
- Apply the patch in memory and reject malformed or ambiguous hunks.
- Ensure the resulting content remains valid text and preserves the source newline style.

Validation failures return structured errors to the agent and never create an actionable proposal.

## Patch Review Experience

A `PatchReviewCard` appears in the assistant message and shows:

- Target document and rationale.
- Added and removed line counts.
- Compact unified diff with syntax-aware markdown styling.
- `Review`, `Reject`, and `Apply` actions.

`Review` opens a larger diff dialog with full context. The dialog clearly identifies additions,
deletions, and unchanged context and remains keyboard accessible.

`Apply` is disabled while the agent is streaming. Before applying, Mdow rechecks the base hash. A
stale proposal changes state to `Needs regeneration` and cannot be forced through.

`Reject` records the proposal as rejected without changing the file. The chat keeps the proposal
card as an audit trail for the current session.

## Applying and Undoing

On acceptance:

1. Re-read the target and verify the base hash.
2. Write the complete new content to a temporary sibling file.
3. Flush and atomically replace the target.
4. Allow the existing file watcher to refresh the rendered document.
5. Store the immediately previous content in an in-memory bounded undo record.
6. Mark the proposal applied and show `Undo`.

Undo uses the same hash guard: it applies only if the file still matches the accepted result. If
the file changed afterward, Undo is disabled and explains why. Undo records expire when the app
closes and are bounded by count and total bytes.

Multi-file requests produce independent proposals. The first release applies them individually so
a failure in one document cannot partially apply a hidden batch.

## Permissions and Provider Behavior

Mdow continues to deny ACP filesystem writes, terminal requests, and unknown permission requests.
The agent prompt states that the only supported editing path is `propose_document_patch`.

Read-only document tools may run automatically within scope. Patch proposal creation may also run
automatically because it has no side effect. Only the renderer’s explicit Apply action can mutate a
file.

## Error Handling

- Stale base hash: mark the proposal stale and offer regeneration.
- Invalid patch: return a structured tool error with no proposal.
- Path violation: deny, emit a security warning, and reveal no outside content.
- Atomic replacement failure: keep the original file and mark Apply failed.
- Watcher delay: show applied state immediately, then reconcile with the next file update.
- Provider crash: keep existing proposal cards usable as long as their hashes remain current.
- Undo conflict: retain the current file and disable Undo.

## Testing

Unit and integration tests will verify:

- Read and proposal tools accept only in-folder markdown paths.
- Traversal, symlink escape, non-markdown targets, malformed patches, stale hashes, oversized
  payloads, and unsupported encodings are rejected.
- Proposal creation has no filesystem side effects.
- Patch cards render pending, rejected, applied, stale, failed, and undone states.
- Apply performs atomic replacement and preserves newline style.
- File watcher refreshes the active document after Apply and Undo.
- Undo succeeds only against the accepted-result hash.
- Multiple proposals remain independent.
- ACP direct write and terminal requests stay denied.
- Keyboard and screen-reader interaction works in the compact card and full review dialog.

Hands-on verification will cover a paragraph rewrite, heading insertion, internal-link correction,
cross-document terminology update as separate proposals, rejection, stale-file conflict, failed
write simulation, Apply, and Undo in both light and dark themes.

## Acceptance Criteria

- The agent can propose useful markdown changes without directly writing files.
- The user sees and explicitly approves the exact diff before mutation.
- No allowed path can escape the open folder or target a non-markdown file.
- Stale or conflicting changes never overwrite newer content.
- Accepted edits update the open document and can be undone when still safe.
