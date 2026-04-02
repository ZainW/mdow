# Design System Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the cool-toned Linear-inspired design system with a warm neutral palette, reworked typographic scale, seamless chrome, and curated font pairings — visual layer only, all functionality preserved.

**Architecture:** New CSS custom property values (OKLCH) for warm light + neutral dark themes. Reworked markdown typography scale tuned for Inter as the default font. Font options trimmed from 8/7 to 4/3 with per-font sizing adjustments. Component styles updated to use shared background for seamless feel. All changes on a `design-system-v2` branch.

**Tech Stack:** CSS custom properties (OKLCH), Tailwind v4 inline theme, React 19, Zustand, Base UI

**Spec:** `docs/superpowers/specs/2026-04-02-design-system-redesign.md`

---

### Task 0: Create feature branch

**Files:** None

- [ ] **Step 1: Create and switch to feature branch**

```bash
git checkout -b design-system-v2
```

- [ ] **Step 2: Verify branch**

```bash
git branch --show-current
```

Expected: `design-system-v2`

---

### Task 1: Replace color tokens in index.css

**Files:**

- Modify: `apps/desktop/src/renderer/src/assets/styles/index.css:86-156`

- [ ] **Step 1: Replace the `:root` light theme block (lines 86-120)**

Replace the existing `:root` block with warm neutral OKLCH values:

```css
:root {
  color-scheme: light;

  --background: oklch(0.98 0.005 70);
  --foreground: oklch(0.13 0.02 50);
  --card: oklch(0.98 0.005 70);
  --card-foreground: oklch(0.13 0.02 50);
  --popover: oklch(1 0 0);
  --popover-foreground: oklch(0.13 0.02 50);
  --primary: oklch(0.55 0.17 260);
  --primary-foreground: oklch(1 0 0);
  --secondary: oklch(0.96 0.006 70);
  --secondary-foreground: oklch(0.35 0.02 50);
  --muted: oklch(0.96 0.006 70);
  --muted-foreground: oklch(0.45 0.015 50);
  --accent: oklch(0.62 0.16 55);
  --accent-foreground: oklch(1 0 0);
  --destructive: oklch(0.55 0.2 25);
  --border: oklch(0.89 0.01 70);
  --border-subtle: oklch(0.93 0.008 70);
  --input: oklch(0.89 0.01 70);
  --ring: oklch(0.55 0.17 260);
  --radius: 0.5rem;

  --chart-1: oklch(0.62 0.16 55);
  --chart-2: oklch(0.6 0.15 160);
  --chart-3: oklch(0.55 0.17 260);
  --chart-4: oklch(0.65 0.15 310);
  --chart-5: oklch(0.7 0.15 30);

  --sidebar: oklch(0.98 0.005 70);
  --sidebar-foreground: oklch(0.13 0.02 50);
  --sidebar-primary: oklch(0.55 0.17 260);
  --sidebar-primary-foreground: oklch(1 0 0);
  --sidebar-accent: oklch(0.94 0.008 70);
  --sidebar-accent-foreground: oklch(0.13 0.02 50);
  --sidebar-border: oklch(0.93 0.008 70);
  --sidebar-ring: oklch(0.55 0.17 260);
}
```

Key changes: hue shifted from 260 (blue) to 70 (warm stone/sand), sidebar tokens now match main surface tokens for seamless feel.

- [ ] **Step 2: Replace the `.dark` theme block (lines 123-156)**

Replace with neutral dark values (no warmth):

```css
.dark {
  color-scheme: dark;

  --background: oklch(0.14 0 0);
  --foreground: oklch(0.92 0 0);
  --card: oklch(0.14 0 0);
  --card-foreground: oklch(0.92 0 0);
  --popover: oklch(0.17 0 0);
  --popover-foreground: oklch(0.92 0 0);
  --primary: oklch(0.68 0.15 260);
  --primary-foreground: oklch(0.14 0 0);
  --secondary: oklch(0.2 0 0);
  --secondary-foreground: oklch(0.92 0 0);
  --muted: oklch(0.19 0 0);
  --muted-foreground: oklch(0.65 0 0);
  --accent: oklch(0.75 0.15 80);
  --accent-foreground: oklch(0.14 0 0);
  --destructive: oklch(0.6 0.2 25);
  --border: oklch(0.27 0 0);
  --border-subtle: oklch(0.22 0 0);
  --input: oklch(0.27 0 0);
  --ring: oklch(0.68 0.15 260);
  --radius: 0.5rem;

  --chart-1: oklch(0.75 0.15 80);
  --chart-2: oklch(0.65 0.15 160);
  --chart-3: oklch(0.68 0.15 260);
  --chart-4: oklch(0.7 0.15 310);
  --chart-5: oklch(0.75 0.15 30);

  --sidebar: oklch(0.14 0 0);
  --sidebar-foreground: oklch(0.92 0 0);
  --sidebar-primary: oklch(0.68 0.15 260);
  --sidebar-primary-foreground: oklch(0.14 0 0);
  --sidebar-accent: oklch(0.2 0 0);
  --sidebar-accent-foreground: oklch(0.92 0 0);
  --sidebar-border: oklch(0.22 0 0);
  --sidebar-ring: oklch(0.68 0.15 260);
}
```

