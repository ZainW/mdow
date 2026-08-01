# Adaptive Companion Efficiency and Model Picker Design

Date: 2026-08-01

## Summary

Mdow Companion will replace eager folder-context injection with focus-first adaptive retrieval. A
turn begins with only the focused document and explicitly attached files. Linked documents are
represented as metadata, and broader folder content is searched and read only when the question
requires it. The system will expose what it actually injected, searched, read, and reused without
claiming visibility into provider context that ACP does not report.

The Companion header will gain a searchable model picker populated exclusively from the active
OpenCode ACP instance. The first supported catalog groups are the user's OpenAI ChatGPT
subscription, OpenCode Zen, and OpenCode Go. Mdow will never maintain its own model catalog or show
models that the active instance does not advertise.

The composer will adopt the approved compact context-capsule layout. New interactive primitives
will be locally owned shadcn components backed by Base UI. Context detail, warnings, explicit
attachments, and retrieval history move into a compact popover instead of consuming permanent
vertical space.

## Goals

- Inject exactly the focused or explicitly referenced document by default.
- Retrieve additional evidence only when the current question requires it.
- Keep linked-document metadata cheap until a linked document must be read.
- Bound every source, search result, read range, and turn-level context packet.
- Avoid resending unchanged document bodies within one provider session.
- Show an honest, compact trace of injected and retrieved context.
- Populate the model picker from the user's active OpenCode instance.
- Support the OpenAI subscription, OpenCode Zen, and OpenCode Go model groups initially.
- Reduce the composer context UI to one line with details available on demand.
- Use shadcn-owned components backed by Base UI for new chat controls.

## Non-goals

- A hard-coded global model catalog.
- Direct authentication with OpenAI, OpenCode Zen, or OpenCode Go.
- Reading OpenCode credential files in Mdow.
- Direct calls from Mdow to public OpenCode model endpoints.
- Semantic embeddings or a persistent vector database.
- Eagerly summarizing or injecting every file in the open folder.
- Exact total-provider token accounting when ACP does not report it.
- Adding write, terminal, shell, or unrestricted filesystem access.
- Migrating unrelated legacy components or dependencies away from Radix.

## Existing Problem

`buildCompanionContext` currently includes the focused document, explicit tags, and then up to the
first 20 markdown files from the open folder. The limits allow 24 KB per source and 120 KB total.
Opening a folder therefore changes a one-document question into a many-document prompt even when
the user referenced only one file. The folder order is not a relevance signal, so the additional
tokens can be both expensive and misleading.

The current ACP session passes no MCP servers, advertises no read capability, and ignores model
configuration returned by `session/new`. The renderer shows context summary text and warnings in a
persistent block above the composer. Explicit tags add another row, so the composer grows as
context becomes more complex.

## Selected Direction

Use focus-first adaptive retrieval:

1. Build a deterministic minimal packet from the focused document and explicit file attachments.
2. Represent in-document links as a bounded reference manifest without linked file bodies.
3. Give the active provider folder-scoped, read-only search and range-read tools.
4. Let the provider search when the question requires evidence beyond the initial packet.
5. Return short ranked search results before reading any additional file content.
6. Read only the few relevant ranges the provider selects.
7. Record the turn's actual context activity for the compact context capsule.

This keeps ordinary single-document questions cheap while preserving cross-document answers.

## Focused Document Packet

The focused document is snapshotted at send time from the active pane. It remains semantically
distinct from explicit attachments and search results.

### Small Documents

A small focused document is included as one bounded source on its first use in the active provider
session. The prompt identifies it explicitly so phrases such as "this document" and "this file"
cannot resolve to an attachment or search result.

### Large Documents

A large focused document is divided along markdown heading boundaries. The initial packet contains:

- Document path and stable source ID.
- Title and bounded heading map.
- Sections selected through deterministic lexical relevance to the current question.
- A visible truncation or partial-context flag when the entire document was not sent.

