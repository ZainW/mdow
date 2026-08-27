# Arena synthesis — GPUI Electron-parity chrome

Base: **c1**. No dropouts. Parent and [cross-judge](47c7a09f-08dc-444e-8132-e3cdfed3b22b) picked the same candidate.

## Parent scores (out of 35)

Scored after reading every `usage.md`, `sketch.rs`, and `rationale.md` end to end, then the judge file.

| #   | Criterion                                                                 | c1     | c2     | c3     | c4     |
| --- | ------------------------------------------------------------------------- | ------ | ------ | ------ | ------ |
| 1   | Dual-open overlays unrepresentable                                        | 5      | 5      | 2      | 3      |
| 2   | Persistence: boundary parse, total/field-tolerant, converges under retry  | 5      | 4      | 2      | 3      |
| 3   | DocumentBlock renderer + 132 tests remain the markdown path               | 5      | 5      | 5      | 2      |
| 4   | Text input is a deep module (no IME/UTF-16/EntityInputHandler leakage)    | 5      | 2      | 4      | 4      |
| 5   | Typed prefs/sidebar; no stringly theme; wideMode/readingWidth can't drift | 5      | 4      | 2      | 3      |
| 6   | Usage written first; sketch matches the call sites                        | 5      | 4      | 4      | 5      |
| 7   | Maintainer can add one chrome feature; small surface hiding policy        | 5      | 3      | 3      | 4      |
|     | **Total**                                                                 | **35** | **27** | **22** | **24** |

Judge totals: c1 35, c2 27, c4 24, c3 22. Same ranking, same numbers.

Agreement on the base confirms the pick. Criterion 7 is the tiebreaker the arena skill names; c1 wins it by making a new overlay one variant plus one entity, and a new pref one `PrefEdit` plus one wire field.

## Why c1

`OverlayHost { open: Option<OpenOverlay> }` makes dual-open, and events from a closed overlay, unrepresentable. `ReaderWidth::{Column, Full { returns_to }}` is the only design that keeps cmd-shift-W as an involution and still writes Electron's two keys. `StateStore::load` is the only parse; `StoredPrefs` makes "mutated but not persisted" unconstructible. `Field` hides the eight-method `EntityInputHandler` surface. Chrome attaches beside `AppModel` and mutates tabs through existing `MdowApp` methods, which is what HOW.md required.

## Grafts

Folded into the implementation plan, not pasted:

- **c2** — `RequestId` on session restore and reload so a user open racing restore cannot lose. Watch reconciliation as a full desired set. The reducer test list (exclusivity, stale rejection, session derivation) as the test plan for c1's pure functions.
- **c3** — HTML sanitizer contract for `html_to_blocks`: strip `script`/`iframe`/`object`/`embed` and `on*` attributes, rewrite relative `src`/`href` against the document parent. Version-pin dossier stays a recorded fallback if `Field` exceeds the ~300-line estimate. Comfortable/wide pixels checked against Electron CSS before freezing (`48rem`/`56rem`/`68rem` at 16px = 768/896/1088).
- **c4** — Find reads the same string the glyphs came from (`InlineStyleRange` / painted runs, not `plain_text`). Match-cursor preservation on retarget/reload. Seed the find field from the current selection. Byte-identical reload short-circuit.

## Rejections

- **c2's `AppCore` replacement of `AppModel`/`TabSet`** — HOW.md says chrome must call the existing methods so reader-transient cleanup stays consistent. The reducer is a rearchitecture of working, tested structure. Field-resync-after-dispatch is an IME hazard.
- **c3's `gpui-component` 0.5.1 adoption** — overlay exclusivity split across two owners, stub persistence, stringly fonts, persisted `sidebar_open`, delegate copies that must be rebuilt. The crate hides one-line input; it adds a second theme system. Keep the pin research as a fallback note.
- **c4's renderer rewrite** — justified by selection/copy, which the done predicate does not include. It deletes the 132-test `DocumentBlock` path. File it for a later selection project.
- **c4's `modal` + `find` as two fields** — settings-over-find stays representable. Grounding asked for one exclusive slot that includes search.
- **c3/c4 persisting `sidebar_open`** — Electron resets it to open on launch.
- **c2 dropping wide mode** — avoids drift by deleting a shipped behavior.

## First implementation cut

`prefs.rs` + `session.rs` + `persist.rs`. No gpui types in that cut. Round-trip and field-tolerant load tests lock the wire contract before `MdowApp` changes signature.
