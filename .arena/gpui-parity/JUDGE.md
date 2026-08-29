# Arena cross-judge verdict — GPUI Electron-parity chrome

Judge: independent readonly pass over c1–c4 against the Phase-A rubric and the design red flags.
All four candidates produced complete output; no dropouts.

## Score table

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

## Per-candidate assessment

### c1 — OverlayHost + typed Prefs/Session + Field entity + persist boundary

**Deep:** This is the only candidate that nails all three invariants the grounding names and does it with the smallest structural footprint. `OverlayHost { open: Option<OpenOverlay> }` makes dual-open unrepresentable _and_ bundles the event `Subscription` into the slot so a closed overlay's events are also unrepresentable — that second invariant is one nobody else caught. `ReaderWidth::{Column(w), Full { returns_to: w }}` is the best answer in the set to the wideMode/readingWidth drift: the toggle is an involution and the return column lives inside the wide state, so it cannot be lost or desync. `StateStore::load` is total and field-tolerant with the migration table written out; `StoredPrefs` owning both the live `Prefs` and the store makes "mutated but not persisted" unconstructible. The `Field` module hides the full eight-method `EntityInputHandler` surface behind three events. Business logic (`find_in_blocks`, `palette_items`, `Prefs::apply`) is pure and headless-testable, which fits a crate that already leans on 132 headless tests. It also respects the HOW.md attach points exactly: chrome state sits beside `AppModel`, tab mutations go through existing `MdowApp` methods, highlights extend the reader's inline layout.

**Fails:** One real correctness bug: `find_in_blocks` matches over "the existing `ParsedDocument::plain_text` traversal, per block" — HOW.md explicitly flags that `plain_text` maps `SoftBreak` to `"\n"` while the painted layout renders a space, so byte offsets from that extraction drift from the rendered `InlineStyleRange` runs the highlights attach to. The candidate's own open-questions section half-senses the risk but doesn't connect it to the gotcha. Second, `active_document_changed` is a manual funnel obligation — a discipline, not a structure — honestly disclosed as a tradeoff. Third, session restore has no stale-request story: a user open racing the sequential restore is unhandled. Red-flag screen: `run_command` is a pass-through match, defended as routing (acceptable — it adds dispatch policy, not a shape-preserving forward); `SettingsPanel` holds a display copy of `Prefs`, mitigated by the single `refresh_settings` funnel. No shallow modules, no temporal decomposition.

### c2 — command/reducer core with effects

**Deep:** The reducer itself is genuinely deep — one `dispatch` hides atomic cross-feature folds (tabs + recents + session + watches + errors in one transition), and the async discipline is the best in the arena: `RequestId` on every completion so stale dialog/load/reload results are dropped, watch reconciliation as a full desired set (idempotent), persistence as revisioned snapshots through a serial worker so disk state cannot move backward. `OverlayState` as a sum type is clean. `SessionSnapshot` derived from `DocumentsState` rather than synced is exactly right.

**Fails:** It rebuilds the app instead of extending it. `AppState`/`DocumentsState` replace `AppModel`/`TabSet` and the existing `MdowApp` methods wholesale — HOW.md's attach points say chrome must go _through_ those methods so the reader-transient cleanup invariants hold, and this candidate discards them along with the tab/watcher behavior they encode. Text input is its biggest rubric failure: there is no input module at all, just a prose rule that the shell owns an `EntityInputHandler` and must "re-synchronize from the query view after every transition" — so the shell author learns IME/UTF-16 anyway, and resyncing field text from the core after each dispatch is caller-visible coordination (shallow at that edge) and an active IME hazard during composition. The read side is a large family of `AppView`/`TabsView`/`TabView`/`SettingsView` wrappers whose methods are pure getters — pass-through bordering on ceremony — and the ~30-variant `AppCommand` plus reducer arms plus view projection plus shell wiring means every one-line chrome feature touches four surfaces. Wide mode simply disappears (no toggle, only `ReadingWidth`), which avoids drift by dropping a feature. No temp+rename is named; the revision scheme substitutes for convergence but atomicity of a single write is unstated.

### c3 — adopt gpui-component 0.5.1