Requests that require broader coverage, such as whole-document summaries, can use the scoped range
reader to fetch additional sections. The builder never treats the arbitrary first bytes of a large
file as sufficient context.

### Explicit Attachments

Explicit `@file` attachments have the same content rules as the focused document but remain
removable. They cannot replace focused-document identity. Explicit folder attachment does not mean
"inject the folder"; it authorizes scoped adaptive search within that folder.

## Linked Reference Manifest

Mdow parses standard markdown links in focused and explicitly attached documents. The manifest
contains only validated, bounded metadata:

- Link text.
- Resolved folder-relative target path.
- Optional heading anchor.
- Whether the target is a supported markdown document inside the open folder.

The target body is not read or included merely because a link exists. The provider can search or
read a linked target when the question depends on it. Invalid, external, unsupported, traversing,
or outside-folder references are described as unavailable without revealing outside content.

## Adaptive Search and Read

### FFF Path

When available, Mdow registers the FFF MCP server with the active OpenCode ACP session. Its root is
the canonical open folder. Only allowlisted read-only path and content search operations are
available. Search results return bounded entries containing path, match location, and a short
snippet. Full raw result payloads are not appended to the conversation or context packet.

The provider calls search when the question requires broader evidence, such as comparisons,
references to other documents, named but unattached files, or unresolved terms from the focused
document.

### Local Fallback

If FFF is unavailable, a bounded lexical fallback runs only when cross-document intent is
structurally observable: the user attached a folder, named a different file, or asked for an
explicit multi-document comparison. It ranks supported markdown paths by filename, title, heading,
exact phrase, and question-term matches. It returns the same compact result shape as FFF and reads
only selected ranges afterward.

The fallback never reintroduces arbitrary folder ordering or first-20-file injection. A normal
single-document question performs no folder-body scan for prompt construction.

### Scoped Range Reader

The ACP client advertises read-only text-file capability and implements a bounded markdown reader.
Every request must:

- Resolve to a canonical supported markdown path.
- Remain inside the authorized open folder.
- Reject traversal and symlink escape.
- Specify or receive a bounded byte/line range.
- Respect per-read and per-turn budgets.

Write requests, terminal requests, unknown tools, unsupported extensions, and outside paths remain
denied.

## Context Ledger and Budgets

Each active provider session owns an in-memory ledger keyed by canonical path. An entry records the
source ID, role, content hash, byte count, and last turn on which content was sent.

On later turns:

- Unchanged required sources send identity, role, and hash rather than another full body.
- Changed sources send newly selected bounded content and update the ledger.
- Search and range reads add or update ledger entries only for content actually returned.
- Provider-session restarts and folder changes clear the ledger. Switching models inside the same
  confirmed ACP session retains the ledger because the conversation remains the same.
- If the provider needs details no longer retained after compaction, it can reload them through the
  scoped reader.

Initial centralized limits are:

- 16 KB before a focused or explicit document switches to section selection.
- 16 KB maximum initial content from one source.
- 4 KB maximum content from one additional read range.
- Three additional read ranges per turn.
- 32 KB combined source and retrieved content per turn.
- Small bounded search result and reference-manifest counts.

The exact constants live together and are tested as security and efficiency boundaries.

## Context Trace and Usage Language

Each turn emits structured context statistics instead of a preformatted sentence. The renderer can
show a compact summary such as:

`1 focused · searched 1 · read 2 · ≈6.2k added`

The estimate covers content Mdow added to the turn. It is not labeled as total context or billing
usage. When ACP reports authoritative usage, the UI can show the confirmed value separately.

The detailed trace distinguishes:

- Focused sources.
- Explicit attachments.
- Linked references discovered as metadata.
- Search results considered.
- Documents and ranges actually read.
- Sources reused by hash.
- Truncation, omitted results, and budget warnings.

## OpenCode Instance Model Picker

Mdow treats the active OpenCode ACP session as the only model-catalog authority. It parses the
model configuration advertised by `session/new` and presents only available values from that
specific instance.