Key change: chroma set to 0 across all neutral colors (pure gray, no blue lean). Only primary and accent retain chroma.

- [ ] **Step 3: Add `--border-subtle` to the `@theme inline` block**

Add this line inside the `@theme inline` block (after the existing color mappings around line 42):

```css
--color-border-subtle: var(--border-subtle);
```

This makes `border-subtle` available as a Tailwind utility class.

- [ ] **Step 4: Update scrollbar colors (lines 185-215)**

Replace scrollbar color values to use warm tones for light and neutral for dark:

```css
/* Light mode scrollbar */
::-webkit-scrollbar-thumb {
  background: oklch(0.6 0.01 70 / 0.25);
}
::-webkit-scrollbar-thumb:hover {
  background: oklch(0.5 0.015 70 / 0.45);
}

/* Dark mode scrollbar */
.dark ::-webkit-scrollbar-thumb {
  background: oklch(0.75 0 0 / 0.2);
}
.dark ::-webkit-scrollbar-thumb:hover {
  background: oklch(0.75 0 0 / 0.4);
}
```

- [ ] **Step 5: Update search highlight colors (lines 432-464)**

Replace search highlight oklch values to use warm tone for light, neutral for dark:

Light mode highlights:

```css
.search-highlight {
  background-color: oklch(0.93 0.08 85); /* warm yellow */
}
.search-highlight.search-highlight-active {
  background-color: oklch(0.82 0.12 260); /* blue active — keep for contrast */
}
```

Dark mode highlights:

```css
.dark .search-highlight {
  background-color: oklch(0.45 0.08 85);
}
.dark .search-highlight.search-highlight-active {
  background-color: oklch(0.45 0.12 260);
}
```

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/renderer/src/assets/styles/index.css
git commit -m "feat(design): replace color tokens with warm neutral light + neutral dark palette"
```

---

### Task 2: Restyle markdown.css

**Files:**

- Modify: `apps/desktop/src/renderer/src/assets/styles/markdown.css` (full file, 163 lines)

- [ ] **Step 1: Replace the full markdown.css contents**

```css
/* ── Base Variables ── */

.markdown-body {
  --md-content-font: 'Inter', system-ui, -apple-system, sans-serif;
  --md-code-font: 'Geist Mono', ui-monospace, monospace;
  --md-font-size: 15.5px;
  --md-line-height: 1.65;

  font-family: var(--md-content-font);
  font-size: var(--md-font-size);
  line-height: var(--md-line-height);
  color: hsl(var(--foreground));
}

/* ── Headings ── */

.markdown-body h1,
.markdown-body h2,
.markdown-body h3,
.markdown-body h4,
.markdown-body h5,
.markdown-body h6 {
  font-weight: 600;
  color: hsl(var(--foreground));
}

.markdown-body h1 {
  font-size: 1.875em;
  font-weight: 700;
  letter-spacing: -0.025em;
  line-height: 1.2;
  margin: 2em 0 0.6em;
}

.markdown-body h1:first-child {
  margin-top: 0;
}

.markdown-body h2 {
  font-size: 1.4em;
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.25;
  margin: 1.8em 0 0.5em;
}

.markdown-body h3 {
  font-size: 1.15em;
  font-weight: 600;
  letter-spacing: -0.01em;
  line-height: 1.3;
  margin: 1.5em 0 0.4em;
}

.markdown-body h4 {
  font-size: 1em;
  font-weight: 600;
  line-height: 1.4;
  margin: 1.3em 0 0.3em;
  color: hsl(var(--muted-foreground));
}

/* ── Links ── */

.markdown-body a {
  color: hsl(var(--primary));
  text-decoration: none;
}

.markdown-body a:hover {
  text-decoration: underline;
}

/* ── Paragraphs ── */

.markdown-body p {
  margin: 0 0 1em;
}

/* ── Images ── */

.markdown-body img {
  max-width: 100%;
  border-radius: 8px;
}

