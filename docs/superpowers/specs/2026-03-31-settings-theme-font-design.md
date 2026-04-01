# Settings Dialog, Theme Picker & Default Font

## Overview

Polish the settings dialog layout, add a theme picker (Light / Dark / System defaulting to System), and change the default body font from Inter to Charter.

## 1. Settings Dialog Reorganization

### Current state

The dialog is a flat list of controls separated by `<Separator />` between every field, with per-field helper text. It feels repetitive and lacks visual hierarchy.

### New layout

Group settings into two sections with clear headers:

**Appearance**

- Section header: "Appearance" with subtitle "Colors and visual style"
- Theme: Select dropdown with options Light / Dark / System (default: System)

**Typography**

- Section header: "Typography" with subtitle "Fonts and reading comfort"
- Content Font: Select dropdown (default changes from `inter` to `charter`)
- Code Font: Select dropdown (default: `geist-mono`, unchanged)
- Font Size: Slider 13–24px (default: 16px, unchanged)
- Line Height: Slider 1.2–2.2 (default: 1.6, unchanged)

Separators appear only between the two groups, not between individual fields within a group. Remove the per-field `<p>` descriptions — the section subtitle provides enough context.

### Dialog header

Keep title "Settings" and description "Customize how markdown content is displayed."

## 2. Theme Picker

### Architecture: Electron `nativeTheme.themeSource`

The theme preference is a three-way choice: `'light' | 'dark' | 'system'`.

**Main process (`store.ts`):**

- Add `theme: string` to `StoreSchema` with default `'system'`
- Persist via `saveAppState` / `getAppState` (same pattern as other settings)

**Main process (`index.ts`):**

- On app ready, read persisted theme and set `nativeTheme.themeSource`
- The existing `nativeTheme.on('updated')` listener already broadcasts `theme:changed` to the renderer — no change needed there

**Main process (`ipc.ts`):**

- Add handler `theme:set` that receives `'light' | 'dark' | 'system'`, sets `nativeTheme.themeSource`, and persists to store

**Preload (`index.ts`):**

- Add `setTheme: (theme: string) => Promise<void>` to `ElectronAPI`
- Wire to `ipcRenderer.invoke('theme:set', theme)`

**Renderer (`useTheme.ts`):**

- No changes needed — it already listens for `theme:changed` from main and toggles `.dark` class

**Renderer (`app-store.ts`):**

- Add `theme: string` state with default `'system'`
- Add `setTheme: (theme: string) => void` that calls `window.api.setTheme(theme)` and updates store
- Restore from `getAppState` on init (same pattern as other settings)

**Renderer (`SettingsDialog.tsx`):**

- Theme select with three options: Light, Dark, System
- Calls `setTheme()` on change

### Behavioral notes

- `nativeTheme.themeSource = 'system'` means follow OS preference (current behavior)
- Setting `themeSource` to `'light'` or `'dark'` overrides OS, and `nativeTheme.shouldUseDarkColors` updates accordingly
- The existing `nativeTheme.on('updated')` fires when `themeSource` changes, so the renderer gets notified automatically

## 3. Default Body Font: Charter

### Change

- Default `contentFont` changes from `'inter'` to `'charter'`
- Charter is a system font on macOS (ships since macOS 10.x)
- Fallback stack: `Charter, 'Bitstream Charter', Georgia, serif`

### Files affected

- `SettingsDialog.tsx`: Change `CONTENT_FONTS` array — move Charter to first position, update default
- `app-store.ts`: Change `contentFont` default from `'inter'` to `'charter'`
- `store.ts` (main): Change `contentFont` default from `'inter'` to `'charter'`
- `markdown.css`: Update `--md-content-font` default to Charter stack

### Migration

Existing users who never changed their font will still have `'inter'` persisted. This is fine — they explicitly have Inter selected and can switch in settings. New installs get Charter.

## 4. Files Changed (Summary)

| File                                                          | Change                                                            |
| ------------------------------------------------------------- | ----------------------------------------------------------------- |
| `apps/desktop/src/main/store.ts`                              | Add `theme` to schema, change `contentFont` default               |
| `apps/desktop/src/main/index.ts`                              | Set `nativeTheme.themeSource` on startup from persisted value     |
| `apps/desktop/src/main/ipc.ts`                                | Add `theme:set` IPC handler                                       |
| `apps/desktop/src/preload/index.ts`                           | Add `setTheme` to API, add `theme` to `AppState`                  |
| `apps/desktop/src/renderer/src/store/app-store.ts`            | Add `theme` state/setter, change `contentFont` default            |
| `apps/desktop/src/renderer/src/components/SettingsDialog.tsx` | Reorganize into grouped sections, add theme select, reorder fonts |
| `apps/desktop/src/renderer/src/assets/styles/markdown.css`    | Update default `--md-content-font`                                |

## 5. What's NOT Changing

- Code font default (Geist Mono)
- Font size default (16px)
- Line height default (1.6)
- `useTheme.ts` hook (already works)
- No new npm dependencies
- No bundled font files (Charter is a system font)
