# Design System Redesign

## Context

After switching fonts, the markdown rendering area lost its visual cohesion — heading sizes, spacing, code blocks, and body text no longer feel proportionate. Rather than patch individual values, this is a ground-up redesign of the visual layer: new color tokens, typographic scale, and component styling. The component architecture and all existing functionality (Shiki, Mermaid, file watchers, IPC, settings persistence) remain unchanged.

Work happens on a separate branch so the old and new designs can be compared side-by-side.

## Design Decisions

- **Reading experience:** Balanced/hybrid — comfortable for both prose and code
- **App chrome:** Seamless — sidebar, tab bar, and content share one continuous surface, no strong visual boundaries
- **Color palette (light):** Warm neutrals — stone/sand backgrounds, warm grays for text hierarchy
- **Color palette (dark):** Standard neutral — plain dark grays, no warmth carried over
- **Font curation:** Tight — 1 default content font + 1 default code font, 2-3 alternatives each, hand-tuned sizing per option
- **Headings:** Structural only — size and weight create hierarchy, no borders or decorative elements
- **Existing functionality preserved:** Shiki syntax highlighting/themes, Mermaid diagrams, all settings (theme toggle, font picker, size/line-height sliders), zoom, search, command palette

## Color Tokens

All values in OKLCH to match the existing approach.

### Light Theme

| Token                  | Hex Reference | Role                                            |
| ---------------------- | ------------- | ----------------------------------------------- |
| `--background`         | `#faf9f7`     | Page and sidebar background                     |
| `--foreground`         | `#1c1917`     | Primary text                                    |
| `--muted`              | `#f5f3f0`     | Subtle backgrounds (hover states, code bg)      |
| `--muted-foreground`   | `#57534e`     | Secondary text                                  |
| `--border`             | `#e2dfd8`     | Primary borders                                 |
| `--border-subtle`      | `#ebe8e2`     | Sidebar/tab dividers (lighter)                  |
| `--accent`             | `#b45309`     | Accent color (amber/terracotta)                 |
| `--primary`            | `#2563eb`     | Links, active states                            |
| `--primary-foreground` | `#ffffff`     | Text on primary backgrounds                     |
| `--card`               | `#faf9f7`     | Card backgrounds (same as bg for seamless feel) |
| `--popover`            | `#ffffff`     | Popover/dialog backgrounds                      |
| `--ring`               | `#2563eb`     | Focus ring                                      |
| `--destructive`        | `#dc2626`     | Error/delete actions                            |
| `--code-bg`            | `#f0ede8`     | Inline code and code block background           |
| `--code-border`        | `#e2dfd8`     | Code block border                               |
| `--blockquote-border`  | `#d6d3cc`     | Blockquote left border                          |

Sidebar tokens (`--sidebar`, `--sidebar-foreground`, etc.) collapse to use the same values as the main surface — this is the "seamless" approach. Keep the variables for compatibility but point them at the same base tokens.

### Dark Theme

| Token                  | Hex Reference | Role                                  |
| ---------------------- | ------------- | ------------------------------------- |
| `--background`         | `#1a1a1a`     | Page and sidebar background           |
| `--foreground`         | `#e5e5e5`     | Primary text                          |
| `--muted`              | `#242424`     | Subtle backgrounds                    |
| `--muted-foreground`   | `#a3a3a3`     | Secondary text                        |
| `--border`             | `#333333`     | Primary borders                       |
| `--border-subtle`      | `#2c2c2c`     | Subtle dividers                       |
| `--accent`             | `#f59e0b`     | Accent (amber, brighter for contrast) |
| `--primary`            | `#60a5fa`     | Links, active states                  |
| `--primary-foreground` | `#1a1a1a`     | Text on primary backgrounds           |
| `--card`               | `#1a1a1a`     | Card backgrounds                      |
| `--popover`            | `#222222`     | Popover/dialog backgrounds            |
| `--ring`               | `#60a5fa`     | Focus ring                            |
| `--destructive`        | `#ef4444`     | Error/delete actions                  |
| `--code-bg`            | `#242424`     | Code block background                 |
| `--code-border`        | `#333333`     | Code block border                     |
| `--blockquote-border`  | `#404040`     | Blockquote left border                |

## Typographic Scale

Base content font size: **15.5px**, line-height: **1.65**

| Element       | Size    | Weight | Letter-spacing | Line-height | Margin-top             | Margin-bottom |
| ------------- | ------- | ------ | -------------- | ----------- | ---------------------- | ------------- |
| h1            | 1.875em | 700    | -0.025em       | 1.2         | 2em (0 if first-child) | 0.6em         |
| h2            | 1.4em   | 650    | -0.02em        | 1.25        | 1.8em                  | 0.5em         |
| h3            | 1.15em  | 600    | -0.01em        | 1.3         | 1.5em                  | 0.4em         |
| h4            | 1em     | 600    | 0              | 1.4         | 1.3em                  | 0.3em         |
| p             | 1em     | 400    | 0              | 1.65        | 0                      | 1em           |
| code (inline) | 0.875em | 400    | 0              | —           | —                      | —             |
| code (block)  | 13.5px  | 400    | 0              | 1.5         | —                      | 1em           |
| li            | 1em     | 400    | 0              | 1.65        | 0.25em                 | 0             |

No heading borders or decorative elements. Hierarchy through size, weight, and negative letter-spacing only.

## Font Curation

### Content Fonts (reduced from 8 to 4)

| Name                | Type       | Notes                                                                |
| ------------------- | ---------- | -------------------------------------------------------------------- |
| **Inter** (default) | Sans-serif | Already bundled as variable font. The type scale is tuned for Inter. |
| **Charter**         | Serif      | Already available. Good for long-form reading.                       |
| **System Sans**     | Sans-serif | Maps to -apple-system / system-ui. Zero-cost fallback.               |
| **Georgia**         | Serif      | Widely available, good web serif. Simple fallback option.            |

