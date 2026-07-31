# FFF Document Search Design

## Summary

Mdow Companion will gain on-demand read-only search across the open folder by registering the FFF
MCP server with the active ACP session. This replaces eager inclusion of many folder documents with
targeted retrieval while keeping the focused document and explicit context tags authoritative.

FFF is an optional integration. Focused-document chat must continue to work when it is not
installed.

## Goals

- Let the agent find relevant markdown documents by path and content.
- Keep searches scoped to the folder the user opened in Mdow.
- Stream search activity through the existing thinking and tool blocks.
- Reduce prompt size by retrieving additional documents only when needed.
- Detect and explain missing FFF without making Companion unavailable.
- Preserve a strict read-only permission boundary.

## Non-goals

- Semantic or embedding-based similarity search.
- Searching outside the open folder.
- Searching remote services or the web.
- Installing FFF silently.
- Allowing arbitrary MCP servers or unrestricted OpenCode tools.
- Modifying files.

## Integration Boundary

Mdow will detect `fff-mcp` using explicit candidate paths and the inherited executable path. The
resolved executable must be a regular executable file. Detection results will report available,
missing, or failed status with an actionable detail.

When available, `CompanionService` will pass one stdio MCP server descriptor to `session/new`. Its
working root will be the validated open folder. The descriptor will expose only the FFF server; no
user-supplied shell fragment will be interpolated.

Mdow will also advertise ACP client text-file reading and implement `fs/read_text_file` in the main
process. That handler will accept only canonical, in-folder markdown paths and bounded ranges. It
will not expose directory listing, writing, or terminal capabilities. This gives the agent a way to
read a document selected by FFF without granting general filesystem access.

The integration will allowlist these read-only FFF tool families:

- File and path search.
- Content grep.
- Multi-pattern content grep.

All filesystem write requests, terminal requests, unknown MCP tools, and paths outside the open
folder remain denied.

## Retrieval Strategy

Every turn begins with compact deterministic context:

1. The focused document, when present.
2. Explicit `@file` and `@folder` tags, subject to the existing byte budget.
3. A statement that FFF search is available for additional open-folder documents.

The app will stop automatically appending the first set of files from the entire open folder when
FFF is active. The agent searches only when the question requires broader context, reads the
relevant markdown results through the scoped ACP read handler, and cites the documents it used.

When FFF is unavailable, Mdow retains the current bounded folder-context fallback so users do not
lose existing behavior.

FFF is path and content search, not semantic retrieval. Queries such as exact terms, related
filenames, misspellings, and repeated phrases are appropriate. A future semantic index can be added
behind the same retrieval boundary without changing focused-document behavior.

## User Interface

The Companion setup/status area will show one of:

- `Document search ready` when FFF is connected.
- `Focused document only` when no folder is open.
- `Enable document search` when FFF is missing, with platform-appropriate installation guidance.
- `Document search unavailable` with a concise error when startup fails.

Search calls use the existing tool-card presentation. Cards show the query, running/completed/error
state, and a concise result summary. Large raw result payloads remain collapsed to avoid flooding
the conversation.

No permission dialog is shown for allowlisted read-only searches. A denied or malformed request
appears as an error tool card and a security warning.

## Session and Folder Lifecycle

FFF indexes one root per Companion session. Opening a different folder shuts down the old ACP
session and starts a new one with the new validated root. Closing the folder disables FFF for future
turns but does not erase chat messages.

The search server lifecycle is owned by the ACP provider session. Mdow must terminate the session
and child processes on provider change, folder change, window shutdown, or app exit.

## Security

- Canonicalize the open folder before session creation.
- Reject filesystem roots and the user home directory as implicit broad scopes.
- Resolve search results and every ACP read request against the canonical root.
- Reject traversal and symlink escape.
- Filter retrieved content to supported markdown extensions.
- Bound the bytes returned by each read and the total read context for a turn.
- Never pass secrets or unrelated environment overrides to the FFF process.
- Never auto-approve a tool solely from its display title; approve by validated server and tool
  identity.

## Error Handling

- FFF missing: show setup guidance and use bounded context fallback.
- Index warm-up: keep the tool running and show status without blocking focused-document answers.
- Search timeout: mark the tool failed and let the agent continue with known context.
- Provider or FFF crash: end the tool cleanly, preserve the conversation, and restart on demand.
- Oversized results: paginate or truncate with a visible result count.
- Invalid result path: discard it and record a security warning.

## Testing

Unit and integration tests will verify:

- Detection distinguishes available, missing, and non-executable candidates.
- `session/new` receives the expected FFF MCP descriptor only for a valid open folder.
- ACP initialization advertises read-only text-file support and the read handler returns only
  bounded, in-folder markdown content.
- Folder changes recreate the scoped session and terminate the previous process.
- FFF-active prompts omit eager open-folder stuffing but keep focus and tags.
- FFF-missing prompts retain the bounded fallback.
- Allowlisted read tools are accepted and write, terminal, unknown, traversal, and symlink-escape
  requests are rejected.
- Search tool updates render in order and large payloads remain collapsed.
- Timeouts, server exits, and empty searches settle without leaving Companion streaming.

Hands-on verification will search by filename, exact phrase, misspelling, and cross-document term
in both drawer and fullscreen modes. It will also test no-folder, missing-FFF, folder-switch, and
provider-restart states.

## Acceptance Criteria

- The agent can locate and use relevant markdown documents beyond the focused file.
- Search never escapes the open folder and never modifies files.
- Search activity is visible without overwhelming the chat.
- Missing FFF does not break existing Companion behavior.
- Changing folders cannot leak search context from the previous folder.
