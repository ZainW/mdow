# Mdow audit fixes — implementation plan

**Goal:** Land every fix in the in-depth UI/UX audit (sections §3–§7), with tests where they're meaningful, and re-run the Playwright audit harness to verify.

**Architecture:** All changes are renderer-side under `apps/desktop/src/renderer/src`. Tests sit next to the components they cover (`*.test.tsx` via Vitest + Testing Library). Visual verification is done by re-running `apps/desktop/perf/audit.mjs`, comparing screenshots against the pre-fix run in `/tmp/mdow-audit-before/`.

**Tech stack:** React 19, Tailwind v4, Base UI primitives, Phosphor icons, Vitest + jsdom + Testing Library, Playwright Electron for visual verification.

**Out of scope (deferred — listed in audit §8):** reading-first mode, content-side outline minimap, pinned tabs, global folder search, PDF export, Mermaid lazy import. Each is a multi-day effort; not part of this branch.

---

## Group A — Shared primitives + utilities

Files: create `src/renderer/src/components/EmptyState.tsx`, create `src/renderer/src/lib/path-utils.ts` additions (already exists; extend), tests at `*.test.tsx` next to component / inside existing test file.

**A1. `EmptyState` component**

- Props: `icon: PhosphorIcon`, `title: string`, `hint?: string`, `action?: ReactNode`, `size?: 'sm' | 'md'`.
- Sm variant for sidebar empty rows; md for full-pane (matches `ErrorView` visual).
- Test: renders title and hint, calls action's onClick when clicked, applies size classes.

**A2. `truncatePathMiddle`**

- Signature: `truncatePathMiddle(path: string, maxLen = 56): string`
- Behavior: if `<= maxLen` return as-is; else drop middle segments and join with `…`.
- Test cases: short path passthrough; long path collapses; preserves leading + trailing segments.

---

## Group B — Markdown content polish

Files: `src/renderer/src/assets/styles/markdown.css`, `src/renderer/src/assets/styles/index.css`, `src/renderer/src/components/MarkdownView.tsx`, `src/renderer/src/lib/markdown.ts` (if needed for table wrapper — likely just CSS).

**B1. Default max-width 52rem → 48rem** in `MarkdownView.tsx:294`.

**B2. Inline code padding `0.15em 0.4em` → `0.1em 0.35em`** in `markdown.css`.

**B3. Dark code-block "shadow" → inset highlight**:

```css
.dark .markdown-body .code-block-wrapper {
  box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.04);
}
```

**B4. Task list checkboxes use accent color**:

```css
.markdown-body .task-list-item input[type='checkbox'] {
  accent-color: var(--primary);
}
```

**B5. Heading scale H2 1.4em → 1.5em** for clearer hierarchy.

**B6. Table overflow wrapper.** Add `.markdown-body .table-wrap { overflow-x: auto; border-radius: 8px; }` and wrap `<table>` via a `ComarkRenderer` component override (the `table` HTML element) that emits the wrap. Test: render content with a wide table fixture, assert wrap exists.

**B7. Copy-code-btn fades in on hover**:

```css
.copy-code-btn {
  opacity: 0;
}
.code-block-wrapper:hover .copy-code-btn,
.copy-code-btn:focus-visible,
.copy-code-btn[data-copied='true'] {
  opacity: 1;
}
@media (hover: none) {
  .copy-code-btn {
    opacity: 1;
  }
}
```