The first supported groups are identified by model ID prefix:

- `openai/*`: the user's OpenAI ChatGPT subscription or other OpenAI connection configured in
  OpenCode.
- `opencode/*`: OpenCode Zen.
- `opencode-go/*`: OpenCode Go.

A group is absent when the active instance does not advertise any matching values. Mdow does not
assume that authentication exists, inspect credentials, fetch a public catalog, or invent models
from documentation.

The picker uses the provider-advertised label when available and retains the raw ID for searching
and disambiguation. Selecting a model calls `session/set_config_option` with the session's model
configuration ID. The returned configuration state is authoritative.

Model switching is disabled while a response is streaming or a switch is pending. A failed switch
keeps the previous confirmed selection and shows a non-blocking error. The last confirmed model is
persisted as a preference; if it disappears, Mdow accepts the instance's current value and replaces
the stale preference.

The existing non-OpenCode ACP providers can continue to function. This increment does not promise a
picker for providers that do not advertise compatible model configuration.

References:

- OpenCode provider setup and ChatGPT Plus/Pro authentication:
  <https://dev.opencode.ai/docs/providers>
- OpenCode model selection behavior: <https://opencode.ai/v2/docs/models>

## Compact Base UI Experience

The approved layout uses one compact row inside the composer:

- Focused-document capsule, such as `product-brief.md`.
- Adaptive context capsule, such as `Adaptive · 1 + 2`.
- Muted injected-context estimate, such as `≈6.2k`.

The row remains one line tall. Long names truncate, and the detailed state moves into a context
popover. The popover lists focused, explicit, discovered, read, and reused sources in separate
sections. Warnings and truncation details live there unless immediate action is required.

The `+` action opens a searchable document attachment control. Existing `@file` entry remains a
keyboard shortcut and uses the same candidate and selection state. Removing an explicit attachment
does not remove the focused document or historical retrieval trace.

The header contains one compact searchable model trigger, for example:

`OpenAI · GPT 5.6 Sol`

The picker groups the live catalog, filters by display label and raw ID, marks the confirmed model,
and exposes loading, empty, switching, and error states.

## Component Boundaries

The existing large Companion component will be separated into units with explicit responsibilities:

- `CompanionHeader`: navigation, model trigger, and window controls.
- `ModelCombobox`: live catalog filtering, grouping, selection, and pending state.
- `CompanionMessages`: conversation and streaming presentation.
- `ContextCapsule`: one-line summary and popover trigger.
- `ContextPopover`: detailed trace, warnings, and attachment management.
- `CompanionComposer`: draft, send/cancel, mention shortcut, and compact controls.

New primitives will be added through the repository-scoped shadcn CLI and owned locally. The model
picker uses the shadcn Combobox backed by Base UI. Context detail uses the shadcn Popover backed by
Base UI. Existing Button, Badge, Tooltip, Collapsible, ScrollArea, Input Group, and other local
primitives are reused where appropriate.

No new Radix primitive is introduced for this work. Existing unrelated Radix dependencies are not
part of this migration.

References:

- shadcn Base UI Combobox: <https://ui.shadcn.com/docs/components/base/combobox>
- shadcn Base UI Popover: <https://ui.shadcn.com/docs/components/base/popover>

## Data Flow

### Session Start

1. Renderer selects the OpenCode provider and current folder.
2. Main process starts or reuses the folder-scoped ACP process.
3. `session/new` returns session ID and available configuration.
4. Main process parses compatible model options and returns them to the renderer.
5. Renderer reconciles the live current model with its saved preference.

### Prompt Turn

1. Renderer snapshots focused path, explicit attachments, selected provider/model, open folder, and
   question.
2. Main process validates the snapshot and ensures the provider session matches the folder.
3. Context builder creates the focused packet, reference manifest, and ledger reuse entries within
   budget.
