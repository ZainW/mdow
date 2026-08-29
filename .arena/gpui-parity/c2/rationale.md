## Problem

GPUI already has a measured native shell and a working `DocumentBlock` reader, but its application
behavior is spread across mutable `MdowApp` fields, `AppModel` methods, GPUI callbacks, watcher
tasks, and direct external calls. Electron parity adds tabs, recents, three sidebar modes, four
mutually exclusive overlays, settings, session restore, live reload, and persistence. Adding each
as another field plus callback would create multiple writers and duplicated sequencing rules:
opening one document must update the active tab, recent list, session snapshot, watch set, error
surface, and reader transients as one operation. The design must preserve the existing markdown
pipeline, keep GPUI 0.2.2 input mechanics at the edge, and prohibit webviews.

## Usage (caller's view)

The detailed caller contract is in `usage.md`. The GPUI root owns one `AppCore`, translates every
menu/click/key/field event into `AppCommand`, calls `dispatch`, and renders only borrowed views from
`core.view()`. It submits returned `AppEffect` values to a shell executor. Dialog and filesystem
results, restored state, and watcher notifications return through `dispatch` as typed completion
commands.

Three representative flows define the API:

1. `OpenFile` dispatches `OpenRequested(FilePicker)`; the shell executes `ShowOpenDialog`, then
   `ResolveOpenTargets`; one `OpenTargetsResolved` transition updates tabs, active selection,
   recents, errors, session-derived persistence, and desired watches.
2. A GPUI `EntityInputHandler` dispatches `OverlayTextChanged`; caret and IME stay local to the
   field, while the query text, matches, and selected result come back from `OverlayView`.
3. A watch notification dispatches `DocumentChanged`; the shell performs `ReloadDocument`; the
   matching completion replaces only that tab's prepared document or records a reload error while
   preserving the last good document.

The public mutation surface is therefore one method, `dispatch(AppCommand)`, rather than a set of
order-dependent setters.

## Shape

`AppCore` contains one private, normalized `AppState`. `DocumentsState` owns tab order and active
identity; `ChromeState` owns sidebar and overlay state; `AppSettings`, `RecentsState`, workspace,
system appearance, and pending request tokens are siblings folded in the same transition. Session
state is not another live field: `SessionSnapshot` is derived from `DocumentsState` when producing
a persistence effect. This gives every invariant one owner, per `encode-lessons-in-structure`.

`OverlayState` is a sum type (`Closed`, `Search`, `CommandPalette`, `Settings`, `Shortcuts`), making
the Electron-style dual-open states unrepresentable. `SearchState` and `PaletteState` contain their
authoritative query and selection. `DocumentId` and `WorkspaceId` wrap canonical paths and can only
be produced by loaders or validated restoration. Setting enums encode theme, reading width, and
interface scale; validated newtypes bound zoom and font identifiers. Boundary adapters reject or
migrate malformed disk data before constructing `RestoredState`, per `boundary-discipline`.

The reducer has the literal shape
`reduce(AppState, AppCommand) -> (AppState, Vec<AppEffect>)`. It may classify links, calculate find
matches, fold open results, choose errors, and derive snapshots, but it cannot open a dialog, read a
file, mutate a watcher, serialize data, open a URL, or call GPUI. Those operations are data in
`AppEffect`. Effects return validated domain payloads—`LoadedDocument`, `LoadedWorkspace`, or
`RestoredState`—rather than storage or framework types, preventing information leakage.

Request IDs make asynchronous transitions explicit. A newer unresolved open replaces the older
pending open, so late picker/load results cannot override the latest user intent. Reload requests
are tracked independently per `DocumentId`, allowing unrelated tabs to reload concurrently while
discarding stale results for the same tab. Watch reconciliation carries the complete desired
document set, making retries idempotent. Persistence effects carry monotonically increasing
revisions and complete `DurableSnapshot` values; one serial persistence worker drops older queued
writes, so disk state cannot move backward, per `make-operations-idempotent`.

The markdown branch stores `Arc<PreparedDocument>` and hands it back through
`DocumentContentView::Markdown`; the current `DocumentBlock` parser/preparation/renderer remains
the reader implementation. Native HTML is a separate sanitized document payload and renderer, not
a webview. Strikethrough, alerts, footnotes, and Mermaid cards remain changes inside the document
pipeline rather than new chrome state or reducer effects.