/* ── Horizontal Rule ── */

.markdown-body hr {
  border: none;
  border-top: 1px solid hsl(var(--border));
  margin: 2em 0;
}

/* ── Inline Code ── */

.markdown-body code {
  font-family: var(--md-code-font);
  font-size: 0.875em;
  background: hsl(var(--muted));
  padding: 0.15em 0.4em;
  border-radius: 4px;
}

/* ── Code Blocks ── */

.markdown-body pre {
  background: hsl(var(--muted));
  border: 1px solid hsl(var(--border));
  border-radius: 8px;
  padding: 16px 20px;
  overflow-x: auto;
  margin: 0 0 1em;
  line-height: 1.5;
}

.markdown-body pre code {
  background: none;
  padding: 0;
  font-size: 13.5px;
}

/* ── Shiki Syntax Highlighting ── */

.markdown-body pre.shiki {
  overflow-x: auto;
  border-radius: 8px;
  border: 1px solid hsl(var(--border));
}

.dark .markdown-body pre.shiki {
  color: var(--shiki-dark) !important;
  background-color: var(--shiki-dark-bg) !important;
}

.dark .markdown-body pre.shiki span {
  color: var(--shiki-dark) !important;
}

/* ── Blockquotes ── */

.markdown-body blockquote {
  border-left: 3px solid hsl(var(--border));
  padding: 0.4em 1em;
  margin: 0 0 1em;
  color: hsl(var(--muted-foreground));
}

.markdown-body blockquote p:last-child {
  margin-bottom: 0;
}

/* ── Tables ── */

.markdown-body table {
  width: 100%;
  border-collapse: collapse;
  margin: 0 0 1em;
  font-size: 0.925em;
}

.markdown-body th {
  text-align: left;
  font-weight: 600;
  font-size: 0.85em;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 8px 12px;
  border-bottom: 2px solid hsl(var(--border));
  color: hsl(var(--muted-foreground));
}

.markdown-body td {
  padding: 8px 12px;
  border-bottom: 1px solid hsl(var(--border-subtle));
}

/* ── Lists ── */

.markdown-body ul,
.markdown-body ol {
  padding-left: 1.8em;
  margin: 0 0 1em;
}

.markdown-body li {
  margin: 0.25em 0;
}

.markdown-body li + li {
  margin-top: 0.35em;
}

/* ── Task Lists ── */

.markdown-body ul.contains-task-list {
  list-style: none;
  padding-left: 0;
}

.markdown-body .task-list-item {
  display: flex;
  align-items: baseline;
  gap: 0.5em;
}

.markdown-body .task-list-item input[type='checkbox'] {
  margin: 0;
}

/* ── Mermaid Diagrams ── */

.markdown-body .mermaid-container {
  display: flex;
  justify-content: center;
  margin: 1.5em 0;
  overflow-x: auto;
}

.markdown-body .mermaid-container svg {
  max-width: 100%;
  height: auto;
}

.markdown-body .mermaid-error {
  color: hsl(var(--destructive));
  font-family: var(--md-code-font);
  font-size: 0.875em;
  white-space: pre-wrap;
  padding: 1em;
  background: hsl(var(--muted));
  border-radius: 8px;
}
```

Key changes from current:

- Default font changed from Charter to Inter
- No `border-bottom` on h1/h2
- Heading scale: 1.875em → 1.4em → 1.15em → 1em with negative letter-spacing
- `font-weight: 650` on h2 (semi-bold+)
- Blockquote: no background fill, just left border
- Tables: no alternating row background, uppercase small headers with 2px bottom border
- Code blocks: `hsl(var(--muted))` background + `hsl(var(--border))` border
- Body font size: 15.5px, line-height: 1.65

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/renderer/src/assets/styles/markdown.css
git commit -m "feat(design): rework markdown typography — new scale, no heading borders, balanced spacing"
```

---

### Task 3: Trim font options and update defaults

**Files:**

- Modify: `apps/desktop/src/renderer/src/components/SettingsDialog.tsx:14-63`
- Modify: `apps/desktop/src/renderer/src/store/app-store.ts:210-213`

- [ ] **Step 1: Write a test for the font mapping functions**

Create or append to an existing test file. The font mapping functions are exported from SettingsDialog.tsx.

Add to a new file `apps/desktop/src/renderer/src/components/SettingsDialog.test.ts`:

```typescript
import { describe, expect, it } from 'vitest'
import { getContentFontFamily, getCodeFontFamily } from './SettingsDialog'

describe('getContentFontFamily', () => {
  it('returns Inter family for "inter"', () => {
    expect(getContentFontFamily('inter')).toContain('Inter')
  })

  it('returns Charter family for "charter"', () => {
    expect(getContentFontFamily('charter')).toContain('Charter')
  })

  it('returns system-ui family for "system-sans"', () => {
    expect(getContentFontFamily('system-sans')).toContain('system-ui')
  })

  it('returns Georgia family for "georgia"', () => {
    expect(getContentFontFamily('georgia')).toContain('Georgia')
  })

  it('returns default (Inter) for unknown value', () => {
    expect(getContentFontFamily('unknown-font')).toContain('Inter')
  })
})

describe('getCodeFontFamily', () => {
  it('returns Geist Mono family for "geist-mono"', () => {
    expect(getCodeFontFamily('geist-mono')).toContain('Geist Mono')
  })

  it('returns SF Mono family for "sf-mono"', () => {
    expect(getCodeFontFamily('sf-mono')).toContain('SF Mono')
  })

  it('returns JetBrains Mono family for "jetbrains-mono"', () => {
    expect(getCodeFontFamily('jetbrains-mono')).toContain('JetBrains Mono')
  })

  it('returns default (Geist Mono) for unknown value', () => {
    expect(getCodeFontFamily('unknown-font')).toContain('Geist Mono')
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
pnpm run --filter desktop test -- -t 'getContentFontFamily'
```

Expected: FAIL — the test references fonts that don't exist yet in the trimmed list (like 'inter' as default fallback).

- [ ] **Step 3: Replace CONTENT_FONTS array (lines 14-35 of SettingsDialog.tsx)**

```typescript
const CONTENT_FONTS = [
  { value: 'inter', label: 'Inter', family: "'Inter', system-ui, -apple-system, sans-serif" },
  { value: 'charter', label: 'Charter', family: "Charter, 'Bitstream Charter', Georgia, serif" },
  {
    value: 'system-sans',
    label: 'System Sans',
    family: 'system-ui, -apple-system, sans-serif',
  },
  { value: 'georgia', label: 'Georgia', family: "Georgia, 'Times New Roman', serif" },
] as const
```

- [ ] **Step 4: Replace CODE_FONTS array (lines 37-49 of SettingsDialog.tsx)**

```typescript
const CODE_FONTS = [
  { value: 'geist-mono', label: 'Geist Mono', family: "'Geist Mono', ui-monospace, monospace" },
  {
    value: 'sf-mono',
    label: 'SF Mono',
    family: "'SF Mono', SFMono-Regular, ui-monospace, monospace",
  },
  {
    value: 'jetbrains-mono',
    label: 'JetBrains Mono',
    family: "'JetBrains Mono', ui-monospace, monospace",
  },
] as const
```

- [ ] **Step 5: Update default contentFont in app-store.ts (line 210)**

Change:

```typescript
contentFont: 'charter',
```

To:

```typescript
contentFont: 'inter',
```

- [ ] **Step 6: Update default font size and line height in app-store.ts (lines 212-213)**

Change:

```typescript
fontSize: 16,
lineHeight: 1.6,
```

To:

```typescript
fontSize: 15.5,
lineHeight: 1.65,
```

- [ ] **Step 7: Run the test to verify it passes**

```bash
pnpm run --filter desktop test -- -t 'getContentFontFamily'
```

Expected: All tests PASS.

- [ ] **Step 8: Run full test suite**

```bash
pnpm run test
```

Expected: All tests pass. The app-store tests may reference old default values — if so, update the test expectations for `contentFont`, `fontSize`, and `lineHeight`.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/src/renderer/src/components/SettingsDialog.tsx \
       apps/desktop/src/renderer/src/components/SettingsDialog.test.ts \
       apps/desktop/src/renderer/src/store/app-store.ts
