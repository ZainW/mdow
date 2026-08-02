# Companion Workspace Presentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Companion's expanded modal with a true full-window workspace while preserving the existing drawer and one shared conversation.

**Architecture:** Replace the two Companion presentation booleans with one discriminated presentation state. `App.tsx` selects either the reader shell or a workspace component below the native titlebar, while `CompanionBody` remains the shared conversation surface for drawer and workspace layouts.

**Tech Stack:** React 19, Zustand, TypeScript, Tailwind CSS v4, Vitest, Testing Library.

## Global Constraints

- Keep the native titlebar visible in workspace mode.
- Do not render a dialog, backdrop, sidebar, document tabs, breadcrumb, reader, update banner, or drawer behind workspace mode.
- Preserve the same in-memory messages, composer draft during mounted presentation transitions, provider state, tags, and streaming state.
- `Back to document` returns to the drawer presentation; closing the drawer remains a separate action.
- Keep existing narrow-window drawer overlay behavior.
- Add no dependencies.

---

### Task 1: Unify Companion presentation state

**Files:**
- Modify: `apps/desktop/src/renderer/src/store/slices/companion-slice.ts`
- Test: `apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`
- Modify: `apps/desktop/src/renderer/src/components/TabBar.tsx`

**Interfaces:**
- Produces: `type CompanionPresentation = 'closed' | 'drawer' | 'workspace'`
- Produces: `companionPresentation: CompanionPresentation`
- Produces: `setCompanionPresentation(presentation: CompanionPresentation): void`
- Preserves: `toggleCompanion(): void`, now toggling `closed` and `drawer`

- [ ] **Step 1: Write failing state-transition tests**

Add tests that assert:

```ts
expect(useAppStore.getState().companionPresentation).toBe('closed')
useAppStore.getState().toggleCompanion()
expect(useAppStore.getState().companionPresentation).toBe('drawer')
useAppStore.getState().setCompanionPresentation('workspace')
expect(useAppStore.getState().companionPresentation).toBe('workspace')
useAppStore.getState().setCompanionPresentation('drawer')
expect(useAppStore.getState().companionPresentation).toBe('drawer')
```

- [ ] **Step 2: Run the focused slice test and verify failure**

Run: `pnpm run --filter desktop test -- src/renderer/src/store/slices/companion-slice.test.ts`

Expected: FAIL because `companionPresentation` and `setCompanionPresentation` do not exist.

- [ ] **Step 3: Implement the discriminated presentation state**

Replace `companionOpen`, `companionFullscreen`, `setCompanionOpen`, and
`setCompanionFullscreen` with:

```ts
export type CompanionPresentation = 'closed' | 'drawer' | 'workspace'

companionPresentation: CompanionPresentation
setCompanionPresentation: (presentation: CompanionPresentation) => void
```

Initialize to `closed`. Implement `toggleCompanion` as:

```ts
toggleCompanion: () =>
  set((state) => ({
    companionPresentation: state.companionPresentation === 'closed' ? 'drawer' : 'closed',
  })),
```

Update both desktop and compact TabBar controls to derive their active state from
`companionPresentation !== 'closed'`.

- [ ] **Step 4: Run the focused slice and TabBar tests**

Run: `pnpm run --filter desktop test -- src/renderer/src/store/slices/companion-slice.test.ts src/renderer/src/components/TabBar.test.tsx`

Expected: PASS.

### Task 2: Build the shared full-window workspace surface

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx`
- Test: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`

**Interfaces:**
- Consumes: `companionPresentation` and `setCompanionPresentation`
- Produces: `CompanionWorkspace(): React.JSX.Element | null`
- Produces: `CompanionBody` layout variant `'drawer' | 'workspace'`

- [ ] **Step 1: Replace the modal test with workspace semantics**

Add a test that sets presentation to `workspace`, renders `CompanionWorkspace`, and asserts:

```ts
expect(screen.getByRole('region', { name: 'AI companion workspace' })).toBeVisible()
expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
expect(screen.getByRole('button', { name: 'Back to document' })).toBeVisible()
```

Click `Back to document` and assert the presentation becomes `drawer`.

- [ ] **Step 2: Run the focused component test and verify failure**

Run: `pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionPanel.test.tsx`

Expected: FAIL because `CompanionWorkspace` does not exist and the old component is a dialog.

