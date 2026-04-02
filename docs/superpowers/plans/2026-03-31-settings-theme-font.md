# Settings Dialog, Theme Picker & Default Font — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish the settings dialog into grouped sections, add a theme picker (Light/Dark/System), and change the default body font to Charter.

**Architecture:** Theme preference flows through Electron's `nativeTheme.themeSource` — the main process owns the source of truth, persists it in electron-store, and broadcasts changes to the renderer via the existing `theme:changed` IPC event. The settings dialog reorganizes into Appearance and Typography groups. Default content font changes from Inter to Charter (system font).

**Tech Stack:** Electron (nativeTheme API), React, Zustand, electron-store, shadcn/ui Select + Slider

---

### Task 1: Add theme persistence to main process store

**Files:**

- Modify: `apps/desktop/src/main/store.ts`

- [ ] **Step 1: Add `theme` to StoreSchema and update defaults**

In `apps/desktop/src/main/store.ts`, add `theme` to the interface and defaults, and change `contentFont` default:

```typescript
// In StoreSchema interface, add:
theme: string

// In store defaults, add:
theme: 'system',

// In store defaults, change:
contentFont: 'charter',  // was 'inter'
```

- [ ] **Step 2: Add theme to getAppState return**

In the `getAppState` function, add:

```typescript
theme: store.get('theme'),
```

- [ ] **Step 3: Add theme to saveAppState**

In the `saveAppState` function, add:

```typescript
if (state.theme !== undefined) store.set('theme', state.theme)
```

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/main/store.ts
git commit -m "feat: add theme persistence to electron-store, change default font to charter"
```

---

### Task 2: Add theme IPC handler and apply on startup

**Files:**

- Modify: `apps/desktop/src/main/ipc.ts`
- Modify: `apps/desktop/src/main/index.ts`

- [ ] **Step 1: Add `theme:set` IPC handler**

In `apps/desktop/src/main/ipc.ts`, add import for `nativeTheme`:

```typescript
import { ipcMain, shell, BrowserWindow, nativeTheme } from 'electron'
```

Add import for `saveAppState` (already imported — just verify it's there).

Add the handler inside `registerIpcHandlers`, after the existing `store:save-state` handler:

```typescript
ipcMain.handle('theme:set', (_, theme: string) => {
  const valid = ['light', 'dark', 'system']
  if (!valid.includes(theme)) return
  nativeTheme.themeSource = theme as typeof nativeTheme.themeSource
  saveAppState({ theme })
})
```

- [ ] **Step 2: Apply persisted theme on startup**

In `apps/desktop/src/main/index.ts`, add `getAppState` to the import from `./store`:

```typescript
import { getWindowBounds, saveWindowBounds, getLastFolder, getAppState } from './store'
```

In the `app.whenReady().then(...)` callback, before `createWindow()`, add:

```typescript
const appState = getAppState()
if (appState.theme && appState.theme !== 'system') {
  nativeTheme.themeSource = appState.theme as typeof nativeTheme.themeSource
}
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/main/ipc.ts apps/desktop/src/main/index.ts
git commit -m "feat: add theme:set IPC handler, apply persisted theme on startup"
```

---

### Task 3: Add theme to preload API

**Files:**

- Modify: `apps/desktop/src/preload/index.ts`

- [ ] **Step 1: Add `theme` to AppState interface**

In the `AppState` interface, add:

```typescript
theme: string
```

- [ ] **Step 2: Add `setTheme` to ElectronAPI interface**

In the `ElectronAPI` interface, add:

```typescript
setTheme: (theme: string) => Promise<void>
```

- [ ] **Step 3: Add `setTheme` to the api object**

In the `api` object, add:

```typescript
setTheme: (theme) => ipcRenderer.invoke('theme:set', theme),
```

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/preload/index.ts
git commit -m "feat: expose setTheme in preload API"
```

---

### Task 4: Add theme state to Zustand store

**Files:**

- Modify: `apps/desktop/src/renderer/src/store/app-store.ts`

- [ ] **Step 1: Add theme to the AppStore interface**

Add these lines to the `AppStore` interface:

```typescript
theme: string
setTheme: (theme: string) => void
```

- [ ] **Step 2: Add theme state and setter to the store**

In the `create<AppStore>` call, add:

```typescript
theme: 'system',
setTheme: (theme) => {
  void window.api.setTheme(theme)
  set({ theme })
},
```

- [ ] **Step 3: Change contentFont default**

Change the default from `'inter'` to `'charter'`:

```typescript
contentFont: 'charter',
```

- [ ] **Step 4: Restore theme from persisted state on init**

In `App.tsx`, inside the `getAppState().then(...)` callback, the typography restoration block already sets state. Add `theme` to the restored state. Find the section that restores typography settings:

```typescript
const typo: Record<string, unknown> = {}
if (state.contentFont) typo.contentFont = state.contentFont
if (state.codeFont) typo.codeFont = state.codeFont
if (state.fontSize) typo.fontSize = state.fontSize
if (state.lineHeight) typo.lineHeight = state.lineHeight
if (Object.keys(typo).length) useAppStore.setState(typo)
```

Add theme restoration before it:

```typescript
if (state.theme) useAppStore.setState({ theme: state.theme })
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/renderer/src/store/app-store.ts apps/desktop/src/renderer/src/App.tsx
git commit -m "feat: add theme state to Zustand store, change default font to charter"
```

---

### Task 5: Reorganize SettingsDialog with grouped sections and theme picker

**Files:**

- Modify: `apps/desktop/src/renderer/src/components/SettingsDialog.tsx`

- [ ] **Step 1: Move Charter to first position in CONTENT_FONTS**