**Deep:** The dependency due-diligence is the best-grounded document in the arena: the crates.io-0.5.1-on-gpui-0.2.2 pin versus git-HEAD `gpui_platform` distinction is real and carefully argued, the 108-field `ThemeColor` projection is written out so no Longbridge default can leak, and `TextView::html` is an honest webview-free answer for `.html` files. `open::classify` and `OpenDocument::{Markdown, Html}` are clean. If the arena had concluded hand-rolling input is infeasible, this is the adoption you'd want.

**Fails:** It scores lowest because the state discipline the rubric weights most is weakest here. Overlay exclusivity is split across two owners: Mdow's `Overlay { kind: Option<OverlayKind> }` and the crate's Root dialog layer — `dismiss` carries a TODO "close an open Root dialog if the crate tracks one," and the crate's own Escape handling can close a dialog without updating `kind`, so the illegal state (kind says Settings, no dialog open — or vice versa) is representable by desync. Persistence is a stub: `Session::load`/`save` unimplemented with no field tolerance, no atomic write, `zoom_level: f32` unclamped with a suspicious `0.0` default, `content_font: String` stringly. `sidebar_open` sits in persisted `Preferences`, violating the HOW.md gotcha that Electron deliberately does not persist it. `ReadingWidth::Wide => None` conflates Electron's wide column (1088px) with full-bleed wideMode — drift avoided by losing fidelity. The sidebar `ListDelegate`s copy `WorkspaceTree` rows/recents/headings into delegate state that must be manually rebuilt on model change — sync-not-derive, a standing staleness trap and mild temporal decomposition. `theme_bridge::sync` writing a process Global during `render` every frame is an information-leakage risk with an open question ("this frame or next?") the candidate itself flags.

### c4 — the reader engine (atoms rewrite)

**Deep:** The best pure module design in the arena. `Reader` — ten methods, three events — hides parsing, incremental reparse, highlighting, shaping, geometry, hit-testing, selection algebra, clipboard formatting, and match ordering; that's the textbook deep module. `AtomId` vs `AtomIndex` as separate types (persistence-valid vs order-valid) is a genuinely good invariant; `AtomBuilder` enforcing GPUI's run-tiling by construction, semantic runs with paint-time color (theme switch invalidates nothing expensive), match preservation across live reload, and the `FrameLog` write-phase/read-phase fence are all ideas worth keeping somewhere. Usage-first discipline is excellent.

**Fails:** It answers a different question than the one asked. It deletes `document.rs`, `syntax.rs`, and `ui/reader.rs` — the rubric says that path scores low unless the rewrite is _proven required for the reader loop_, and the proof offered rests on cross-block selection and copy, which are not in the grounding's done predicate. Find-in-document — which is in scope — is achievable inside the existing renderer via the exact attach point HOW.md names (c1 demonstrates it). Claiming the 132 tests survive because "the spacing table moves intact" is not the same as the tests remaining green: they assert against `DocumentBlock` structures that cease to exist. Beyond the scope violation: chrome keeps `modal: Option<Modal>` _plus_ `find: Option<FindBar>` — the coexistence argument (find is non-modal) has merit, but the grounding explicitly asked for one exclusive slot including search, so settings-over-find remains representable; `Preferences` keeps `wide_mode: bool` _beside_ `reading_width` — the exact Electron drift pair criterion 5 exists to kill, which c1 already solved; `Session` persists `sidebar_open` against the HOW.md gotcha; `Store::load` is total but whole-file, not field-tolerant.

## Recommended base: c1

c1 is the only candidate that satisfies all seven criteria simultaneously, and it wins criterion 7 — the tiebreaker the arena skill names — decisively: a maintainer adds a chrome feature by adding an `OverlayKind` variant plus an entity plus an event arm, or a `PrefEdit` variant plus a wire field, and the compiler finds every site. Its module seams follow knowledge ownership (text editing / exclusivity / preference legality / wire format), not execution order. It is also the candidate that treats the existing crate as the grounding demands: additive `DocumentBlock` variants, highlights through the reader's own inline layout, tab mutations through existing `MdowApp` methods, 132 tests untouched. c2 centralizes invariants beautifully but at the cost of replacing working, tested structure and leaving text input — a hard, explicit criterion — unsolved. c3 and c4 each relitigate a constraint the grounding set (the gpui-component refusal; the renderer's permanence) without clearing the bar the grounding set for revisiting it.