4. Provider receives the minimal prompt and may invoke allowlisted search and range-read tools.
5. Main process validates each tool request, enforces remaining turn budget, and records trace data.
6. Provider streams its answer and validated citations.
7. Main process emits final structured context statistics for the renderer capsule and popover.

### Model Change

1. Renderer disables the picker and sends the selected advertised value.
2. Main process calls `session/set_config_option` on the active session.
3. Main process parses the returned current configuration.
4. Renderer updates and persists only the confirmed returned value.

## Error Handling

- Focused file unreadable: show a compact actionable error and send no inferred replacement.
- Large document has no strong matching section: send heading map and bounded leading context, then
  let the provider request ranges.
- FFF missing: use the lexical fallback only for cross-document intent.
- FFF timeout or crash: mark search unavailable for the turn and continue with known context.
- Search returns too many matches: rank and truncate before returning results.
- Read exceeds budget: deny or truncate with a structured budget warning.
- Linked path is invalid or outside scope: expose no content and mark it unavailable.
- Provider or folder changes: terminate old scoped processes and clear the context ledger.
- Model catalog missing: hide the picker without blocking chat.
- Saved model stale: use and persist the live current model.
- Model switch rejected: retain the prior confirmed model.
- Context warnings: keep them in the popover unless the user must act immediately.

## Accessibility

- Model selection and document attachment use Base UI combobox keyboard semantics.
- Context detail uses a Base UI popover with focus restoration.
- The compact capsules have descriptive accessible names beyond their abbreviated visible labels.
- Search, switching, streaming, and errors use appropriate live-region behavior.
- Included-source roles are conveyed by text and semantics, not color alone.
- Truncated labels expose their full value through accessible text or tooltip.
- Enter sends, Shift+Enter inserts a newline, and Escape closes transient controls before changing
  workspace presentation.

## Testing

Automated tests will verify:

- An open folder plus one focused file produces exactly one initial content source.
- No arbitrary open-folder documents are appended.
- Small focused documents are bounded and included.
- Large documents produce a heading map and relevant sections within limits.
- Linked references contribute metadata but not target bodies.
- Search results and read ranges respect individual and combined budgets.
- At most three additional ranges are read in one turn.
- Unchanged source bodies are represented through ledger reuse rather than resent.
- Changed sources are resent and update their hashes.
- Provider and folder changes clear the ledger.
- FFF and lexical fallback produce the same bounded result shape.
- Traversal, symlink escape, unsupported files, writes, terminal calls, and unknown tools are denied.
- Model groups contain only values advertised by the active OpenCode instance.
- Unsupported prefixes are omitted from the first-version picker.
- Model switching sends the exact advertised configuration ID/value and trusts returned state.
- Missing, empty, stale, pending, streaming, and failed model states behave safely.
- The context capsule remains one line tall in drawer and workspace layouts.
- The context popover exposes every source role and warning accessibly.
- The attachment and model comboboxes support keyboard filtering and selection.
- Existing streaming, cancellation, reasoning, tools, citations, drawer, and workspace behavior remain
  intact.

Hands-on Electron verification will cover small and large focused documents, linked references,
explicit attachments, cross-document comparisons, missing FFF, folder changes, repeated turns,
all three supported OpenCode model groups available in the local instance, model switching, narrow
drawer and full workspace layouts, light and dark themes, and keyboard-only operation.

## Acceptance Criteria

- Referencing one file never eagerly injects unrelated folder files.
- Additional content is searched and read only when the question requires it.
- The context trace makes injected, searched, read, and reused material understandable.
- Per-source and per-turn limits prevent context from reaching the previous 120 KB behavior.
- Linked references cost metadata only until read.
- The model picker shows only compatible models advertised by the user's active OpenCode instance.
- OpenAI subscription, OpenCode Zen, and OpenCode Go groups appear only when supported locally.
- Model selection persists safely without breaking chat when catalogs change.
- Context controls occupy one compact row and reveal detail on demand.
- New chat controls are shadcn-owned components backed by Base UI.