Reorder the `CONTENT_FONTS` array so Charter is first (it's the new default):

```typescript
const CONTENT_FONTS = [
  { value: 'charter', label: 'Charter', family: "Charter, 'Bitstream Charter', Georgia, serif" },
  { value: 'system-sans', label: 'System Sans', family: 'system-ui, -apple-system, sans-serif' },
  { value: 'inter', label: 'Inter', family: "'Inter', system-ui, sans-serif" },
  {
    value: 'helvetica-neue',
    label: 'Helvetica Neue',
    family: "'Helvetica Neue', Helvetica, Arial, sans-serif",
  },
  { value: 'georgia', label: 'Georgia', family: "Georgia, 'Times New Roman', serif" },
  { value: 'palatino', label: 'Palatino', family: "Palatino, 'Palatino Linotype', serif" },
  {
    value: 'times-new-roman',
    label: 'Times New Roman',
    family: "'Times New Roman', Times, serif",
  },
  {
    value: 'bookerly',
    label: 'Bookerly',
    family: "Bookerly, 'Iowan Old Style', Palatino, Georgia, serif",
  },
] as const
```

- [ ] **Step 2: Add theme options array**

Add a theme options array above the component:

```typescript
const THEME_OPTIONS = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
] as const
```

- [ ] **Step 3: Wire up theme state in the component**

Add theme state subscriptions inside `SettingsDialog`:

```typescript
const theme = useAppStore((s) => s.theme)
const setTheme = useAppStore((s) => s.setTheme)
```

- [ ] **Step 4: Rewrite the dialog body with grouped sections**

Replace the entire `<div className="flex flex-col gap-5">` block with:

```tsx
<div className="flex flex-col gap-5">
  {/* ── Appearance ── */}
  <div className="flex flex-col gap-3">
    <div>
      <h3 className="text-sm font-semibold">Appearance</h3>
      <p className="text-xs text-muted-foreground">Colors and visual style</p>
    </div>
    <div className="flex items-center justify-between">
      <span className="text-sm">Theme</span>
      <Select value={theme} onValueChange={(v) => v && setTheme(v)}>
        <SelectTrigger className="w-[130px]" size="sm">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {THEME_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>
  </div>

  <Separator />

  {/* ── Typography ── */}
  <div className="flex flex-col gap-3">
    <div>
      <h3 className="text-sm font-semibold">Typography</h3>
      <p className="text-xs text-muted-foreground">Fonts and reading comfort</p>
    </div>

    <div className="flex items-center justify-between">
      <span className="text-sm">Content Font</span>
      <Select value={contentFont} onValueChange={(v) => v && setContentFont(v)}>
        <SelectTrigger className="w-[160px]" size="sm">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {CONTENT_FONTS.map((font) => (
              <SelectItem key={font.value} value={font.value}>
                <span style={{ fontFamily: font.family }}>{font.label}</span>
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>

    <div className="flex items-center justify-between">
      <span className="text-sm">Code Font</span>
      <Select value={codeFont} onValueChange={(v) => v && setCodeFont(v)}>
        <SelectTrigger className="w-[160px]" size="sm">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {CODE_FONTS.map((font) => (
              <SelectItem key={font.value} value={font.value}>
                <span style={{ fontFamily: font.family }}>{font.label}</span>
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>

    <div className="flex flex-col gap-2 pt-1">
      <div className="flex items-center justify-between">
        <span className="text-sm">Font Size</span>
        <span className="text-xs tabular-nums text-muted-foreground">{fontSize}px</span>
      </div>
      <Slider
        value={[fontSize]}
        onValueChange={(v) => setFontSize(Number(v[0]))}
        min={13}
        max={24}
        step={1}
      />
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>Small</span>
        <span>Large</span>
      </div>
    </div>

    <div className="flex flex-col gap-2 pt-1">
      <div className="flex items-center justify-between">
        <span className="text-sm">Line Height</span>
        <span className="text-xs tabular-nums text-muted-foreground">{lineHeight.toFixed(1)}</span>
      </div>
      <Slider
        value={[lineHeight]}
        onValueChange={(v) => setLineHeight(Number(v[0]))}
        min={1.2}
        max={2.2}
        step={0.1}
      />
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>Compact</span>
        <span>Spacious</span>
      </div>
    </div>
  </div>
</div>
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/renderer/src/components/SettingsDialog.tsx
git commit -m "feat: reorganize settings into grouped sections, add theme picker"
```

---

### Task 6: Update markdown.css default font

**Files:**

- Modify: `apps/desktop/src/renderer/src/assets/styles/markdown.css`

- [ ] **Step 1: Update the default `--md-content-font` value**

Change line 3 from:

```css
--md-content-font: 'Inter', system-ui, sans-serif;
```

to:

```css
--md-content-font: Charter, 'Bitstream Charter', Georgia, serif;
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/renderer/src/assets/styles/markdown.css
git commit -m "feat: change default markdown body font to Charter"
```

---

### Task 7: Verify and test

- [ ] **Step 1: Run typecheck**

```bash
pnpm run typecheck
```

Expected: No errors.

- [ ] **Step 2: Run lint**

```bash
pnpm run lint
```

Expected: No errors.

- [ ] **Step 3: Run format check**

```bash
pnpm run fmt:check
```

If formatting issues, run `pnpm run fmt` and commit.

- [ ] **Step 4: Run dev and manually test**

```bash
pnpm run dev
```

Manual checks:

1. Open Settings (Cmd+,) — should show Appearance and Typography sections
2. Theme dropdown defaults to "System" — switching to Light/Dark should change the app theme immediately
3. Switching back to "System" should follow OS preference
4. Content font defaults to "Charter" for new installs
5. Quit and reopen — theme preference should persist
6. All existing font/size/line-height settings still work

- [ ] **Step 5: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: address typecheck/lint/format issues"
```