GPUI-specific focus handles, scroll handles, hover state, drag coordinates, caret/selection/IME
composition, and short-lived copy feedback remain in the shell because they are render mechanics,
not product truth. The shell may not infer product state from them. A field must synchronize its
committed text from `OverlayView` after dispatch.

This is a deep interface: callers learn one mutation operation and a small set of read-only
projections, while the reducer hides atomic cross-feature updates, stale-result policy, overlay
exclusivity, recent-list policy, session derivation, persistence scheduling, and watch
reconciliation. The command enum is intentionally rich because it names user and system events,
but callers never coordinate internal stages or select implementation strategies, per
`minimize-reader-load`. The only exposed complexity is effects the host must actually execute.

## Synthesis decision

Use the command/reducer core as the candidate base because the parity work is dominated by
cross-feature invariants, not by independent widgets. Keep the existing `PreparedDocument` and
`DocumentBlock` reader as an opaque document payload, and adapt GPUI input/focus/scroll facilities
as shell-owned mechanics. Reject any graft that adds public setters, lets an effect mutate
`AppState`, persists from subscriptions outside the transition, or recreates overlays as
independent booleans; each would defeat the single-writer property this candidate is designed to
establish.

## Tradeoffs accepted

- We accept a broad `AppCommand` enum in exchange for one explicit, exhaustive mutation protocol.
- We accept reducer helpers that understand several chrome domains in exchange for atomic updates
  where tabs, recents, session, errors, and watches cannot drift.
- We accept immutable full persistence snapshots and watch sets in exchange for idempotent,
  order-safe effect execution; these sets are small for a reader application.
- We accept one active open pipeline, where a newer open supersedes an unresolved older one, in
  exchange for deterministic latest-intent behavior.
- We accept GPUI field text synchronization after each transition in exchange for keeping IME
  mechanics local without creating a second source of product truth.
- We accept that `AppState` is not a general extension point in exchange for preventing production
  code from bypassing reducer invariants.

## Alternatives considered

- **Mutable `MdowApp` feature fields and setters.** This is a shallow interface: callers would need
  to coordinate tab, recent, session, watcher, and error setters in the right order while the
  methods hide almost no policy. It loses despite requiring fewer initial files.
- **Independent Zustand-like Rust slices or feature controllers.** Each slice could hide its local
  mutation, but open/close/restore invariants would leak across tab, recent, persistence, sidebar,
  and watcher boundaries. Callers would still orchestrate a temporal chain or an event bus with
  implicit ordering, giving a deeper call graph and a larger effective interface.
- **Actor per subsystem with shared event publication.** Actors hide concurrency mechanics well,
  but the read boundary would have to merge asynchronously changing tab, recent, session, and
  settings views. The product has one UI writer, so actors expose consistency and reconciliation
  complexity that a pure fold eliminates; workers are better limited to effect execution.
- **Append-only event sourcing.** A replayable log would hide persistence ordering but expose event
  versioning, migrations, compaction, and failure recovery to a local desktop reader that only
  needs a current snapshot. Commands plus revisioned snapshots retain deterministic tests without
  making the command history a product database.

## Open questions and risks

- Should a user open that arrives during session restoration replace the restore batch entirely,
  or should already restored tabs remain while only the pending remainder is cancelled?
- Should failed session entries be silently pruned from the next snapshot, or shown once in a
  non-blocking restoration summary?
- Which font identifiers are supported natively at launch, and should unavailable persisted fonts
  fall back silently or produce a settings warning?
- What zoom bounds preserve the existing measured chrome and minimum readable layout?
- Should opening a workspace add its first document automatically, or retain the current behavior
  of populating only the folder sidebar?
- Does native HTML participate in find and outline using the same plain-text/range contract, and
  what sanitizer subset is required before its payload can be treated as validated?

## Next implementation step

Build `core/state.rs` and reducer tests for overlay exclusivity, atomic open/close bookkeeping,
stale request rejection, session derivation, and revisioned effects before wiring any GPUI callback.