git commit -m "feat(design): curate font options — 4 content, 3 code, Inter as default"
```

---

### Task 4: Restyle TabBar for seamless chrome

**Files:**

- Modify: `apps/desktop/src/renderer/src/components/TabBar.tsx:23,31-34,44`

- [ ] **Step 1: Update the tab bar container (line 23)**

Change:

```tsx
<div className="flex h-9 shrink-0 items-center overflow-x-auto border-b border-border/60 bg-sidebar/30 scrollbar-none">
```

To:

```tsx
<div className="flex h-9 shrink-0 items-center overflow-x-auto border-b border-border-subtle bg-background scrollbar-none">
```

Changes: `border-border/60` → `border-border-subtle`, `bg-sidebar/30` → `bg-background` (seamless with content area).

- [ ] **Step 2: Update tab button styling (lines 31-34)**

Change:

```tsx
className={cn(
  'tab-btn group/tab relative flex h-full items-center gap-1.5 border-r border-border/40 px-3 text-xs',
  isActive ? 'bg-background text-foreground' : 'text-muted-foreground',
)}
```

To:

```tsx
className={cn(
  'tab-btn group/tab relative flex h-full items-center gap-1.5 px-3 text-xs',
  isActive ? 'text-foreground font-medium' : 'text-muted-foreground',
)}
```

Changes: removed `border-r border-border/40` (no dividers between tabs for seamless feel), active state uses `font-medium` instead of `bg-background`, inactive tabs no longer differ in background.

- [ ] **Step 3: Remove the active indicator bar (line 44)**

Change:

```tsx
<span className="tab-active-indicator absolute inset-x-0 bottom-0 h-[2px] bg-primary/80" />
```

To:

```tsx
<span className="tab-active-indicator absolute inset-x-0 bottom-0 h-[2px] bg-foreground/20" />
```

Changes: subtle underline using foreground with low opacity instead of primary color — quieter, more seamless.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/renderer/src/components/TabBar.tsx
git commit -m "feat(design): restyle tab bar — seamless background, quieter active state"
```

---

### Task 5: Restyle Sidebar for seamless chrome

**Files:**

- Modify: `apps/desktop/src/renderer/src/components/Sidebar.tsx:80`

- [ ] **Step 1: Update sidebar container border (line 80)**

Change:

```tsx
className = 'shrink-0 overflow-hidden border-r border-border/60'
```

To:

```tsx
className = 'shrink-0 overflow-hidden border-r border-border-subtle'
```

This uses the new `--border-subtle` token for a lighter separator, matching the seamless feel.

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/renderer/src/components/Sidebar.tsx
git commit -m "feat(design): soften sidebar border for seamless chrome"
```

---

### Task 6: Update index.css tab and sidebar animation styles

**Files:**

- Modify: `apps/desktop/src/renderer/src/assets/styles/index.css:274-289,292-317`

- [ ] **Step 1: Update active file accent bar color (around line 278)**

Find the active file left-border accent bar and change its color from primary-based to a warm accent. Look for references to `bg-primary` or the active file left bar styling. Change the color to use the foreground at low opacity:

In the `.file-active` or similar selector, change any `hsl(var(--primary))` usage to `hsl(var(--accent))` for the left bar indicator, keeping it subtle.

- [ ] **Step 2: Verify tab animation CSS (around lines 292-317) still works**

The existing `.tab-btn` and `.tab-close-btn` transition rules in index.css should continue to work since they reference generic properties (`background-color`, `color`, `transform`). No changes needed to transition properties — only the color values changed (via the token swap in Task 1).

- [ ] **Step 3: Commit (if any changes were made)**

```bash
git add apps/desktop/src/renderer/src/assets/styles/index.css
git commit -m "feat(design): update active file accent to warm tone"
```

---

### Task 7: Verification

**Files:** None (read-only checks)

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

Expected: Clean (or run `pnpm run fmt` to fix).

- [ ] **Step 4: Build**

```bash
pnpm run build
```

Expected: Successful build.

- [ ] **Step 5: Run tests**

```bash
pnpm run test
```

Expected: All tests pass including new SettingsDialog font mapping tests.

- [ ] **Step 6: Manual visual verification**

```bash
pnpm run dev
```

Check the following in the running app:

1. **Light mode:** warm stone/sand backgrounds, no blue tint anywhere in neutrals
2. **Dark mode:** clean neutral grays, no warmth in backgrounds
3. **Sidebar:** shares same background as content area, subtle border only
4. **Tab bar:** seamless with content, no tab dividers, subtle active underline
5. **Markdown headings:** no bottom borders on h1/h2, size+weight hierarchy only
6. **Code blocks:** warm-tinted background (light), border, proper Shiki highlighting in both themes
7. **Blockquotes:** left border only, no background fill
8. **Tables:** clean bottom borders, uppercase small headers, no striped rows
9. **Font selector:** shows 4 content fonts (Inter, Charter, System Sans, Georgia) and 3 code fonts (Geist Mono, SF Mono, JetBrains Mono)
10. **Switching fonts:** each option feels proportionate with the type scale
11. **Mermaid diagrams:** render and theme-sync correctly
12. **Settings sliders:** font size and line height work
13. **Scrollbars:** styled in both themes
14. **Search highlights:** visible in both themes
15. **Command palette:** styled consistently
16. **Reduced motion:** disable animations in system prefs, verify no transitions

- [ ] **Step 7: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix(design): visual verification fixes"
```