- [ ] **Step 3: Implement workspace rendering**

Remove `Dialog`, `DialogContent`, `DialogHeader`, and `DialogTitle` from this file. Replace
`CompanionFullscreen` with:

```tsx
export function CompanionWorkspace() {
  const presentation = useAppStore((s) => s.companionPresentation)
  const setPresentation = useAppStore((s) => s.setCompanionPresentation)

  useEffect(() => {
    if (presentation === 'workspace') refreshCompanionMeta()
  }, [presentation])

  if (presentation !== 'workspace') return null

  return (
    <section
      aria-label="AI companion workspace"
      className="flex min-h-0 flex-1 flex-col overflow-hidden bg-background"
    >
      <CompanionBody
        layout="workspace"
        onBack={() => setPresentation('drawer')}
      />
    </section>
  )
}
```

Give `CompanionBody` a workspace header with a visible `Back to document` button and wrap the
messages/composer in `mx-auto flex min-h-0 w-full max-w-4xl flex-1 flex-col`. Keep the drawer body
full-width. `CompanionPanel` renders its open width only for `drawer`, enters `workspace` from
Expand, and sets `closed` from Close.

- [ ] **Step 4: Run the focused component tests**

Run: `pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionPanel.test.tsx`

Expected: PASS.

### Task 3: Route the app shell into workspace mode

**Files:**
- Modify: `apps/desktop/src/renderer/src/App.tsx`
- Test: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`

**Interfaces:**
- Consumes: `CompanionWorkspace`
- Consumes: `companionPresentation === 'workspace'`

- [ ] **Step 1: Add a reader-hidden regression assertion**

Extend the workspace component test fixture to render a sentinel reader sibling only when the
presentation is not `workspace`, matching the App conditional, and assert the sentinel is absent
in workspace mode and present after `Back to document`.

- [ ] **Step 2: Run the focused test and verify the new assertion fails before routing**

Run: `pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionPanel.test.tsx`

Expected: FAIL until the fixture and production routing both use the presentation selector.

- [ ] **Step 3: Replace dialog mounting with app-shell routing**

In `MainApp`, select `companionPresentation`. Inside the area below `TitlebarInset`, render:

```tsx
{companionPresentation === 'workspace' ? (
  <CompanionWorkspace />
) : (
  <>
    <Sidebar />
    <main aria-label="Document">...</main>
    <CompanionPanel />
  </>
)}
```

Keep `CommandPalette`, `ShortcutsDialog`, and `SettingsDialog` mounted outside this conditional so
global commands remain available. Remove the `CompanionFullscreen` import and render.

- [ ] **Step 4: Run focused renderer tests**

Run: `pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionPanel.test.tsx src/renderer/src/components/TabBar.test.tsx src/renderer/src/store/slices/companion-slice.test.ts`

Expected: PASS.

### Task 4: Verify and package the test build

**Files:**
- Modify: `docs/superpowers/plans/2026-07-31-companion-workspace-presentation.md` (checkbox status only)

**Interfaces:**
- Produces: packaged macOS app at `apps/desktop/dist/mac-arm64/Mdow.app`

- [ ] **Step 1: Run full repository verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test`

Expected: all commands exit 0; existing lint warnings are allowed, lint errors are not.

- [ ] **Step 2: Build the macOS distribution**

Run: `pnpm run --filter desktop build:dist -- --mac`

Expected: `apps/desktop/dist/mac-arm64/Mdow.app` reports version 1.6.0 and packaging exits 0.

- [ ] **Step 3: Perform a hands-on workspace check**

Launch the packaged app, load `/tmp/mdow-companion-lab-2026-07-31`, open Companion, and select
Expand. Verify the resulting screen has no dialog frame or backdrop, no visible reader/sidebar,
and a working `Back to document` action that restores the reader and drawer.

- [ ] **Step 4: Commit the implementation**

```bash
git add \
  apps/desktop/src/renderer/src/App.tsx \
  apps/desktop/src/renderer/src/components/TabBar.tsx \
  apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx \
  apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx \
  apps/desktop/src/renderer/src/store/slices/companion-slice.ts \
  apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts \
  docs/superpowers/plans/2026-07-31-companion-workspace-presentation.md
git commit -m "feat: replace companion modal with workspace"
```