### Code Fonts (reduced from 7 to 3)

| Name                     | Type      | Notes                                    |
| ------------------------ | --------- | ---------------------------------------- |
| **Geist Mono** (default) | Monospace | Already bundled as variable font.        |
| **SF Mono**              | Monospace | macOS system mono. Clean and compact.    |
| **JetBrains Mono**       | Monospace | Popular with developers, slightly wider. |

Each font pairing gets minor sizing adjustments (via CSS variable overrides) so they feel proportionate against the same typographic scale. For example, Charter at the same `px` size as Inter reads slightly larger — the override compensates.

## Component Styling Changes

### Sidebar

- Background: `var(--background)` (same as content — seamless)
- Border-right: `1px solid var(--border-subtle)`
- Item hover: `var(--muted)` background
- Active item: `var(--muted)` background + `font-weight: 500`
- Section labels: 11px uppercase, `var(--muted-foreground)` color

### Tab Bar

- Background: `var(--background)` (seamless)
- Border-bottom: `1px solid var(--border-subtle)`
- Active tab: `font-weight: 500`, `var(--foreground)` color, no bottom-border indicator
- Inactive tab: `var(--muted-foreground)` color
- Tab border-radius: 6px, hover background: `var(--muted)`

### Markdown Content

- Max-width: 720px (unchanged)
- Padding: 48px horizontal, 48px top, 96px bottom
- No heading borders — remove existing `border-bottom` on h1/h2
- Blockquotes: 3px left border only, no background fill
- Code blocks: `var(--code-bg)` background, `1px solid var(--code-border)`, 8px radius
- Tables: no alternating row background, just bottom borders. Header row uses 2px bottom border + uppercase small text
- HR: 1px `var(--border)`, 2em vertical margin
- Images: 8px border-radius (unchanged)

### Dialogs (Settings, Shortcuts)

- Popover background: `var(--popover)`
- Keep existing Base UI dialog structure
- Border: `1px solid var(--border)`

### Buttons

- Keep existing CVA variant structure
- Update color values to use new tokens
- Default variant: `var(--muted)` background, `var(--foreground)` text
- Ghost variant: transparent background, `var(--muted-foreground)` text

### Scrollbars

- Thumb: `var(--border)` at rest, `var(--muted-foreground)` on hover
- 6px width (4px in sidebar)

## Spacing Scale

Define a spacing scale as CSS custom properties for consistency:

| Token        | Value | Usage                             |
| ------------ | ----- | --------------------------------- |
| `--space-1`  | 4px   | Tight gaps (icon-to-label)        |
| `--space-2`  | 8px   | Default component padding         |
| `--space-3`  | 12px  | Sidebar item padding, tab padding |
| `--space-4`  | 16px  | Code block padding, section gaps  |
| `--space-6`  | 24px  | Card padding, dialog sections     |
| `--space-8`  | 32px  | Large section gaps                |
| `--space-12` | 48px  | Content area padding              |

## Radius Scale

| Token         | Value        |
| ------------- | ------------ |
| `--radius`    | 0.5rem (8px) |
| `--radius-sm` | 4px          |
| `--radius-md` | 6px          |
| `--radius-lg` | 8px          |
| `--radius-xl` | 12px         |

## What Does NOT Change

- Component structure (Base UI primitives, CVA variants)
- Zustand store shape and state management
- IPC bridge and preload layer
- Shiki syntax highlighting and theme integration
- Mermaid diagram rendering and theme sync
- Settings dialog controls (theme picker, font select, size/line-height sliders)
- Command palette functionality
- File watching and auto-refresh
- Sidebar resize behavior
- Keyboard shortcuts
- Search bar functionality
- Zoom controls
- Animation easing function (`--ease-out-ui`)
- Reduced motion media query support

## Files to Modify

1. `apps/desktop/src/renderer/src/assets/styles/index.css` — color tokens, theme definitions, Tailwind inline config
2. `apps/desktop/src/renderer/src/assets/styles/markdown.css` — full restyle of markdown rendering
3. `apps/desktop/src/renderer/src/components/ui/button.tsx` — update variant colors
4. `apps/desktop/src/renderer/src/components/ui/sidebar.tsx` — simplify to seamless styling
5. `apps/desktop/src/renderer/src/components/Sidebar.tsx` — update any hardcoded styles
6. `apps/desktop/src/renderer/src/components/TabBar.tsx` — restyle tabs
7. `apps/desktop/src/renderer/src/components/MarkdownView.tsx` — update font mappings (trim options)
8. `apps/desktop/src/renderer/src/components/SettingsDialog.tsx` — update font option lists
9. `apps/desktop/src/renderer/src/store/app-store.ts` — update default font values if needed
10. Any other component files that hardcode color values or reference removed tokens

## Verification

1. `pnpm run typecheck` — no type errors
2. `pnpm run lint` — no lint errors
3. `pnpm run build` — builds successfully
4. `pnpm run test` — all tests pass
5. Manual: open the app in dev mode, verify:
   - Light and dark themes render correctly
   - Sidebar, tab bar, and content area feel seamless
   - Markdown headings, code blocks, blockquotes, tables, lists render with new styles
   - Shiki syntax highlighting works in both themes
   - Mermaid diagrams render and theme-sync correctly
   - Settings dialog: theme picker, font selects, sliders all functional
   - Each font option looks proportionate
   - Scrollbars styled correctly in both themes
   - Reduced motion preference respected
   - Command palette, search bar, shortcuts dialog all styled consistently
