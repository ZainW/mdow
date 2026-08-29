# Rationale — candidate c1

## Problem

The GPUI beta renders markdown well but has none of the Electron reader loop's
chrome: no find, no palette, no settings, no session restore, no persistence, and
three latent state bugs the Electron side already exhibits — four independent overlay
booleans (11 of 16 combinations illegal), a stringly `theme: string`, and
`wideMode: bool` living beside `readingWidth: 'standard'|'comfortable'|'wide'` with
nothing keeping them coherent. Constraints from grounding: GPUI is pinned at 0.2.2
with no built-in input widget (the official `examples/input.rs` `EntityInputHandler`
pattern is the sanctioned path); `apps/gpui` has 132 passing tests and a measured
visual contract that must not move; no webview and no mermaid JS runtime; the
existing `pulldown-cmark → DocumentBlock` renderer stays; `gpui-component` stays out
so Mdow's metrics remain authoritative. `MdowApp` is already ~750 lines of shell —
whatever we add must not land there.

## Usage (caller's view)

Written first, in [`usage.md`](usage.md). Summary of the three call sites:

1. **Launch** — `StateStore::open_default().load()` returns `Restored { prefs, session }`,
   infallibly. Window bounds come from the session; `MdowApp::new(prefs, store, …)`
   then `restore_session(session, cx)` tolerantly reopens tabs.
2. **cmd-F** — `toggle_overlay(OverlayKind::Find, …)` builds a `FindOverlay` entity
   and subscribes; the app hears only `ActiveHit` (scroll there) and `Dismissed`
   (close slot). Highlights are _read_ from the overlay at render time, so closing
   find erases them for free.
3. **Palette runs "Theme: Dark"** — `PaletteEvent::Invoked → run_command →
apply_pref(PrefEdit::Theme(Dark))`. `StoredPrefs::apply` mutates and persists in
   one motion; the next frame derives the theme from `(mode, appearance)`.

The sketch in [`sketch.rs`](sketch.rs) is derived from these call sites.

## Shape

Data structures first, because they carry the three invariants this design exists
for:

- **`OverlayHost { open: Option<OpenOverlay> }`** — dual-open overlays are
  unrepresentable because there is one slot. Each `OpenOverlay` bundles the overlay
  entity with the `Subscription` routing its events, so a closed overlay's events
  are also unrepresentable (the subscription drops with the slot). Focus restore is
  the host's job, captured at open. Per encode-lessons-in-structure.
- **`ReaderWidth::{Column(ColumnWidth), Full { returns_to: ColumnWidth }}`** — the
  Electron pair `wideMode`/`readingWidth` collapses into one value. `ToggleFull` is
  an involution; the column you toggle away from is stored _inside the wide state_,
  so it cannot desync and cannot be lost. The wire format still writes Electron's
  two keys; the merge happens exactly once, in `StateStore::load`.
- **`ThemeMode` + `Theme::resolve(mode, appearance)`** — theme is derived every
  frame from two inputs; there is no stored resolved theme to invalidate. Strings
  exist only inside `persist.rs`.
- **`SessionTabs` zipper** (`before / active / after`) — Electron's
  `sessionActiveTabPath` can name a path missing from `sessionTabs`; the zipper
  makes membership structural.
- **`ZoomLevel(u16)`** — constructor-clamped to 60..=200 step 10; garbage on disk
  parses to the nearest legal zoom.

Flow through the signatures: disk → `StateStore::load` (the only parse; total,
field-tolerant, migrating) → typed `Prefs`/`Session` → `MdowApp` funnels
(`apply_pref`, `toggle_overlay`, `active_document_changed`) → derived render inputs
(`Theme::resolve`, `ShellLayout::for_width`, `Prefs::reader_style`,
`overlays.find().map(read matches)`). Per boundary-discipline: validation happens at
the file boundary and nowhere else; everything past `load()` trusts the types.
Business logic is pure functions — `find_in_blocks`, `palette_items`,
`Prefs::apply`, `ThemeMode::scheme` — testable without a window, which matters in a
crate that already leans on 132 headless tests.

Interface depth, judged explicitly:

- `Field` hides the entire `EntityInputHandler` surface (eight UTF-16 methods, IME
  marked text, mouse selection, clipboard) behind `new / text / set_text /
FieldEvent`. Three events out; nothing in the app knows a UTF-16 offset exists.
- `FindOverlay` hides query editing, match recomputation, cursor stability while
  typing, and wrap-around behind two events and a `matches()` accessor. The app
  contributes one funnel call (`retarget_find`) at the document-change point it
  already has.
- `StateStore` hides serde, camelCase wire keys, atomic temp-file renames, and the
  `wideMode` migration behind `load()`/`save()`. `StoredPrefs` goes one step
  further: it owns both the live `Prefs` and the store, so "mutated but not
  persisted" is not a state a caller can construct.
- `MdowApp` stays a router: `run_command` is deliberately a pass-through match —
  that's routing, not hidden behavior, and it keeps every call chain at three files
  or fewer (keybinding → app funnel → module).

