# Companion Workspace Design

Date: 2026-07-31

## Summary

Mdow Companion will gain a first-class workspace mode for focused AI work. The existing drawer
remains available for quick questions, while expanding Companion replaces the reader layout with a
full-window conversation workspace. The workspace keeps one shared conversation, can reveal a
temporary artifact pane for sources and proposed edits, and returns the user to the exact reading
state they left.

This increment also makes context use substantially more efficient. Mdow will stop eagerly adding
the first set of folder documents to every prompt, reuse unchanged sources within an ACP session,
and retrieve additional documents on demand through the optional FFF MCP integration. Edit mode
will use Mdow-owned proposal tools so no file changes until the user reviews and applies a diff.

The design adapts the useful interaction model of an agent-centered workspace without importing
Cursor's IDE-specific multi-agent, terminal, branch, or worktree features.

## Goals

- Provide a focused full-window Companion experience without opening another native window.
- Keep one conversation when moving between closed, drawer, and workspace presentations.
- Preserve and restore the reader's active document, pane, selection, and scroll state.
- Make sources, search results, and edit proposals inspectable without permanently showing a
  document beside the conversation.
- Reduce repeated prompt content through source hashing, retrieval, and smaller explicit budgets.
- Support useful markdown editing through reviewable proposals, explicit Apply, and guarded Undo.
- Keep filesystem access scoped to markdown files inside the open folder.
- Degrade cleanly when FFF or an ACP capability is unavailable.

## Non-goals

- Multiple concurrent agents, tiled agent sessions, or persistent chat history.
- A separate Companion operating-system window.
- Unrestricted filesystem, terminal, shell, Git, or network access.
- Automatic edit approval or background document mutation.
- Editing source code, binary files, HTML files, or files outside the open folder.
- Exact token accounting when the provider does not report usage.
- Semantic or embedding-based document retrieval.

## Existing Behavior and Migration

Companion currently uses two booleans, `companionOpen` and `companionFullscreen`, and renders the
expanded view inside a large dialog. The context builder can attach up to 20 open-folder documents
and 120 KB of source text to each prompt.

The presentation state will become one discriminated value:

- `closed`
- `drawer`
- `workspace`

This removes invalid combinations such as an open inert drawer behind a full-screen dialog. The
existing conversation, provider selection, messages, context tags, streaming state, and update
pipeline remain shared across all three presentations.

The existing FFF search, focused-document/model-picker, and guarded-editing design specs remain the
detailed security references for their respective integrations. This spec defines how they compose
into the Companion workspace and tightens the prompt-budget behavior.

## Workspace Experience

### Entering and Leaving

Expanding the drawer changes the application presentation to `workspace`; it does not open a
dialog. Mdow retains the native titlebar but replaces the sidebar, document tabs, breadcrumb,
reader, update banner, and drawer with the Companion workspace.

The workspace header contains:

- A prominent `Back to document` control.
- The current conversation title or `Companion` fallback.
- Provider and model controls when available.
- A compact context-usage indicator.
- A menu for response/tool-detail density and Companion settings.

Returning to the reader restores the prior sidebar visibility and mode, active tab, active split
pane, document scroll location, and text selection when those targets still exist. If the active
document was removed, Mdow returns to its existing deleted-file/error behavior rather than choosing
an unrelated document.

The existing expand button and a command-palette action enter workspace mode. `Escape` closes an
open artifact overlay first; it does not unexpectedly discard a draft or leave workspace mode.
`Back to document` and its keyboard shortcut are the explicit workspace exit paths.

### Conversation Layout

The default workspace contains a centered conversation column approximately 800–900 px wide. The
message history scrolls independently, while the composer remains pinned at the bottom within the
same column. Long messages, code blocks, reasoning, and tool output must not widen the column.

Tool activity defaults to a compact presentation: name, status, and concise result. Users may
expand an individual tool card or choose compact, balanced, or detailed display density. Display
density changes rendering only; it is never described as a token-saving control.

The composer contains:

- An `Ask` / `Edit` mode selector.
- The focused document as non-removable status.
- Removable explicit file and folder context chips.
- Document-search readiness.
- An approximate or provider-reported context-usage value.
- The multiline prompt and Send/Cancel action.

The drawer uses the same messages and composer behavior in a denser layout. Workspace-only controls
may collapse into menus in the drawer, but their semantics remain identical.

### Artifact Pane

The workspace does not show a document by default. Selecting a citation, search result, or edit
proposal opens a temporary artifact pane beside the conversation. The conversation narrows but
keeps its scroll position and draft.

The artifact pane supports three initial artifact types:

