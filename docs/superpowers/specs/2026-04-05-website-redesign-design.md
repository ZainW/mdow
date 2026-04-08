# Mdow Website Redesign

## Overview

Redesign the mdow marketing site (TanStack Start, Cloudflare Pages) from a minimal placeholder into a warm, approachable product landing page inspired by Notion and Linear. Upgrade the docs experience to feel cohesive with the new landing page.

**Design direction:** Warm & approachable — soft shadows, rounded corners, gentle gradients. Friendly and inviting without being childish.

**Tech stack (unchanged):** TanStack Start, React 19, Tailwind CSS v4, OKLCH color system, Cloudflare Pages.

---

## Landing Page

### 1. Hero Section

**Layout:** Centered text above a large browser-frame mockup, with a subtle warm background gradient.

**Content:**

- Headline: large, bold, warm tone — keep "A quiet place to read markdown" or refine during implementation
- Subline: one sentence explaining the value (beautiful rendering, syntax highlighting, distraction-free)
- Two CTAs side by side:
  - Primary (filled): "Download for free" → `/download`
  - Secondary (outlined): "Read the docs" → `/docs`

**Visual:** A stylized browser-frame mockup built entirely in HTML/CSS. Shows:

- File tree sidebar on the left with a few `.md` files
- Main content area rendering a markdown document with:
  - A heading and body text
  - A syntax-highlighted code block
  - A small mermaid diagram
- Uses the app's actual warm light theme colors

The mockup is not a screenshot — it's built with markup so it stays crisp at any resolution and can be themed.

**Background:** Subtle radial or linear gradient using warm OKLCH tones, fading from a slightly warmer center to the page background.

### 2. Feature Showcase

**Layout:** 3-4 features in alternating rows — text on one side, visual on the other, flipping each row.

**Features:**

1. **Syntax highlighting** — "Editor-quality highlighting for 30+ languages"
   - Visual: a small mockup showing a code block with colorful syntax highlighting
2. **Mermaid diagrams** — "Flowcharts, sequence diagrams, and more rendered inline"
   - Visual: a small mockup showing a rendered mermaid flowchart
3. **Light & dark themes** — "Warm tones in light mode, pure neutrals in dark. Follows your system."
   - Visual: side-by-side mini mockups showing the same content in light and dark mode
4. **File tree sidebar** — "Open a folder and browse your files in a collapsible tree"
   - Visual: a small mockup showing an expanded file tree with nested folders

**Card styling:** Each feature row has a soft card feel — subtle shadow, rounded corners, warm background. Not flat bordered boxes.

**Responsive:** Stacks vertically on mobile with visual above text.

### 3. Product Preview

**Layout:** Full-width section with a larger, detailed mockup of the app.

**Content:** A split view showing the app in both light and dark mode. The light mode mockup on the left, dark mode on the right, slightly overlapping or offset for visual interest.

**Background:** Subtle floating gradient or glow behind the mockups to create depth. Slightly different background color to distinguish this section.

### 4. Trust Section

**Layout:** Centered, compact section.

**Content:**

- Headline: "Free to download"
- Supported format badges: `.md`, `.mdx`, `.markdown`
- Platform icons: macOS, Windows, Linux
- GitHub link (include if repo is public, omit otherwise)

**Styling:** Minimal, clean. Badges/icons in muted tones with subtle borders.

### 5. Final CTA

**Layout:** Full-width section with warm background (slightly elevated from page background).

**Content:**

- Headline: "Ready to read markdown beautifully?"
- Subline: one short sentence
- Single primary CTA: "Download for free" → `/download`
- Small text below with platform availability

---

## Docs Upgrade

### Visual Cohesion

- Same warm OKLCH color palette as the landing page
- Sidebar: better spacing, hover states with soft transitions, category headers with subtle warm accent
- Active doc indicator: warm accent left-border or background, not just `bg-muted`
- Cards and code blocks use soft shadows consistent with landing page

### Typography & Content

