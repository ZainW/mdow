# Settings Sidebar Footer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a visible Settings button to the bottom of the sidebar that opens the existing Settings dialog.

**Architecture:** Keep the existing Settings dialog and store state unchanged. Add a compact sidebar footer inside `Sidebar.tsx` that calls `setSettingsOpen(true)`, and update sidebar tests to define the new footer utility separately from the removed old rail actions.

**Tech Stack:** React 19, Zustand app store, shadcn-style `Button`, lucide-react `Settings` icon, Vitest, Testing Library.

---

## File Structure

- Modify `apps/desktop/src/renderer/src/components/Sidebar.tsx`: render a sidebar footer with an accessible Settings icon button wired to `useAppStore((s) => s.setSettingsOpen)`.
- Modify `apps/desktop/src/renderer/src/components/Sidebar.test.tsx`: update expectations for the removed old rail actions and add a click test for the new Settings footer button.
- No new component file is needed because the footer is small and only used by `Sidebar`.
- No store, IPC, or dialog files should change because `settingsOpen` and `SettingsDialog` already exist.

---

### Task 1: Add Failing Sidebar Tests

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.test.tsx`

- [ ] **Step 1: Update the old rail-actions test to stop rejecting Settings**

Replace the existing test named `does not render the old permanent sidebar rail actions` with this version:

```tsx
it('does not render the old permanent sidebar rail file actions', () => {
  renderSidebar()

  expect(screen.queryByRole('button', { name: 'Quick Open' })).not.toBeInTheDocument()
  expect(screen.queryByRole('button', { name: 'Open File' })).not.toBeInTheDocument()
})
```

- [ ] **Step 2: Add a failing test for the new footer button**

Add this test immediately after the updated old rail-actions test:

```tsx
it('opens settings from the sidebar footer', () => {
  useAppStore.setState({ settingsOpen: false })
  renderSidebar()

  fireEvent.click(screen.getByRole('button', { name: 'Settings' }))

  expect(useAppStore.getState().settingsOpen).toBe(true)
})
```

- [ ] **Step 3: Run the targeted test and verify it fails**

Run:

```bash
pnpm run --filter desktop test -- Sidebar.test.tsx -t 'opens settings from the sidebar footer'
```

Expected result: FAIL because Testing Library cannot find a button with accessible name `Settings`.

- [ ] **Step 4: Commit the failing tests**

Run:

```bash
git add apps/desktop/src/renderer/src/components/Sidebar.test.tsx
git commit -m "test: cover sidebar settings footer"
```

---

### Task 2: Implement The Sidebar Footer Button

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.tsx`
- Test: `apps/desktop/src/renderer/src/components/Sidebar.test.tsx`

- [ ] **Step 1: Add the needed imports**

In `apps/desktop/src/renderer/src/components/Sidebar.tsx`, update the sidebar UI import to include `SidebarFooter`:

```tsx
import {
  Sidebar as ShadcnSidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarGroup,
  SidebarGroupContent,
} from './ui/sidebar'
```

Update the lucide import to include `Settings`:

```tsx
import { Clock, Folder, FolderOpen, List, Settings } from 'lucide-react'
```

- [ ] **Step 2: Read the settings action from the store**

Inside `Sidebar`, after the existing `setSidebarMode` selector, add:

```tsx
const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
```

The relevant selector block should look like this:

```tsx
const mode = useAppStore((s) => s.sidebarMode)
const setSidebarMode = useAppStore((s) => s.setSidebarMode)
const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
const modeRoving = useRovingFocus({ orientation: 'horizontal' })
```

- [ ] **Step 3: Render the footer below sidebar content**

In `Sidebar`, insert this footer after the closing `</SidebarContent>` and before `</ShadcnSidebar>`:

```tsx
<SidebarFooter className="border-t border-border-subtle p-2">
  <Button
    type="button"
    variant="ghost"
    size="sm"
    aria-label="Settings"
    title="Settings"
    className="w-full justify-start gap-2 text-muted-foreground hover:text-foreground"
    onClick={() => setSettingsOpen(true)}
  >
    <Settings className="size-3.5 shrink-0" aria-hidden />
    <span>Settings</span>
  </Button>
</SidebarFooter>
```

- [ ] **Step 4: Run the targeted Settings footer test**

Run:

```bash
pnpm run --filter desktop test -- Sidebar.test.tsx -t 'opens settings from the sidebar footer'
```

Expected result: PASS.

- [ ] **Step 5: Run all sidebar tests**

Run:

```bash
pnpm run --filter desktop test -- Sidebar.test.tsx
```

Expected result: all tests in `Sidebar.test.tsx` pass.

- [ ] **Step 6: Commit the implementation**

Run:

```bash
git add apps/desktop/src/renderer/src/components/Sidebar.tsx
git commit -m "feat: add sidebar settings footer"
```

---

### Task 3: Final Verification

**Files:**
- Verify: `apps/desktop/src/renderer/src/components/Sidebar.tsx`
- Verify: `apps/desktop/src/renderer/src/components/Sidebar.test.tsx`

- [ ] **Step 1: Run typecheck**

Run:

```bash
pnpm run typecheck
```

Expected result: command exits with status 0.

- [ ] **Step 2: Run lint**

Run:

```bash
pnpm run lint
```

Expected result: command exits with status 0.

- [ ] **Step 3: Run format check**

Run:

```bash
pnpm run fmt:check
```

Expected result: command exits with status 0.

- [ ] **Step 4: Run tests**

Run:

```bash
pnpm run test
```

Expected result: command exits with status 0.

- [ ] **Step 5: Inspect final diff**

Run:

```bash
git diff --stat HEAD~2..HEAD
git diff HEAD~2..HEAD -- apps/desktop/src/renderer/src/components/Sidebar.tsx apps/desktop/src/renderer/src/components/Sidebar.test.tsx
```

Expected result: the diff only adds the sidebar Settings footer and updates the relevant sidebar tests.

- [ ] **Step 6: Commit verification notes if needed**

If no code changes were made during verification, do not create a commit. If formatting changed files, commit only the touched implementation/test files:

```bash
git add apps/desktop/src/renderer/src/components/Sidebar.tsx apps/desktop/src/renderer/src/components/Sidebar.test.tsx
git commit -m "chore: format sidebar settings footer"
```

---

## Self-Review

- Spec coverage: The plan adds a visible sidebar footer button, uses the existing dialog state, keeps menu and keyboard access unchanged, avoids a fourth sidebar mode, and updates tests.
- Placeholder scan: No placeholder steps remain; every code-edit step includes exact code and every verification step includes exact commands and expected results.
- Type consistency: `setSettingsOpen(true)`, `SidebarFooter`, `Settings`, and `settingsOpen` match existing project names and APIs.