- A read-only markdown source with its path and relevant location.
- A compact search-results list with query and result summaries.
- A patch review containing rationale, path, line counts, and a unified diff.

Only one artifact is active at a time. Closing it restores the centered conversation. On narrow
windows, the artifact appears as a modal sheet over the workspace with equivalent keyboard and
screen-reader behavior.

## Context and Token Efficiency

### Context Ledger

Each active ACP session owns an in-memory context ledger keyed by canonical source path. A ledger
entry records the source ID, content hash, byte count, and the turn on which full content was last
sent.

For focused documents and explicit file tags:

1. Send bounded content the first time the source is used in the session.
2. Re-read and hash the source before a later turn.
3. If the hash is unchanged, send only its source ID, path, focused/tagged role, and hash.
4. If the hash changed, send the new bounded content and update the ledger.
5. If the provider session restarts, rebuild the ledger and resend required content.

The prompt identifies the focused document on every turn even when its content is reused. Explicit
tags cannot replace focused-document identity.

### Retrieval

When FFF is available, Mdow registers the read-only, open-folder-scoped FFF MCP server with the ACP
session. The open folder is not eagerly appended to the prompt. The agent searches by path or
content when broader context is needed, then reads bounded markdown excerpts through Mdow's scoped
read capability.

When FFF is unavailable, focused-document chat and editing remain usable. For folder-wide
questions, a lightweight fallback ranks markdown paths using the current question, filenames,
titles, and exact text matches. It includes only the highest-ranked few sources within the same
turn budget. It never falls back to the current arbitrary first-20-files behavior.

FFF readiness states are:

- `Document search ready`
- `Focused document only`
- `Enable document search`
- `Document search unavailable`

Missing FFF provides installation guidance but never makes Companion unavailable.

### Budgets and Usage Display

The initial limits are:

- 24 KB maximum for one focused or explicitly tagged source.
- 48 KB combined source/excerpt content per turn.
- A small bounded result count and excerpt size for each search/read tool call.
- Concise tool results in conversation state; large raw payloads remain outside rendered messages.

These limits replace the current 120 KB total and 20-file eager folder inclusion. Constants remain
centralized so tests can verify budget behavior and later tuning does not change security checks.

If the ACP provider reports token usage, Mdow displays the confirmed value. Otherwise it estimates
tokens from UTF-8 content size and prefixes the number with `≈`. The UI also shows focused source,
explicit-reference count, and search readiness so the estimate is not mistaken for a complete
provider billing total.

Changing provider or open folder starts a new provider/search session and clears the old ledger.
No source identity or retrieved excerpt crosses that boundary.

## Ask and Edit Modes

Ask mode remains read-only and is the default. Edit mode changes the agent instructions and exposes
two narrow Mdow-owned document tools only when a validated open folder exists.

### `read_document`

Accepts a folder-relative markdown path and optional bounded range. Mdow validates the canonical
path, rejects traversal and symlink escape, and returns content plus a version hash.

### `propose_document_patch`

Accepts a folder-relative markdown path, base version hash, bounded patch, and short rationale. It
validates and applies the patch in memory, creates a proposal, and emits a dedicated Companion
update. It never writes the target file.

The resulting proposal card shows path, rationale, added/removed counts, and `Review`, `Reject`, and
`Apply` actions. `Review` opens the artifact pane with the full diff. `Apply` remains disabled while
the proposal is incomplete or while a conflicting operation is active.

Before Apply, Mdow re-reads the target and verifies the base hash. A stale proposal changes to
`Needs regeneration`; it cannot be forced through. A successful Apply uses an atomic sibling-file
replacement, lets the existing watcher refresh the reader, and records the previous content in a
bounded in-memory undo entry.

Undo succeeds only while the target still matches the applied-result hash. Rejection and failed or
stale proposals remain in the session conversation as an audit trail. Multi-file requests create
independent proposals that users apply individually.

Direct ACP write requests, terminal requests, non-markdown targets, unknown tools, traversal,
symlink escape, and automatic approval remain denied in both modes.

## Data Flow

### Ask Turn

1. The renderer snapshots the focused document, folder, explicit tags, selected provider/model,
   and mode.
2. The main process starts or reuses the provider session scoped to that folder.
3. The context builder re-hashes required sources and consults the session ledger.
4. It formats the compact policy, focused-source identity, changed/new source content, reused-source
   references, and the user's question within the turn budget.
5. The provider may use allowlisted FFF search and bounded read tools for additional context.
6. Updates stream into the existing Zustand conversation state.
7. Citations are validated against provided or retrieved source IDs before becoming interactive.
8. The renderer records per-turn context statistics for the usage indicator.