- Larger, more comfortable prose line-height and font size
- Better heading hierarchy: more vertical breathing room between sections
- Code blocks: language label in top-right corner, copy-to-clipboard button, softer background color
- Inline code: subtle warm background tint

### Table of Contents

- Wire up the existing stubbed-out `DocsToc` component
- Parse headings from rendered markdown HTML (h2, h3)
- Sticky positioning on desktop (xl+ screens), right column
- Active section highlighting via IntersectionObserver as user scrolls
- On mobile/tablet: collapsible "On this page" section above content

### Search Polish

- Keep Cmd+K trigger and Fuse.js engine
- Larger modal with better visual hierarchy
- Search results show a content snippet/preview below the title
- Keyboard navigation hints visible (arrow keys, enter, esc)
- Subtle animations on open/close

### Mobile Experience

- Hamburger menu to toggle sidebar (currently hidden on mobile)
- TOC moves to collapsible section above article content
- Better touch targets on sidebar links and nav
- Search accessible from header on mobile

---

## Shared Design System Changes

### Color Refinements

Keep the existing OKLCH warm palette. Add:

- `--surface`: slightly elevated card background (between `--background` and `--card`)
- `--gradient-warm-start` / `--gradient-warm-end`: for section backgrounds
- Use `--accent` more intentionally for interactive hover states and indicators

### Component Upgrades

**Cards:**

- Replace flat `border bg-card` with soft shadow: `shadow-sm` + slightly warmer background
- Keep rounded-lg corners
- Subtle hover lift on interactive cards: `hover:shadow-md hover:-translate-y-0.5 transition-all`

**Buttons:**

- Primary: add subtle shadow for depth (`shadow-sm`)
- Secondary/outline: softer border color, warm hover background
- Consistent height and padding across the site

**Section dividers:**

- Use generous vertical spacing (py-20 to py-32) between landing page sections
- Optional: subtle gradient horizontal rule between major sections

### New Components Needed

1. **BrowserFrame** — stylized window chrome (title bar with dots, rounded corners, shadow) wrapping content to look like an app window
2. **FeatureRow** — alternating text + visual layout for feature showcase
3. **GradientSection** — section wrapper with configurable warm gradient background
4. **MobileNav** — hamburger-triggered sidebar for docs on mobile
5. **CopyButton** — clipboard copy for code blocks
6. **Scrollspy** — IntersectionObserver hook for TOC active state

---

## Files to Create/Modify

### New Files

- `src/components/browser-frame.tsx` — reusable app window mockup wrapper
- `src/components/feature-row.tsx` — alternating feature layout
- `src/components/gradient-section.tsx` — section with warm gradient bg
- `src/components/landing/hero.tsx` — new hero with mockup
- `src/components/landing/features.tsx` — feature showcase section
- `src/components/landing/product-preview.tsx` — large dual-theme mockup
- `src/components/landing/trust.tsx` — trust/badges section
- `src/components/landing/cta.tsx` — final CTA section
- `src/components/docs-mobile-nav.tsx` — mobile sidebar toggle
- `src/components/copy-button.tsx` — code block copy button

### Modified Files

- `src/routes/index.tsx` — compose new landing page sections
- `src/components/hero.tsx` — replace or remove (superseded by landing/hero.tsx)
- `src/styles/app.css` — add new CSS variables, gradient utilities
- `src/components/docs-layout.tsx` — integrate TOC, mobile nav, improved styling
- `src/components/docs-sidebar.tsx` — visual polish, mobile support
- `src/components/docs-toc.tsx` — implement scrollspy and heading parsing
- `src/components/docs-search.tsx` — visual polish, content previews
- `src/routes/docs/$.tsx` — pass heading data to TOC

### Unchanged

- Routing structure (no new routes)
- Content files (markdown docs stay the same)
- Build/deploy pipeline
- Download page (already functional)

---

## Out of Scope

- Pricing section (deferred)
- Comparison section (vs other tools)
- Blog or changelog redesign
- Authentication or user accounts
- Analytics integration
- Real app screenshots (using HTML/CSS mockups for now)