What the design deliberately does not do: no overlay stack (Mdow never nests
overlays), no draft/apply settings model (Electron applies instantly; the panel is
display-only and proposes `PrefEdit`s), no debounced persistence (the state file is
<1 KB; synchronous atomic writes on change), no generic widget library, no mermaid
execution (native source card), no webview (HTML converts into the same
`DocumentBlock` vocabulary), and no new visual system — chrome uses the existing
`Theme`/`Metrics` contract plus `ScaleTokens` lifted from Electron's
`data-ui-scale` CSS.

## Synthesis decision

Picked as the arena base. Parent and cross-judge both scored it 35/35. See
[`../SYNTHESIS.md`](../SYNTHESIS.md) for scores, grafts, and rejections.

## Tradeoffs accepted

- We accept that closing an overlay drops its transient state (find query, palette
  selection) in exchange for idempotent opens and zero stale-state reconciliation.
  Reopening find starts blank; Electron behaves the same way.
- We accept one manual funnel obligation — `active_document_changed` must be called
  from tab-mutation sites — in exchange for find retargeting and session persistence
  having exactly one entry point. The funnel replaces the already-scattered
  `clear_reader_transient_state` calls, so the call-site count does not grow.
- We accept synchronous whole-file saves on every pref/session change in exchange
  for crash-safe restore with no debounce machinery. If window-move events prove
  chatty, bounds capture degrades to render-time sampling plus save-on-quit without
  changing any signature.
- We accept `SettingsPanel` holding a display copy of `Prefs` (refreshed through
  one `refresh_settings` call in the apply funnel) in exchange for the panel never
  owning preference state. This is a copy, not a second source of truth: it cannot
  be written back except by emitting a `PrefEdit`.
- We accept keeping Electron's camelCase wire keys (slight Rust-side serde noise)
  in exchange for a state file a future importer could read straight out of
  electron-store.
- We accept a hand-rolled subsequence scorer for the palette instead of a fuzzy
  crate — the catalog is ~20 commands plus 20 recents; dependency weight isn't
  justified.

## Alternatives considered

- **Adopt `gpui-component` for input + modals.** Hides the most implementation
  (mature TextInput, focus trap, modal primitives) but its interface is not small:
  it brings its own theme system, sizing scale, and component vocabulary that would
  sit beside Mdow's measured `Metrics`/`Theme` contract, exposing every render site
  to two design systems. The complexity it hides (one-line text editing) is ~300
  lines from a first-party example; the complexity it adds is permanent. Lost on
  interface depth per dollar, and grounding's refusal stands.
- **Mirror Electron's booleans (faithful port).** Smallest diff and trivially
  parity-correct, but it exposes the coordination burden to every caller: each
  open-overlay site must close three others; each wide-mode write must remember
  `readingWidth` exists. That is the definition of a shallow module — interface as
  large as the implementation. Rejected; it also fails the task's brief directly.
- **One `ChromeController` entity owning overlays + prefs + session.** Deepest
  single interface on paper, but it reproduces the `MdowApp` problem one level
  down: find matching, pref persistence, and palette filtering share no knowledge,
  so grouping them is temporal ("things the chrome does"), not domain ownership —
  the temporal-decomposition red flag inverted. Module seams here follow knowledge:
  text editing (`field`), exclusivity (`overlay`), preference legality (`prefs`),
  wire format (`persist`).
- **Overlays as plain structs rendered by `chrome.rs` functions (no entities).**
  Fewer moving parts and no subscriptions, but every `FieldEvent` then lands in
  `MdowApp`, which must know that find's enter means "advance match" while
  palette's enter means "invoke selected" — internal rules leaking to the caller.
  Entities let each overlay consume its own field events and emit a domain
  vocabulary instead. Rejected on information leakage.
- **Two state files (prefs.json / session.json).** Isolates concerns but doubles
  the atomicity story and invites partial states across files; a single
  Electron-shaped file is one atomic rename. Rejected.

## Open questions and risks

- Recents are pruned of missing files lazily at render (Electron prunes on read).
  Is silent skipping of missing _session tabs_ at restore acceptable too, or should
  a one-line "3 files couldn't be reopened" notice appear?
- Should the GPUI app attempt a first-launch import of the actual electron-store
  JSON (path differs, schema matches)? The wire compatibility makes it nearly free,
  but it may surprise users who want the beta isolated.
- `ScaleTokens` values are lifted from Electron's CSS custom properties; compact is
  today's `Metrics`. Does interface scale also scale sidebar row height and tab
  height (Electron scales buttons/controls only)? Needs a screenshot comparison
  before the tokens freeze.
- `scroll_reader_to_block` depends on recording block y-origins during paint
  (`BlockPositions`). Risk: origins for never-painted blocks below the fold. The
  TODO plans clamped best-effort scroll then a second pass next frame; if that
  proves janky, block heights may need measuring eagerly — flagging now since it's
  the one place the sketch touches reader internals.
- Find highlights require painting per-run backgrounds inside the existing
  `InlineSpan` text runs. If the current text system can't split runs mid-span
  cheaply, highlight granularity may fall back to whole-span tinting — acceptable?

## Next implementation step

Implement `prefs.rs` + `persist.rs` with the round-trip property test
(`decode(encode(x)) == x`) and the `wideMode`/`readingWidth` migration table — it is
the only module with no gpui dependency, it unblocks `MdowApp::new`'s signature
change, and it locks the wire contract everything else writes through.
