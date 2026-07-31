# Companion Merge Hardening Design

## Context

The packaged `feat/ai-companion-chat` build passes the existing automated suite, but a real
OpenCode ACP session exposes protocol, streaming, accessibility, and responsive-layout defects.
This design covers the approved fixes from the July 28 merge-readiness audit.

## Considered approaches

### 1. Harden the current architecture and progressively disclose activity

Keep the existing ACP client, companion service, Zustand slice, and AI element components. Correct
the protocol boundary, add a small stream sanitizer, introduce an immediate local request state,
and present thinking/tools as compact disclosures.

This is the recommended approach because it fixes the root causes without replacing the feature
immediately before release.

### 2. Replace the custom ACP client with an SDK

An SDK could reduce long-term protocol maintenance, but it would expand the change surface,
dependency footprint, and release risk. It is not appropriate for this hardening pass.

### 3. Suppress visible errors and patch layout only

Filtering stderr and widening the panel would improve screenshots but leave protocol
non-compliance, citation loss, request races, and accessibility failures intact. This approach is
rejected.

## Architecture

### ACP boundary

- Send `initialize`, then proceed directly to `session/new`; do not send
  `notifications/initialized`.
- Advertise only capabilities Mdow actually implements.
- Source the client version from `apps/desktop/package.json`.
- Apply a finite timeout to JSON-RPC requests and remove timed-out entries from the pending map.
- Keep subprocess stderr out of the conversation. Unexpected process exits and rejected RPC
  requests continue to surface as explicit errors.

### Stream and citation handling

`CompanionService` owns a per-prompt citation stream buffer. Each text delta is appended to the
buffer, known source IDs are removed and emitted once as structured citation updates, and only
safe visible text is forwarded. A suffix that may be the beginning of a source ID is retained for
the next chunk. At prompt completion the remaining visible text is flushed.

Cancellation emits a distinct `cancelled` update. A prompt completion received after cancellation
must not turn the message back into complete.

### Renderer state

The store gains `beginCompanionRequest()` and `cancelCompanionRequest()`. The composer calls
`beginCompanionRequest()` synchronously before invoking IPC, closing the duplicate-send window and
creating the assistant activity row immediately.

Thinking and tool activity remain separate from answer text, but both render as compact collapsed
rows. The currently active row communicates progress through its label, icon, and accessible
status; detailed content opens on demand. Answer markdown is derived synchronously so a stream
delta cannot produce a blank intermediate frame.

### Composer accessibility

The mention picker follows combobox/listbox behavior:

- `ArrowDown` and `ArrowUp` change the active suggestion.
- `Enter` selects the active suggestion while the popup is open.
- `Escape` closes the popup.
- The textarea exposes `aria-controls`, `aria-expanded`, and `aria-activedescendant`.
- The selected option is visually and semantically identified.

### Responsive behavior

At wide widths Companion remains a 20rem side panel. Below the desktop breakpoint it becomes a
fixed overlay over the document instead of consuming document width. The expanded dialog overrides
the base responsive maximum width and uses the available viewport safely.

## Error handling

- Request timeouts include the ACP method name and terminate only the affected request.
- IPC rejection marks the pending assistant message as an error and re-enables Send.
- Cancellation marks the active assistant message cancelled and re-enables Send immediately.
- Agent diagnostic stderr is not treated as user-facing answer content.

## Verification

- ACP client tests assert the exact initialization payload, absence of the unsupported
  notification, truthful capabilities, package version, timeout cleanup, and cancellation
  notification.
- Service tests cover citations split across chunks, raw-ID removal, citation de-duplication,
  final-buffer flushing, and cancellation.
- Store tests cover immediate request state and terminal cancelled state.
- Component tests cover compact thinking/tools, synchronous markdown, mention keyboard behavior,
  duplicate-send prevention, and responsive/fullscreen classes.
- Full repository verification and a signed macOS distribution build run before the packaged-app
  Computer Use audit.
- The packaged audit covers initial request, thinking, tools, answer streaming, source chips,
  source opening, cancellation, keyboard mentions, expanded mode, dark mode, and narrow layout.