Test: open a markdown file with a code block, assert `getComputedStyle(.copy-code-btn).opacity === '0'` at rest. (jsdom won't compute hover state; skip the hover assertion — the Playwright run will catch regressions visually.)

---

## Group C — Welcome view

File: `src/renderer/src/components/WelcomeView.tsx`.

**C1. Centering when no recents.** Drop `grid-cols-1 place-items-center`; switch column children to `items-center text-center` so the hero visually centers.

**C2. Body copy `text-sm` → `text-[15px]`** on the tagline only.

**C3. Drop hint rephrase**: "**Anywhere in this window** — drop a `.md` file." with `<strong>` on "Anywhere in this window".

Test: snapshot WelcomeView with `recents=[]` and with `recents=[…]` and assert layout class differences.

---

## Group D — Sidebar fixes

Files: `Sidebar.tsx`, `RecentsList.tsx`, `FolderTree.tsx`, `index.css`.

**D1. Empty Recents.** Replace `return null` in `RecentsList.tsx:35-37` with `<EmptyState size="sm" icon={Clock} title="No recents yet" hint="Files you open will appear here." />`.

**D2. Empty Folder.** Inside `Sidebar.tsx` Folder mode branch, when `!openFolderPath`, render `<EmptyState size="sm" icon={FolderOpen} title="No folder open" hint="Click the Open Folder icon below, or drag a folder onto this window." />`. `FolderTree.tsx`'s own `return null` stays for when folder is open but tree is empty (rare).

**D3. Outline active row gets the left accent bar.** Reuse `tree-file-active` class on `<a data-active="true">`, OR add a parallel CSS rule. Simpler: extend `.outline-link[data-active='true']` with the same `::before` left bar styling.

**D4. Mode rail = radiogroup.** Wrap the first three rail icons (Recents/Folder/Outline) in a `<div role="radiogroup" aria-label="Sidebar mode">` and switch their `aria-pressed` to `aria-checked`/`role="radio"`. The bottom four (Open File / Open Folder / Settings / Quick Open) stay as `<button>`s in a `role="group" aria-label="Sidebar actions"`.

**D5. Rail focus simpler.** Remove `focus-visible:border-ring` only from the rail icons by adding a CSS override: `.rail-icon-btn { @apply focus-visible:border-transparent; }` — keeps the box-shadow ring as the sole indicator.

**D6. Tests:** RecentsList renders EmptyState when no recents; rail radiogroup has proper roles.

(Skipping D7 contextual sidebar header — out of scope; bigger structural change, deferred to a separate PR if desired.)

---

## Group E — Tab bar

Files: `TabBar.tsx`, `index.css` (if needed), tests.

**E1. Wrap close `<X>` in a 24×24 button.** Today the `<button>` is `size-4` (16). Change to `size-6` (24) with `<X className="size-3" />` centered inside. Visual size of the X stays 12px; hit area grows.

**E2. Drop file-icon weight change.** Always render `FileText` with `weight="regular"`. Active vs inactive expressed via color (text-muted-foreground/80 vs /60 — already in place).

**E3. Tab a11y semantics.** Add `aria-label="Open documents"` to the row; add `aria-setsize` and `aria-posinset` to each tab `<button>`. The row should not be `role="tablist"` because tabs don't have associated `role="tabpanel"`s. Keep `aria-pressed`.

**E4. Context menu items get role="menuitem".** Add `role="menuitem"` to each menu button. Also add ArrowDown/ArrowUp roving focus inside the menu (focus first item on open).

**E5. Context menu kbd hints + isMac branch.** Add right-aligned `<kbd>` for actions with shortcuts: `Close → ⌘W`. Reveal label: `isMac ? 'Reveal in Finder' : 'Show in folder'`.

**E6. Tests:** snapshot tab bar with 3 tabs verifies `aria-setsize=3`, `aria-posinset` 1/2/3; opening context menu via right-click moves focus to first item; ArrowDown wraps and ArrowUp wraps.

---

## Group F — Command palette polish

File: `CommandPalette.tsx`.

**F1. Dir-path opacity `/50` → `/70`.**

**F2. Footer hint bar.**

```tsx
<div className="flex items-center justify-end gap-3 border-t border-border-subtle px-3 py-1.5 text-[10px] text-muted-foreground/70">
  <span>
    <Kbd>↵</Kbd> open
  </span>
  <span>
    <Kbd>esc</Kbd> dismiss
  </span>
</div>
```

Place inside the dialog, after `<CommandList />`.

(No test — the cmdk internals + dialog mount are complex; visual audit is the verification.)

---

## Group G — Search bar polish

File: `SearchBar.tsx`.

**G1. Inline kbd hint.** Place a small `<span>` below the input or to the right of the status counter, showing "↵ next · ⇧↵ prev · esc". Use `text-[10px] text-muted-foreground/60`. Keep `tabular-nums` for the counter.

---

## Group H — Settings dialog

File: `SettingsDialog.tsx`, tests in existing `SettingsDialog.test.ts`.

**H1. Active state contrast.** Bump `border-foreground/15 bg-accent/5 ring-1 ring-foreground/5` → `border-foreground/25 bg-accent/10 ring-1 ring-foreground/10` (light) and use `dark:` modifiers for slightly stronger contrast in dark.

**H2. Add a subtle corner dot for active font-tile.** A 6px `bg-primary` dot positioned `absolute top-1.5 right-1.5` when `active`.

**H3. Theme buttons → radiogroup.** Replace `<fieldset>` semantics with `role="radiogroup" aria-label="Theme"` on the wrapper; `role="radio"` and `aria-checked` on each button (replacing `aria-pressed`).

**H4. Slider thumb dark mode.** Add `dark:bg-foreground/90 dark:border-foreground/30` to the Slider thumb class in `ui/slider.tsx`.

**H5. Close X position `top-2 right-2` → `top-3 right-3`** in `ui/dialog.tsx` (affects all dialogs).

**H6. Tests:** ThemeRadiogroup has correct role/aria-checked; activating one updates store.

---

## Group I — Shortcuts dialog

File: `ShortcutsDialog.tsx`.

**I1. Kbd typography**: `text-xs` → `text-[11px]`.

**I2. Section grouping.** Restructure data into groups (`Files`, `Navigation`, `View`). Render an extra-small section label between groups.

---

## Group J — Zoom indicator

File: `ZoomIndicator.tsx`.

**J1. Always reserve space for reset button.** Render the reset button always, with `opacity-0 pointer-events-none` when `zoomLevel === 100`. This stops the width-jump on first zoom action.

Test: with `zoomLevel=100`, reset button has `opacity: 0`; with `zoomLevel=120`, it has `opacity: 1`.

---

## Group K — Error view

File: `ErrorView.tsx`, uses `truncatePathMiddle` from Group A.

**K1. Show truncated path** instead of just the basename: `{truncatePathMiddle(error.path, 48)}`.

---

## Group L — Breadcrumb interactivity

File: `DocumentBreadcrumb.tsx`.

**L1. Drop trailing CaretRight on the final segment** (the one immediately before the filename), since the filename itself is the visual terminus. Replace the loop with a generator that conditionally renders the chevron only between items.

**L2. Make parent segments clickable.** Each segment becomes a button. Clicking it sets sidebar mode to Folder and selects that folder. Since today's `setOpenFolder` requires reading the folder tree, simpler scope: clicking a segment opens that absolute path via `window.api.openFolderDialog`-equivalent path-set IPC. Simpler still: click reveals that folder in Finder via `window.api.showInFolder`.

Pragmatic choice for this PR: **clicking a parent segment Reveals it in Finder** (matching the filename's behavior on click). That's a small win that doesn't require new IPC. The deeper "select this folder in the sidebar tree" needs IPC support and is deferred.

Test: renders 3 segments with 2 chevrons, click on the first segment calls `window.api.showInFolder` with the accumulated path.

---

## Group M — Audit harness updates

File: `apps/desktop/perf/audit.mjs` (recreate from session memory; the earlier file was uncommitted).

**M1. Replace `Cmd+K`, `Cmd+B`, `Cmd+F`, `Cmd+W` keystrokes with IPC menu events.** The app's IPC already exposes `menu:open-file`, etc., via `window.api.onMenu*`. Send those directly through `app.evaluate(({BrowserWindow}) => win.webContents.send('menu:command-palette'))` — verify the exact channel names by reading `apps/desktop/src/main/ipc.ts` and `preload/index.ts`.

**M2. Also restore the file:opened simulation.**

**M3. Add an after-screenshot pass** that programmatically attaches a tab error to the _active_ tab so the error view is actually captured.

---

## Group N — Final verification

**N1. Run `pnpm typecheck`, `pnpm lint`, `pnpm fmt:check`, `pnpm test`** — all four must pass.

**N2. Build app and re-run the audit harness.** Save the new screenshots to `/tmp/mdow-audit/`. Spot-check each frame for: visible fixes, no regressions, consistent active states, focus rings intact.

**N3. Commit changes incrementally** (per Group) so the diff is reviewable.

**N4. Open PR or hand off** — pending user direction.

---

## Self-review

| Audit reference                                                    | Plan task                           |
| ------------------------------------------------------------------ | ----------------------------------- |
| §3.1 Empty states                                                  | A1, D1, D2                          |
| §3.2 Active state contrast (dark)                                  | H1, H2, D3 (outline bar)            |
| §3.3 Sidebar header earning row                                    | _deferred — D7 dropped_             |
| §3.4 Breadcrumb segments navigable                                 | L1, L2 (scoped to Reveal in Finder) |
| §3.5 Code copy quieter                                             | B7                                  |
| §3.6 Tab close hit area                                            | E1                                  |
| §4.1 Welcome centering / hint                                      | C1, C3                              |
| §4.2 Markdown line length, headings, code, tables, task lists      | B1, B2, B3, B4, B5, B6              |
| §4.3 Outline active bar, outline indentation                       | D3                                  |
| §4.4 Tab file icon, context menu kbd, isMac branch                 | E2, E5                              |
| §4.5 Cmd palette dir path opacity, kbd footer                      | F1, F2                              |
| §4.6 Search kbd hint                                               | G1                                  |
| §4.7 Zoom indicator width jump                                     | J1                                  |
| §4.8 Settings active contrast, radiogroup, slider dark, X position | H1–H5                               |
| §4.9 Shortcuts kbd, sections                                       | I1, I2                              |
| §4.10 Error path truncation                                        | K1                                  |
| §5.1 Tab a11y                                                      | E3                                  |
| §5.2 Sidebar mode radiogroup                                       | D4                                  |
| §5.3 Rail focus border-ring                                        | D5                                  |
| §5.5 Tab context menu role menuitem                                | E4                                  |

Coverage looks complete except the deferred bigger bets (§3.3 contextual header, §8 structural). Calling those out at the bottom of the PR description will be sufficient.