### Edit Turn

1. The Ask flow supplies focused context and retrieval capabilities.
2. The provider reads the target through `read_document` and receives its version hash.
3. It calls `propose_document_patch`; main-process validation creates a proposal with no write.
4. The renderer shows the proposal card and artifact diff.
5. Only an explicit renderer Apply IPC action can perform the guarded atomic write.
6. Apply or Undo updates proposal state, while the normal watcher refreshes open document content.

## Lifecycle and Error Handling

- **Provider unavailable:** Preserve the conversation and show setup/retry actions.
- **Provider crash:** Settle active streaming state, preserve messages and proposals, and reconnect
  on the next send.
- **FFF missing:** Use focused context and ranked fallback without blocking chat or editing.
- **FFF crash or timeout:** Mark the tool failed and let the provider continue with known context.
- **Folder change:** Shut down the old provider, FFF, and editing-tool sessions; clear the ledger;
  preserve visible chat messages but label their prior-folder context where needed.
- **Source changed:** Resend changed content on the next turn.
- **Source deleted or unreadable:** Remove it from active context and show a concise warning.
- **Oversized source/result:** Truncate at a safe text boundary and report the truncation.
- **Invalid or outside path:** Deny the request, reveal no outside content, and emit a security
  warning.
- **Stale proposal:** Disable Apply and request regeneration.
- **Apply failure:** Keep the original document intact and show a retryable proposal error.
- **Undo conflict:** Keep the current file and explain why Undo is no longer safe.
- **Narrow layout:** Present artifacts as an overlay without losing the draft or conversation
  position.

## Accessibility and Keyboard Behavior

- Presentation state changes move focus to the workspace heading or restored reader target.
- `Back to document`, artifact close, mode selection, tool disclosure, proposal actions, and diff
  navigation are fully keyboard accessible.
- `Enter` sends and `Shift+Enter` inserts a newline.
- While streaming, the primary action becomes Cancel.
- `Escape` closes transient popovers and the artifact overlay before any broader navigation.
- Streaming status uses a polite live region; security and apply failures use an assertive alert.
- Diff additions and deletions use text labels and semantics in addition to color.
- Reduced-motion preferences disable non-essential streaming and pane transitions.

## Testing

Automated tests will cover:

- Valid transitions among `closed`, `drawer`, and `workspace` with one shared conversation.
- Reader state capture and restoration for single- and split-pane layouts.
- Workspace and artifact responsive behavior and focus restoration.
- Context-ledger first-send, unchanged reuse, changed resend, and session-reset behavior.
- Focused-document identity on every turn.
- The 24 KB per-source and 48 KB per-turn limits, including UTF-8 boundaries.
- FFF-enabled omission of eager folder context and ranked fallback when FFF is unavailable.
- Retrieval scoping, result limits, traversal rejection, and symlink-escape rejection.
- Provider-reported usage versus clearly marked estimates.
- Ask/Edit prompt and tool-surface separation.
- Proposal creation without writes and all pending, applied, rejected, stale, failed, undone states.
- Atomic Apply, watcher refresh, guarded Undo, and multi-proposal independence.
- Provider/search failures settling streaming and preserving usable state.
- Keyboard navigation, live-region behavior, accessible proposal controls, and non-color diff cues.

Hands-on verification will cover:

- Moving between drawer, workspace, artifact review, and the exact prior document location.
- Long streaming conversations and expanded/collapsed tool details.
- Focused-document follow-ups that reuse unchanged content.
- Folder-wide questions with FFF ready, missing, and failed.
- A paragraph rewrite, heading insertion, link correction, rejection, Apply, stale conflict, and Undo.
- Light and dark themes at compact, comfortable, and large interface scales.
- Narrow and wide windows using a real supported ACP provider.

## Acceptance Criteria

- Expanding Companion produces a true full-window workspace rather than a dialog.
- The reader is hidden by default in workspace mode and restored without losing navigation state.
- Citations, search results, and edit reviews share one consistent artifact-pane interaction.
- Unchanged focused and explicitly tagged sources are not resent in full on every turn.
- Folder documents are retrieved on demand with FFF or selected through a small ranked fallback.
- A normal turn cannot exceed the configured source-content budget.
- Token usage is exact only when provider-supplied and otherwise visibly approximate.
- Edit mode cannot modify a file without explicit Apply against a current version hash.
- No allowed tool can escape the open folder or edit a non-markdown file.
- Missing FFF or a recoverable provider failure does not destroy the conversation.
