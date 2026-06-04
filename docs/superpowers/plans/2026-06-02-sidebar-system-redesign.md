# Sidebar System Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Mdow's rail-plus-drawer sidebar with one quiet left sidebar whose header tabs switch between Recents, Folder, and Outline.

**Architecture:** Reuse the existing Zustand `sidebarMode` and `sidebarOpen` state. Keep `RecentsList`, `FolderTree`, and `OutlineList` as the content units, but move mode selection into a compact header radiogroup inside `Sidebar.tsx` and remove the permanent rail action column.

**Tech Stack:** React 19, TypeScript, Zustand, TanStack Query, Vitest, Testing Library, Tailwind CSS v4, shadcn/ui, Lucide icons.

---

### Task 1: Update Sidebar Tests for Single-Surface Navigation

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.test.tsx`

- [ ] **Step 1: Write the failing tests**

Replace the first test with:

```tsx
it('exposes Sidebar mode tabs inside a single sidebar surface', () => {
  renderSidebar()
  const sidebar = screen.getByRole('complementary', { name: 'Sidebar' })
  const group = screen.getByRole('radiogroup', { name: 'Sidebar mode' })

  expect(sidebar).toContainElement(group)
  expect(screen.queryByLabelText('Workspace actions')).not.toBeInTheDocument()

  const options = screen.getAllByRole('radio')
  expect(options).toHaveLength(3)
  expect(options.map((o) => o.textContent)).toEqual(['Recents', 'Folder', 'Outline'])
  expect(options.map((o) => o.getAttribute('aria-label'))).toEqual([
    'Recents',
    'Folder',
    'Outline',
  ])
})
```

Add a test after the active-mode test:

```tsx
it('does not render the old permanent sidebar rail actions', () => {
  renderSidebar()

  expect(screen.queryByRole('button', { name: 'Quick Open' })).not.toBeInTheDocument()
  expect(screen.queryByRole('button', { name: 'Open File' })).not.toBeInTheDocument()
  expect(screen.queryByRole('button', { name: 'Settings' })).not.toBeInTheDocument()
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- -t Sidebar
```

Expected result: FAIL. The first test should fail because there is no `complementary` landmark named `Sidebar` and because the mode controls currently expose icon-only text content. The rail action test should fail because the old rail action buttons still render.

- [ ] **Step 3: Commit the failing tests**

Do not commit failing tests separately. Keep them staged only after implementation passes.

### Task 2: Replace the Rail with Header Mode Tabs

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.tsx`

- [ ] **Step 1: Remove rail-only imports and helpers**

Delete these imports from `lucide-react`:

```tsx
File, Search, Settings
```

Delete these rail-only constants and helpers:

```tsx
const ICON_HEIGHT = 28
const ICON_GAP = 2
const RAIL_PAD_TOP = 6
const INDICATOR_HEIGHT = 16

function indicatorY(modeIndex: number): number {
  const iconTop = RAIL_PAD_TOP + modeIndex * (ICON_HEIGHT + ICON_GAP)
  return iconTop + (ICON_HEIGHT - INDICATOR_HEIGHT) / 2
}

function railClasses(active: boolean): string {
  return `rail-icon-btn h-7 w-7 hover:bg-transparent ${
    active ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'
  }`
}

function RailModeIcon(...) { ... }
function RailButton(...) { ... }
```

Remove unused store selectors and handlers from `Sidebar`:

```tsx
const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
const openTab = useAppStore((s) => s.openTab)
const queryClient = useQueryClient()
const handleOpenFile = useCallback(...)
const handleOpenFolder = useCallback(...)
const modeIndex = MODES.indexOf(mode)
```

Keep `setOpenFolder` only if it is still needed by the Folder empty state; otherwise remove it.

- [ ] **Step 2: Add a `MODE_CONFIG` map**

Add near `MODES`:

```tsx
const MODE_CONFIG: Record<SidebarMode, { label: string; Icon: typeof Clock }> = {
  recents: { label: 'Recents', Icon: Clock },
  folder: { label: 'Folder', Icon: Folder },
  outline: { label: 'Outline', Icon: List },
}
```

- [ ] **Step 3: Replace the outer JSX with one sidebar surface**

Use this structure:

```tsx
return (
  <div
    role="complementary"
    aria-label="Sidebar"
    className="sidebar-drawer shrink-0 overflow-hidden border-r border-border-subtle"
    style={{ width: sidebarOpen ? DRAWER_WIDTH : 0 }}
    aria-hidden={!sidebarOpen}
    inert={!sidebarOpen ? true : undefined}
  >
    <ShadcnSidebar collapsible="none" className="h-full border-none" style={{ width: DRAWER_WIDTH }}>
      <SidebarHeader className="border-b border-border-subtle px-2 py-2">
        <SidebarModeTabs mode={mode} onModeChange={setSidebarMode} roving={railRoving} />
      </SidebarHeader>
      <SidebarContent key={mode} className="drawer-mode">
        {mode === 'recents' && <RecentsList />}
        {mode === 'folder' && openFolderPath && <FolderTree />}
        {mode === 'folder' && !openFolderPath && (
          <EmptyState
            size="sm"
            icon={FolderOpen}
            title="No folder open"
            hint={`Use the app menu, keyboard shortcut, or drag a folder onto this window. Right-click a file to ${revealLabel.toLowerCase()}.`}
          />
        )}
        {mode === 'outline' && (
          <OutlineList headings={docHeadings} activeId={activeHeadingId} hasActiveDoc={hasOpenTab} />
        )}
      </SidebarContent>
    </ShadcnSidebar>
  </div>
)
```

- [ ] **Step 4: Add `SidebarModeTabs` and `SidebarModeTab`**

Add below `Sidebar`:

```tsx
function SidebarModeTabs({
  mode,
  onModeChange,
  roving,
}: {
  mode: SidebarMode
  onModeChange: (mode: SidebarMode) => void
  roving: ReturnType<typeof useRovingFocus>
}) {
  return (
    <div
      ref={roving.containerRef}
      role="radiogroup"
      aria-label="Sidebar mode"
      className="grid grid-cols-3 gap-1"
      onKeyDown={roving.onKeyDown}
    >
      {MODES.map((item) => (
        <SidebarModeTab
          key={item}
          mode={item}
          checked={mode === item}
          onSelect={() => onModeChange(item)}
        />
      ))}
    </div>
  )
}

function SidebarModeTab({
  mode,
  checked,
  onSelect,
}: {
  mode: SidebarMode
  checked: boolean
  onSelect: () => void
}) {
  const { label, Icon } = MODE_CONFIG[mode]

  return (
    <Button
      variant="ghost"
      size="sm"
      role="radio"
      tabIndex={rovingTabIndex(checked)}
      aria-checked={checked}
      aria-label={label}
      title={label}
      className={`h-7 min-w-0 justify-center gap-1.5 px-1.5 text-xs ${
        checked
          ? 'bg-sidebar-accent text-sidebar-accent-foreground'
          : 'text-muted-foreground hover:bg-sidebar-accent/70 hover:text-foreground'
      }`}
      onClick={onSelect}
    >
      <Icon className="size-3.5 shrink-0" aria-hidden />
      <span className="truncate">{label}</span>
    </Button>
  )
}
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
pnpm run --filter desktop test -- -t Sidebar
```

Expected result: PASS.

### Task 3: Verify Full Behavior and Clean Up

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.tsx` only to resolve verification failures introduced by this task.
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.test.tsx` only to resolve verification failures introduced by this task.

- [ ] **Step 1: Run full verification**

Run:

```bash
pnpm run test
pnpm run typecheck
pnpm run lint
pnpm run fmt:check
```

Expected result: all commands pass.

- [ ] **Step 2: Fix formatting when `fmt:check` reports formatting failures**

If `fmt:check` fails, run:

```bash
pnpm run fmt
pnpm run fmt:check
```

Expected result: `fmt:check` passes.

- [ ] **Step 3: Commit implementation**

Run:

```bash
git add apps/desktop/src/renderer/src/components/Sidebar.tsx apps/desktop/src/renderer/src/components/Sidebar.test.tsx docs/superpowers/plans/2026-06-02-sidebar-system-redesign.md
git commit -m "feat: simplify sidebar mode navigation"
```

Expected result: commit succeeds on `codex/sidebar-system-redesign`.