## Grafts from the losers

**From c2:**

- **`RequestId` stale-completion discipline** for c1's `restore_session` and reload paths. c1's tolerant restore has no answer for a user open racing the sequential session restore; c2's "newer intent supersedes older pending open" rule is small and drops straight into the restore funnel.
- **Watch reconciliation as a full desired set** rather than imperative watch/unwatch deltas — makes watcher retries idempotent and is a one-signature change to how c1's funnel talks to `FileWatcher`.
- The **reducer-style test list** (overlay exclusivity, stale rejection, session derivation) as the test plan for c1's pure functions, even without the reducer.

**From c3:**

- The **HTML sanitize/rewrite specifics** for c1's `html_to_blocks`: strip `script`/`iframe`/`object`/`embed` and `on*` attributes, rewrite relative `src`/`href` against the document parent. c1 names the converter but not the sanitizer contract; c3 wrote it.
- The **version-pin dossier** (0.5.1-on-0.2.2 vs git HEAD) as a recorded fallback: if the hand-rolled `Field` proves harder than the ~300-line estimate, the adoption path is already researched. Keep it as a note, not a dependency.
- The reminder to **verify Electron's actual comfortable/wide pixel values** against the CSS before freezing `ColumnWidth::max_width`.

**From c4:**

- **The find-coordinate principle: "find reads the same string the glyphs came from."** This fixes c1's one real bug — `find_in_blocks` over `plain_text` will drift at soft breaks (HOW.md's `"\n"`-vs-space gotcha). Compute hits against the reader's painted inline text/`InlineStyleRange` runs, not a separate extraction.
- **Match-cursor preservation on retarget/reload** (`Matches::recompute` keeping the current hit when its text survives) — folds into c1's `FindOverlay::retarget` so a watcher save doesn't throw the user from 17-of-43 back to 1.
- **Seeding the find field from the current selection** at open (`open_find(seed, …)`) — cheap Electron-parity polish.
- The **`unchanged: true` short-circuit** for byte-identical reloads, applicable to c1's existing reload path.

## Rejections

- **c2's core replacement of `AppModel`/`MdowApp`** — rejected. It violates the HOW.md attach points (chrome must call the existing methods so reader-transient cleanup invariants hold), discards tested tab/watcher behavior, and its field-resync-after-dispatch pattern is an IME hazard that directly fails criterion 4. The invariants c2 protects are real; c1 protects the same ones with types instead of a rearchitecture.
- **c3's gpui-component adoption** — rejected for this run. The grounding allows revisiting the refusal only if proven; the proof covers version compatibility, not state discipline, and the candidate's own sketch shows the costs: overlay exclusivity split across two owners, delegate copies that must be manually rebuilt, a theme Global written during render with an unanswered frame-lag question, and the weakest persistence in the set. The complexity the crate hides (one-line text editing) is available from a first-party example; the complexity it adds is permanent.
- **c4's renderer rewrite** — rejected on criterion 3. The rewrite is justified by selection/copy, which the done predicate does not include; everything in scope (find, outline jump, GFM gaps, HTML, mermaid card) is reachable additively inside `DocumentBlock`. If cross-block selection later becomes a requirement, c4 is the design document for that project — file it, don't merge it.
- **c4's two-layer chrome (`modal` + `find` as separate fields)** — rejected. The non-modal argument is coherent but the grounding explicitly asked for one exclusive slot covering search; settings-over-find stays representable in c4's shape.
- **c3/c4 persisting `sidebar_open`** — rejected. Electron deliberately resets it to open on launch (HOW.md gotcha); parity means not over-persisting.
- **c2's dropping of wide mode entirely** — rejected. It avoids the drift by deleting a shipped behavior; c1's `ReaderWidth::Full { returns_to }` keeps the behavior and still makes the drift unrepresentable.
