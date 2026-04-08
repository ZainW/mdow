# Mdow Website Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the mdow marketing site from a basic placeholder into a warm, polished product landing page (Notion/Linear-inspired) and upgrade the docs experience to feel cohesive with it.

**Architecture:** TanStack Start app at `apps/web` using Tailwind CSS v4 with OKLCH variables. Build a small set of reusable primitives (BrowserFrame, GradientSection, FeatureRow, CopyButton, scrollspy hook), then compose them into landing-page sections and apply polish to docs components. No new routes or backend changes.

**Tech Stack:** React 19, TanStack Router/Start, Tailwind CSS v4, Vitest + jsdom + @testing-library/react, oxlint, oxfmt, tsgo.

**Working directory:** All paths in this plan are relative to `/Users/zain/projects/mdow`. All commands run from the repo root unless noted. Use `pnpm run --filter web <script>` to target the web app.

**Spec:** `docs/superpowers/specs/2026-04-05-website-redesign-design.md`

---

## File Structure

### New files

- `apps/web/src/components/browser-frame.tsx` — stylized window chrome wrapper
- `apps/web/src/components/gradient-section.tsx` — section with warm gradient background
- `apps/web/src/components/feature-row.tsx` — alternating text+visual feature row
- `apps/web/src/components/copy-button.tsx` — clipboard copy button
- `apps/web/src/components/landing/hero.tsx` — new landing hero
- `apps/web/src/components/landing/features.tsx` — feature showcase
- `apps/web/src/components/landing/product-preview.tsx` — dual-theme preview
- `apps/web/src/components/landing/trust.tsx` — trust badges section
- `apps/web/src/components/landing/cta.tsx` — final CTA section
- `apps/web/src/components/landing/mock-markdown.tsx` — reusable rendered-markdown mockup content
- `apps/web/src/components/docs-mobile-nav.tsx` — mobile sidebar drawer
- `apps/web/src/hooks/use-scrollspy.ts` — IntersectionObserver hook for TOC
- `apps/web/src/lib/extract-headings.ts` — parse h2/h3 from rendered HTML
- `apps/web/src/components/__tests__/browser-frame.test.tsx`
- `apps/web/src/components/__tests__/feature-row.test.tsx`
- `apps/web/src/components/__tests__/copy-button.test.tsx`
- `apps/web/src/lib/__tests__/extract-headings.test.ts`
- `apps/web/src/hooks/__tests__/use-scrollspy.test.ts`

### Modified files

- `apps/web/src/styles/app.css` — add `--surface`, gradient tokens, prose tweaks
- `apps/web/src/routes/index.tsx` — compose new landing sections
- `apps/web/src/components/hero.tsx` — delete (superseded)
- `apps/web/src/components/docs-layout.tsx` — wire mobile nav, polish spacing
- `apps/web/src/components/docs-sidebar.tsx` — warm active indicator, polish
- `apps/web/src/components/docs-toc.tsx` — render with active state from scrollspy
- `apps/web/src/components/docs-search.tsx` — visual polish, content snippets, kbd hints
- `apps/web/src/routes/docs/$.tsx` — extract headings and pass to layout
- `apps/web/src/components/site-header.tsx` — add mobile menu trigger affordance

---

## Conventions for every task

Every task ends with:

1. `pnpm run --filter web typecheck` → expect "no errors"
2. `pnpm run --filter web lint` → expect clean
3. `pnpm run --filter web fmt` → reformat files
4. `git add <files>` then `git commit -m "<message>"`

When a task adds tests, also run `pnpm run --filter web test` (or `pnpm run --filter web test -- -t '<test name>'` for a single test) before commit and expect green.

The dev server (`pnpm run --filter web dev`) is useful for visual sanity checks but is not required for green CI.

---

## Phase 1 — Design system foundations

### Task 1: Add design tokens (surface color + gradient utilities)

**Files:**

- Modify: `apps/web/src/styles/app.css`

- [ ] **Step 1: Add `--surface` token to `:root` and `.dark` blocks**

In `apps/web/src/styles/app.css`, inside the `:root` block (light theme), add a slightly elevated warm surface color right after `--card-foreground`:

```css
--surface: oklch(0.96 0.008 70);
--surface-foreground: oklch(0.13 0.02 50);
```

In the `.dark` block, add:

```css
--surface: oklch(0.17 0 0);
--surface-foreground: oklch(0.92 0 0);
```

- [ ] **Step 2: Expose `--surface` to Tailwind via `@theme inline`**

In the `@theme inline` block at the top of the same file, add (after `--color-card-foreground`):

```css
--color-surface: var(--surface);
--color-surface-foreground: var(--surface-foreground);
```

- [ ] **Step 3: Add gradient + soft-shadow utilities**

Append a new `@layer utilities` block at the bottom of `apps/web/src/styles/app.css`:

```css
@layer utilities {
  .bg-warm-gradient {
    background-image: radial-gradient(
      ellipse 80% 60% at 50% 0%,
      oklch(0.95 0.03 70 / 0.7),
      transparent 70%
    );
  }
  .dark .bg-warm-gradient {
    background-image: radial-gradient(
      ellipse 80% 60% at 50% 0%,
      oklch(0.25 0.04 60 / 0.4),
      transparent 70%
    );
  }
  .bg-section-warm {
    background-color: var(--surface);
  }
  .shadow-soft {
    box-shadow:
      0 1px 2px 0 oklch(0 0 0 / 0.04),
      0 4px 16px -4px oklch(0 0 0 / 0.08);
  }
  .shadow-soft-lg {
    box-shadow:
      0 1px 2px 0 oklch(0 0 0 / 0.05),
      0 12px 40px -8px oklch(0 0 0 / 0.12);
  }
}
```

- [ ] **Step 4: Verify**

Run:

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
pnpm run --filter web build
```

Expect all green. Build verifies CSS still compiles under Tailwind v4.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/styles/app.css
git commit -m "feat(web): add surface token and warm gradient/shadow utilities"
```

---

### Task 2: BrowserFrame component

A reusable window-chrome wrapper used by every landing-page mockup.

**Files:**

- Create: `apps/web/src/components/browser-frame.tsx`
- Create: `apps/web/src/components/__tests__/browser-frame.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/components/__tests__/browser-frame.test.tsx`:

```tsx
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { BrowserFrame } from '../browser-frame'

describe('BrowserFrame', () => {
  it('renders children inside the frame', () => {
    render(
      <BrowserFrame>
        <div>hello</div>
      </BrowserFrame>,
    )
    expect(screen.getByText('hello')).toBeInTheDocument()
  })

  it('renders the title in the chrome when provided', () => {
    render(<BrowserFrame title="readme.md">content</BrowserFrame>)
    expect(screen.getByText('readme.md')).toBeInTheDocument()
  })

  it('applies extra className to the outer wrapper', () => {
    const { container } = render(<BrowserFrame className="custom-x">x</BrowserFrame>)
    expect(container.firstChild).toHaveClass('custom-x')
  })
})
```

- [ ] **Step 2: Run test and confirm it fails**

```bash
pnpm run --filter web test -- -t 'BrowserFrame'
```

Expected: FAIL with module not found.

- [ ] **Step 3: Implement the component**

Create `apps/web/src/components/browser-frame.tsx`:

```tsx
import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface BrowserFrameProps {
  children: ReactNode
  title?: string
  className?: string
}

export function BrowserFrame({ children, title, className }: BrowserFrameProps) {
  return (
    <div
      className={cn(
        'overflow-hidden rounded-xl border border-border-subtle bg-card shadow-soft-lg',
        className,
      )}
    >
      <div className="flex items-center gap-2 border-b border-border-subtle bg-surface px-4 py-2.5">
        <div className="flex gap-1.5">
          <span className="h-3 w-3 rounded-full bg-[oklch(0.78_0.16_25)]" />
          <span className="h-3 w-3 rounded-full bg-[oklch(0.85_0.16_85)]" />
          <span className="h-3 w-3 rounded-full bg-[oklch(0.78_0.16_150)]" />
        </div>
        {title && <span className="ml-3 text-xs text-muted-foreground select-none">{title}</span>}
      </div>
      <div className="bg-card">{children}</div>
    </div>
  )
}
```

- [ ] **Step 4: Run test and confirm pass**

```bash
pnpm run --filter web test -- -t 'BrowserFrame'
```

Expected: PASS (3 tests).

- [ ] **Step 5: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/browser-frame.tsx apps/web/src/components/__tests__/browser-frame.test.tsx
git commit -m "feat(web): add BrowserFrame primitive for landing page mockups"
```

---

### Task 3: GradientSection component

A section wrapper that paints a warm gradient and constrains content width.

**Files:**

- Create: `apps/web/src/components/gradient-section.tsx`

- [ ] **Step 1: Implement**

Create `apps/web/src/components/gradient-section.tsx`:

```tsx
import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface GradientSectionProps {
  children: ReactNode
  className?: string
  variant?: 'gradient' | 'surface' | 'plain'
  innerClassName?: string
}

export function GradientSection({
  children,
  className,
  variant = 'plain',
  innerClassName,
}: GradientSectionProps) {
  return (
    <section
      className={cn(
        'relative w-full',
        variant === 'gradient' && 'bg-warm-gradient',
        variant === 'surface' && 'bg-section-warm',
        className,
      )}
    >
      <div className={cn('mx-auto max-w-6xl px-6 py-20 md:py-28', innerClassName)}>{children}</div>
    </section>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/gradient-section.tsx
git commit -m "feat(web): add GradientSection wrapper"
```

---

### Task 4: MockMarkdown content (shared)

Reusable rendered-markdown mockup content used by Hero and Product Preview.

**Files:**

- Create: `apps/web/src/components/landing/mock-markdown.tsx`

- [ ] **Step 1: Implement**

Create `apps/web/src/components/landing/mock-markdown.tsx`:

```tsx
export function MockMarkdown() {
  return (
    <div className="grid grid-cols-[140px_1fr] min-h-[320px]">
      {/* File tree */}
      <aside className="border-r border-border-subtle bg-surface px-3 py-4 text-xs">
        <div className="mb-2 font-semibold text-muted-foreground/80 uppercase tracking-wider text-[10px]">
          Project
        </div>
        <ul className="space-y-1">
          <li className="text-muted-foreground">📁 docs</li>
          <li className="ml-3 text-foreground font-medium">📄 readme.md</li>
          <li className="ml-3 text-muted-foreground">📄 install.md</li>
          <li className="ml-3 text-muted-foreground">📄 features.md</li>
          <li className="text-muted-foreground">📁 notes</li>
        </ul>
      </aside>
      {/* Rendered content */}
      <div className="px-6 py-5 text-sm leading-relaxed">
        <h2 className="text-lg font-bold tracking-tight mb-2">Getting started</h2>
        <p className="text-muted-foreground mb-3">
          A quick overview of how mdow renders your markdown beautifully.
        </p>
        <pre className="rounded-md bg-surface border border-border-subtle px-3 py-2 mb-3 text-[11px] font-mono overflow-hidden">
          <span className="text-[oklch(0.55_0.17_260)]">function</span>{' '}
          <span className="text-[oklch(0.62_0.16_55)]">render</span>(){' '}
          <span className="text-muted-foreground">{'{'}</span>
          {'\n  '}
          <span className="text-[oklch(0.55_0.17_260)]">return</span>{' '}
          <span className="text-[oklch(0.6_0.15_160)]">'hello'</span>
          {'\n'}
          <span className="text-muted-foreground">{'}'}</span>
        </pre>
        <div className="rounded-md border border-border-subtle bg-surface px-3 py-3">
          <div className="flex items-center justify-center gap-2 text-[10px] text-muted-foreground">
            <span className="rounded border border-border px-2 py-0.5">Idea</span>
            <span>→</span>
            <span className="rounded border border-border px-2 py-0.5">Write</span>
            <span>→</span>
            <span className="rounded border border-border px-2 py-0.5">Render</span>
          </div>
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/landing/mock-markdown.tsx
git commit -m "feat(web): add MockMarkdown landing content"
```

---

## Phase 2 — Landing page

### Task 5: Landing Hero

**Files:**

- Create: `apps/web/src/components/landing/hero.tsx`

- [ ] **Step 1: Implement**

Create `apps/web/src/components/landing/hero.tsx`:

```tsx
import { Link } from '@tanstack/react-router'
import { BrowserFrame } from '~/components/browser-frame'
import { MockMarkdown } from './mock-markdown'

export function LandingHero() {
  return (
    <section className="relative overflow-hidden bg-warm-gradient">
      <div className="mx-auto max-w-6xl px-6 pt-20 pb-16 md:pt-28 md:pb-24">
        <div className="mx-auto max-w-3xl text-center">
          <h1 className="text-4xl font-bold tracking-tight text-balance sm:text-5xl md:text-6xl">
            A quiet place to read markdown
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground text-balance">
            Beautiful rendering, syntax highlighting, and a reading experience that gets out of the
            way. Free for Mac, Windows, and Linux.
          </p>
          <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
            <Link
              to="/download"
              className="inline-flex h-11 items-center rounded-lg bg-primary px-7 text-sm font-medium text-primary-foreground shadow-soft transition-all hover:-translate-y-0.5 hover:bg-primary/95 hover:shadow-soft-lg"
            >
              Download for free
            </Link>
            <Link
              to="/docs"
              className="inline-flex h-11 items-center rounded-lg border border-border bg-card px-7 text-sm font-medium transition-colors hover:bg-muted"
            >
              Read the docs
            </Link>
          </div>
        </div>
        <div className="mx-auto mt-16 max-w-4xl">
          <BrowserFrame title="readme.md — Mdow">
            <MockMarkdown />
          </BrowserFrame>
        </div>
      </div>
    </section>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/landing/hero.tsx
git commit -m "feat(web): add new landing Hero with browser-frame mockup"
```

---

### Task 6: FeatureRow component

**Files:**

- Create: `apps/web/src/components/feature-row.tsx`
- Create: `apps/web/src/components/__tests__/feature-row.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/components/__tests__/feature-row.test.tsx`:

```tsx
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { FeatureRow } from '../feature-row'

describe('FeatureRow', () => {
  it('renders title and description', () => {
    render(
      <FeatureRow title="Hello" description="World" align="left">
        <div data-testid="visual">visual</div>
      </FeatureRow>,
    )
    expect(screen.getByText('Hello')).toBeInTheDocument()
    expect(screen.getByText('World')).toBeInTheDocument()
    expect(screen.getByTestId('visual')).toBeInTheDocument()
  })

  it('reverses order when align is right', () => {
    const { container } = render(
      <FeatureRow title="t" description="d" align="right">
        <div>v</div>
      </FeatureRow>,
    )
    expect(container.querySelector('.md\\:flex-row-reverse')).not.toBeNull()
  })
})
```

- [ ] **Step 2: Run test and confirm fail**

```bash
pnpm run --filter web test -- -t 'FeatureRow'
```

Expected: FAIL.

- [ ] **Step 3: Implement**

Create `apps/web/src/components/feature-row.tsx`:

```tsx
import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface FeatureRowProps {
  title: string
  description: string
  align: 'left' | 'right'
  children: ReactNode
}

export function FeatureRow({ title, description, align, children }: FeatureRowProps) {
  return (
    <div
      className={cn(
        'flex flex-col gap-10 md:flex-row md:items-center md:gap-16',
        align === 'right' && 'md:flex-row-reverse',
      )}
    >
      <div className="flex-1">
        <h3 className="text-2xl font-bold tracking-tight sm:text-3xl text-balance">{title}</h3>
        <p className="mt-3 text-muted-foreground text-balance">{description}</p>
      </div>
      <div className="flex-1">{children}</div>
    </div>
  )
}
```

- [ ] **Step 4: Run test and confirm pass**

```bash
pnpm run --filter web test -- -t 'FeatureRow'
```

Expected: PASS.

- [ ] **Step 5: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/feature-row.tsx apps/web/src/components/__tests__/feature-row.test.tsx
git commit -m "feat(web): add FeatureRow alternating layout primitive"
```

---

### Task 7: Landing Features section

**Files:**

- Create: `apps/web/src/components/landing/features.tsx`

- [ ] **Step 1: Implement**

Create `apps/web/src/components/landing/features.tsx`:

```tsx
import { BrowserFrame } from '~/components/browser-frame'
import { FeatureRow } from '~/components/feature-row'
import { GradientSection } from '~/components/gradient-section'

function CodeMockup() {
  return (
    <BrowserFrame title="example.ts">
      <div className="px-5 py-4 font-mono text-xs leading-relaxed">
        <div>
          <span className="text-[oklch(0.55_0.17_260)]">import</span>{' '}
          <span className="text-foreground">{'{ render }'}</span>{' '}
          <span className="text-[oklch(0.55_0.17_260)]">from</span>{' '}
          <span className="text-[oklch(0.6_0.15_160)]">'mdow'</span>
        </div>
        <div className="mt-2">
          <span className="text-[oklch(0.55_0.17_260)]">const</span>{' '}
          <span className="text-foreground">html</span>{' '}
          <span className="text-muted-foreground">=</span>{' '}
          <span className="text-[oklch(0.62_0.16_55)]">render</span>(
          <span className="text-[oklch(0.6_0.15_160)]">'# Hello'</span>)
        </div>
        <div className="mt-2 text-muted-foreground">// Beautifully highlighted</div>
      </div>
    </BrowserFrame>
  )
}

function MermaidMockup() {
  return (
    <BrowserFrame title="diagram.md">
      <div className="px-6 py-6">
        <div className="flex items-center justify-center gap-3 text-xs">
          <div className="rounded-md border border-border bg-surface px-3 py-2 shadow-soft">
            Idea
          </div>
          <div className="text-muted-foreground">→</div>
          <div className="rounded-md border border-border bg-surface px-3 py-2 shadow-soft">
            Draft
          </div>
          <div className="text-muted-foreground">→</div>
          <div className="rounded-md border border-primary/40 bg-primary/10 px-3 py-2 shadow-soft text-foreground">
            Publish
          </div>
        </div>
      </div>
    </BrowserFrame>
  )
}

function ThemeMockup() {
  return (
    <div className="grid grid-cols-2 gap-3">
      <BrowserFrame className="bg-[oklch(0.98_0.005_70)]">
        <div className="px-4 py-4 text-xs">
          <div className="font-semibold text-[oklch(0.13_0.02_50)]">Light</div>
          <div className="text-[oklch(0.45_0.015_50)] mt-1">Warm stone tones.</div>
        </div>
      </BrowserFrame>
      <BrowserFrame className="bg-[oklch(0.14_0_0)]">
        <div className="px-4 py-4 text-xs">
          <div className="font-semibold text-[oklch(0.92_0_0)]">Dark</div>
          <div className="text-[oklch(0.65_0_0)] mt-1">Pure neutrals.</div>
        </div>
      </BrowserFrame>
    </div>
  )
}

function FileTreeMockup() {
  return (
    <BrowserFrame title="Project">
      <div className="px-5 py-4 text-xs leading-loose">
        <div className="font-medium">📁 my-notes</div>
        <div className="ml-4 text-muted-foreground">📁 daily</div>
        <div className="ml-8 text-muted-foreground">📄 monday.md</div>
        <div className="ml-8 text-foreground font-medium">📄 tuesday.md</div>
        <div className="ml-4 text-muted-foreground">📁 projects</div>
        <div className="ml-8 text-muted-foreground">📄 readme.md</div>
        <div className="ml-4 text-muted-foreground">📄 todo.md</div>
      </div>
    </BrowserFrame>
  )
}

export function LandingFeatures() {
  return (
    <GradientSection innerClassName="space-y-24 md:space-y-32">
      <FeatureRow
        title="Editor-quality syntax highlighting"
        description="Powered by Shiki with the same engine VS Code uses. 30+ languages, themed for both light and dark mode out of the box."
        align="left"
      >
        <CodeMockup />
      </FeatureRow>
      <FeatureRow
        title="Mermaid diagrams, rendered inline"
        description="Flowcharts, sequence diagrams, state machines — write them in plain text and watch them render where you'd expect."
        align="right"
      >
        <MermaidMockup />
      </FeatureRow>
      <FeatureRow
        title="Light and dark, just right"
        description="Warm stone tones in light mode, pure neutrals in dark. Follows your system, switches instantly, never fights your eyes."
        align="left"
      >
        <ThemeMockup />
      </FeatureRow>
      <FeatureRow
        title="A file tree for the way you actually write"
        description="Open any folder and browse your markdown like a familiar workspace. Collapsible, keyboard-friendly, and out of your way."
        align="right"
      >
        <FileTreeMockup />
      </FeatureRow>
    </GradientSection>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/landing/features.tsx
git commit -m "feat(web): add landing Features section with alternating rows"
```

---

### Task 8: Product Preview section

**Files:**

- Create: `apps/web/src/components/landing/product-preview.tsx`

- [ ] **Step 1: Implement**

Create `apps/web/src/components/landing/product-preview.tsx`:

```tsx
import { BrowserFrame } from '~/components/browser-frame'
import { GradientSection } from '~/components/gradient-section'
import { MockMarkdown } from './mock-markdown'

export function LandingProductPreview() {
  return (
    <GradientSection variant="surface" innerClassName="text-center">
      <h2 className="mx-auto max-w-2xl text-3xl font-bold tracking-tight sm:text-4xl text-balance">
        Built for long reading sessions
      </h2>
      <p className="mx-auto mt-4 max-w-xl text-muted-foreground text-balance">
        Whether you're skimming notes or diving into a long-form document, mdow keeps things
        comfortable, no matter the time of day.
      </p>
      <div className="relative mx-auto mt-14 max-w-5xl">
        <div className="grid gap-6 md:grid-cols-2 md:gap-4">
          <div className="md:translate-y-4">
            <BrowserFrame title="readme.md — Light">
              <div className="bg-[oklch(0.98_0.005_70)] text-[oklch(0.13_0.02_50)]">
                <MockMarkdown />
              </div>
            </BrowserFrame>
          </div>
          <div className="md:-translate-y-4">
            <BrowserFrame title="readme.md — Dark">
              <div className="bg-[oklch(0.14_0_0)] text-[oklch(0.92_0_0)]">
                <MockMarkdown />
              </div>
            </BrowserFrame>
          </div>
        </div>
      </div>
    </GradientSection>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/landing/product-preview.tsx
git commit -m "feat(web): add landing ProductPreview dual-theme section"
```

---

### Task 9: Trust section

**Files:**

- Create: `apps/web/src/components/landing/trust.tsx`

- [ ] **Step 1: Implement**

Create `apps/web/src/components/landing/trust.tsx`:

```tsx
import { GradientSection } from '~/components/gradient-section'

const formats = ['.md', '.markdown', '.mdx']
const platforms = [
  { name: 'macOS', icon: '🍎' },
  { name: 'Windows', icon: '🪟' },
  { name: 'Linux', icon: '🐧' },
]

export function LandingTrust() {
  return (
    <GradientSection innerClassName="text-center py-16 md:py-20">
      <p className="text-sm font-medium uppercase tracking-wider text-muted-foreground">
        Free to download
      </p>
      <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
        {formats.map((f) => (
          <span
            key={f}
            className="rounded-full border border-border bg-card px-4 py-1.5 font-mono text-xs text-muted-foreground"
          >
            {f}
          </span>
        ))}
      </div>
      <div className="mt-6 flex flex-wrap items-center justify-center gap-6 text-sm text-muted-foreground">
        {platforms.map((p) => (
          <div key={p.name} className="flex items-center gap-2">
            <span className="text-base" aria-hidden>
              {p.icon}
            </span>
            <span>{p.name}</span>
          </div>
        ))}
      </div>
    </GradientSection>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/landing/trust.tsx
git commit -m "feat(web): add landing Trust section with format and platform badges"
```

---

### Task 10: Final CTA section

**Files:**

- Create: `apps/web/src/components/landing/cta.tsx`

- [ ] **Step 1: Implement**

Create `apps/web/src/components/landing/cta.tsx`:

```tsx
import { Link } from '@tanstack/react-router'
import { GradientSection } from '~/components/gradient-section'

export function LandingCta() {
  return (
    <GradientSection variant="surface" innerClassName="text-center">
      <h2 className="mx-auto max-w-2xl text-3xl font-bold tracking-tight sm:text-4xl text-balance">
        Ready to read markdown beautifully?
      </h2>
      <p className="mx-auto mt-4 max-w-xl text-muted-foreground text-balance">
        Download mdow and turn any folder of markdown into a calm reading experience.
      </p>
      <div className="mt-10 flex justify-center">
        <Link
          to="/download"
          className="inline-flex h-12 items-center rounded-lg bg-primary px-8 text-sm font-medium text-primary-foreground shadow-soft transition-all hover:-translate-y-0.5 hover:bg-primary/95 hover:shadow-soft-lg"
        >
          Download for free
        </Link>
      </div>
      <p className="mt-4 text-xs text-muted-foreground">Available for macOS, Windows, and Linux</p>
    </GradientSection>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/landing/cta.tsx
git commit -m "feat(web): add landing final CTA section"
```

---

### Task 11: Compose new landing page

**Files:**

- Modify: `apps/web/src/routes/index.tsx`
- Delete: `apps/web/src/components/hero.tsx`

- [ ] **Step 1: Replace `apps/web/src/routes/index.tsx`**

Overwrite the file with:

```tsx
import { createFileRoute } from '@tanstack/react-router'
import { LandingHero } from '~/components/landing/hero'
import { LandingFeatures } from '~/components/landing/features'
import { LandingProductPreview } from '~/components/landing/product-preview'
import { LandingTrust } from '~/components/landing/trust'
import { LandingCta } from '~/components/landing/cta'

export const Route = createFileRoute('/')({
  component: HomePage,
})

function HomePage() {
  return (
    <>
      <LandingHero />
      <LandingFeatures />
      <LandingProductPreview />
      <LandingTrust />
      <LandingCta />
    </>
  )
}
```

- [ ] **Step 2: Delete the old hero**

```bash
rm apps/web/src/components/hero.tsx
```

- [ ] **Step 3: Verify**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
pnpm run --filter web build
```

Expected: clean across all four. The build catches any prerender issues.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/routes/index.tsx apps/web/src/components/hero.tsx
git commit -m "feat(web): wire up new landing page composition"
```

---

## Phase 3 — Docs upgrade

### Task 12: CopyButton component

**Files:**

- Create: `apps/web/src/components/copy-button.tsx`
- Create: `apps/web/src/components/__tests__/copy-button.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/components/__tests__/copy-button.test.tsx`:

```tsx
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { CopyButton } from '../copy-button'

describe('CopyButton', () => {
  it('copies value to the clipboard on click', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })
    render(<CopyButton value="hello" />)
    fireEvent.click(screen.getByRole('button'))
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('hello'))
  })

  it('shows copied state briefly after click', async () => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    })
    render(<CopyButton value="x" />)
    fireEvent.click(screen.getByRole('button'))
    await waitFor(() => expect(screen.getByLabelText('Copied')).toBeInTheDocument())
  })
})
```

- [ ] **Step 2: Run test and confirm fail**

```bash
pnpm run --filter web test -- -t 'CopyButton'
```

Expected: FAIL.

- [ ] **Step 3: Implement**

Create `apps/web/src/components/copy-button.tsx`:

```tsx
import { useState } from 'react'
import { cn } from '~/lib/utils'

interface CopyButtonProps {
  value: string
  className?: string
}

export function CopyButton({ value, className }: CopyButtonProps) {
  const [copied, setCopied] = useState(false)

  async function onClick() {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      // ignore — older browsers without clipboard permission
    }
  }

  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={copied ? 'Copied' : 'Copy to clipboard'}
      className={cn(
        'inline-flex h-7 items-center rounded-md border border-border-subtle bg-surface px-2 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-muted',
        className,
      )}
    >
      {copied ? 'Copied' : 'Copy'}
    </button>
  )
}
```

- [ ] **Step 4: Run test and confirm pass**

```bash
pnpm run --filter web test -- -t 'CopyButton'
```

Expected: PASS.

- [ ] **Step 5: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/copy-button.tsx apps/web/src/components/__tests__/copy-button.test.tsx
git commit -m "feat(web): add CopyButton clipboard component"
```

---

### Task 13: Polish docs sidebar

**Files:**

- Modify: `apps/web/src/components/docs-sidebar.tsx`

- [ ] **Step 1: Replace the file contents**

Overwrite `apps/web/src/components/docs-sidebar.tsx`:

```tsx
import { Link } from '@tanstack/react-router'
import type { DocMeta } from '~/lib/content'
import { groupByCategory } from '~/lib/content'
import { cn } from '~/lib/utils'

interface DocsSidebarProps {
  docs: DocMeta[]
  currentSlug: string
}

export function DocsSidebar({ docs, currentSlug }: DocsSidebarProps) {
  const groups = groupByCategory(docs)

  return (
    <nav className="space-y-7 text-sm">
      {groups.map((group) => (
        <div key={group.category}>
          <h4 className="mb-2 px-3 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            {group.category}
          </h4>
          <ul className="space-y-0.5">
            {group.docs.map((doc) => {
              const active = doc.slug === currentSlug
              return (
                <li key={doc.slug}>
                  <Link
                    to="/docs/$"
                    params={{ _splat: doc.slug }}
                    className={cn(
                      'relative block rounded-md px-3 py-1.5 transition-colors',
                      active
                        ? 'bg-surface font-medium text-foreground before:absolute before:left-0 before:top-1.5 before:bottom-1.5 before:w-0.5 before:rounded-full before:bg-primary'
                        : 'text-muted-foreground hover:bg-surface/60 hover:text-foreground',
                    )}
                  >
                    {doc.title}
                  </Link>
                </li>
              )
            })}
          </ul>
        </div>
      ))}
    </nav>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/docs-sidebar.tsx
git commit -m "feat(web): polish docs sidebar with warm active indicator"
```

---

### Task 14: extractHeadings utility

**Files:**

- Create: `apps/web/src/lib/extract-headings.ts`
- Create: `apps/web/src/lib/__tests__/extract-headings.test.ts`

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/lib/__tests__/extract-headings.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { extractHeadings } from '../extract-headings'

describe('extractHeadings', () => {
  it('returns h2 and h3 with id, text, and level', () => {
    const html = `
      <h2 id="intro">Intro</h2>
      <p>x</p>
      <h3 id="details">Details</h3>
      <h4 id="ignored">Ignored</h4>
      <h2 id="next">Next</h2>
    `
    const result = extractHeadings(html)
    expect(result).toEqual([
      { id: 'intro', text: 'Intro', level: 2 },
      { id: 'details', text: 'Details', level: 3 },
      { id: 'next', text: 'Next', level: 2 },
    ])
  })

  it('slugifies heading text and assigns id when missing', () => {
    const html = '<h2>Hello World!</h2>'
    const result = extractHeadings(html)
    expect(result).toEqual([{ id: 'hello-world', text: 'Hello World!', level: 2 }])
  })

  it('returns empty array for html with no headings', () => {
    expect(extractHeadings('<p>plain</p>')).toEqual([])
  })
})
```

- [ ] **Step 2: Run test and confirm fail**

```bash
pnpm run --filter web test -- -t 'extractHeadings'
```

Expected: FAIL.

- [ ] **Step 3: Implement**

Create `apps/web/src/lib/extract-headings.ts`:

```ts
export interface ExtractedHeading {
  id: string
  text: string
  level: 2 | 3
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^\w\s-]/g, '')
    .replace(/\s+/g, '-')
}

/**
 * Extracts h2 and h3 headings from a rendered markdown HTML string.
 * Pure regex parsing — runs on the server during loader execution.
 * Assigns slugified ids when the heading lacks one.
 */
export function extractHeadings(html: string): ExtractedHeading[] {
  const headings: ExtractedHeading[] = []
  const re = /<h([23])(?:\s+([^>]*))?>([\s\S]*?)<\/h\1>/gi
  let match: RegExpExecArray | null
  while ((match = re.exec(html)) !== null) {
    const level = parseInt(match[1], 10) as 2 | 3
    const attrs = match[2] || ''
    const text = match[3].replace(/<[^>]+>/g, '').trim()
    const idMatch = attrs.match(/\bid="([^"]+)"/)
    const id = idMatch ? idMatch[1] : slugify(text)
    headings.push({ id, text, level })
  }
  return headings
}
```

- [ ] **Step 4: Run test and confirm pass**

```bash
pnpm run --filter web test -- -t 'extractHeadings'
```

Expected: PASS.

- [ ] **Step 5: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/lib/extract-headings.ts apps/web/src/lib/__tests__/extract-headings.test.ts
git commit -m "feat(web): add extractHeadings utility for docs TOC"
```

---

### Task 15: Inject heading ids during render and pass to layout

`md4x`'s `renderToHtml` does not add heading ids. We post-process the HTML to add them so anchor links work, then extract headings for the TOC.

**Files:**

- Modify: `apps/web/src/lib/content.ts`
- Modify: `apps/web/src/routes/docs/$.tsx`

- [ ] **Step 1: Add an id-injection helper inside `content.ts`**

In `apps/web/src/lib/content.ts`, just above `getDoc`, add:

```ts
function injectHeadingIds(html: string): string {
  return html.replace(/<(h[23])>([\s\S]*?)<\/\1>/gi, (_, tag, inner) => {
    const text = inner.replace(/<[^>]+>/g, '').trim()
    const id = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')
    return `<${tag} id="${id}">${inner}</${tag}>`
  })
}
```

Then in `getDoc`, change:

```ts
const html = renderToHtml(body)
```

to:

```ts
const html = injectHeadingIds(renderToHtml(body))
```

- [ ] **Step 2: Update `apps/web/src/routes/docs/$.tsx` to extract and pass headings**

Replace the file contents with:

```tsx
import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getDoc, getAllDocs } from '~/lib/content'
import { extractHeadings } from '~/lib/extract-headings'
import { DocsLayout } from '~/components/docs-layout'
import { DocsNav } from '~/components/docs-nav'
import { seo } from '~/lib/seo'

const fetchDoc = createServerFn({ method: 'GET' })
  .inputValidator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    const [doc, allDocs] = await Promise.all([getDoc(slug), getAllDocs()])
    if (!doc) throw new Error(`Doc not found: ${slug}`)
    const headings = extractHeadings(doc.html).map((h) => ({
      id: h.id,
      text: h.text,
      level: h.level,
    }))
    return { doc, allDocs, headings }
  })

export const Route = createFileRoute('/docs/$')({
  loader: async ({ params }) => {
    const slug = params._splat || 'getting-started'
    return fetchDoc({ data: slug })
  },
  head: ({ loaderData }) => ({
    meta: loaderData
      ? seo({
          title: `${loaderData.doc.meta.title} — Mdow Docs`,
          description: loaderData.doc.meta.description,
        })
      : [],
  }),
  component: DocPage,
})

function DocPage() {
  const { doc, allDocs, headings } = Route.useLoaderData()

  // Content is trusted — rendered from our own .md files by md4x server-side
  return (
    <DocsLayout docs={allDocs} currentSlug={doc.meta.slug} headings={headings}>
      <div dangerouslySetInnerHTML={{ __html: doc.html }} />
      <DocsNav docs={allDocs} currentSlug={doc.meta.slug} />
    </DocsLayout>
  )
}
```

- [ ] **Step 3: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
pnpm run --filter web test
pnpm run --filter web build
```

```bash
git add apps/web/src/lib/content.ts apps/web/src/routes/docs/$.tsx
git commit -m "feat(web): inject heading ids and pass TOC data to docs layout"
```

---

### Task 16: useScrollspy hook

**Files:**

- Create: `apps/web/src/hooks/use-scrollspy.ts`
- Create: `apps/web/src/hooks/__tests__/use-scrollspy.test.ts`

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/hooks/__tests__/use-scrollspy.test.ts`:

```ts
import { renderHook } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useScrollspy } from '../use-scrollspy'

describe('useScrollspy', () => {
  beforeEach(() => {
    vi.stubGlobal(
      'IntersectionObserver',
      vi.fn(() => ({
        observe: vi.fn(),
        disconnect: vi.fn(),
        unobserve: vi.fn(),
        takeRecords: vi.fn(),
      })),
    )
  })

  it('returns null when no headings provided', () => {
    const { result } = renderHook(() => useScrollspy([]))
    expect(result.current).toBeNull()
  })

  it('observes each heading id', () => {
    document.body.innerHTML = '<h2 id="a"></h2><h2 id="b"></h2>'
    const observe = vi.fn()
    vi.stubGlobal(
      'IntersectionObserver',
      vi.fn(() => ({
        observe,
        disconnect: vi.fn(),
        unobserve: vi.fn(),
        takeRecords: vi.fn(),
      })),
    )
    renderHook(() => useScrollspy(['a', 'b']))
    expect(observe).toHaveBeenCalledTimes(2)
  })
})
```

- [ ] **Step 2: Run test and confirm fail**

```bash
pnpm run --filter web test -- -t 'useScrollspy'
```

Expected: FAIL.

- [ ] **Step 3: Implement**

Create `apps/web/src/hooks/use-scrollspy.ts`:

```ts
import { useEffect, useState } from 'react'

/**
 * Returns the id of the heading currently most-visible in the viewport.
 * Uses IntersectionObserver to track visibility of each provided id.
 */
export function useScrollspy(ids: string[]): string | null {
  const [active, setActive] = useState<string | null>(null)

  useEffect(() => {
    if (ids.length === 0) {
      setActive(null)
      return
    }

    const visible = new Map<string, number>()

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const id = entry.target.id
          if (entry.isIntersecting) {
            visible.set(id, entry.intersectionRatio)
          } else {
            visible.delete(id)
          }
        }
        if (visible.size === 0) return
        let best: string | null = null
        let bestRatio = -1
        for (const [id, ratio] of visible) {
          if (ratio > bestRatio) {
            best = id
            bestRatio = ratio
          }
        }
        setActive(best)
      },
      { rootMargin: '-80px 0px -60% 0px', threshold: [0, 0.25, 0.5, 1] },
    )

    for (const id of ids) {
      const el = document.getElementById(id)
      if (el) observer.observe(el)
    }

    return () => observer.disconnect()
  }, [ids])

  return active
}
```

- [ ] **Step 4: Run test and confirm pass**

```bash
pnpm run --filter web test -- -t 'useScrollspy'
```

Expected: PASS.

- [ ] **Step 5: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/hooks/use-scrollspy.ts apps/web/src/hooks/__tests__/use-scrollspy.test.ts
git commit -m "feat(web): add useScrollspy hook for docs TOC"
```

---

### Task 17: Wire up DocsToc with active highlighting

**Files:**

- Modify: `apps/web/src/components/docs-toc.tsx`

- [ ] **Step 1: Replace the file**

Overwrite `apps/web/src/components/docs-toc.tsx`:

```tsx
import { useMemo } from 'react'
import { useScrollspy } from '~/hooks/use-scrollspy'
import { cn } from '~/lib/utils'

export interface TocItem {
  id: string
  text: string
  level: number
}

interface DocsTocProps {
  headings: TocItem[]
}

export function DocsToc({ headings }: DocsTocProps) {
  const ids = useMemo(() => headings.map((h) => h.id), [headings])
  const active = useScrollspy(ids)

  if (headings.length === 0) return null

  return (
    <nav className="hidden w-52 shrink-0 xl:block">
      <div className="sticky top-20">
        <h4 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          On this page
        </h4>
        <ul className="space-y-1.5 text-sm">
          {headings.map((h) => {
            const isActive = h.id === active
            return (
              <li key={h.id} style={{ paddingLeft: `${(h.level - 2) * 12}px` }}>
                <a
                  href={`#${h.id}`}
                  className={cn(
                    'block border-l-2 pl-3 -ml-px transition-colors',
                    isActive
                      ? 'border-primary text-foreground font-medium'
                      : 'border-transparent text-muted-foreground hover:border-border hover:text-foreground',
                  )}
                >
                  {h.text}
                </a>
              </li>
            )
          })}
        </ul>
      </div>
    </nav>
  )
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/docs-toc.tsx
git commit -m "feat(web): activate docs TOC with scrollspy highlighting"
```

---

### Task 18: Polish docs search modal

**Files:**

- Modify: `apps/web/src/components/docs-search.tsx`
- Modify (if needed): `apps/web/src/lib/search-index.ts`

- [ ] **Step 1: Verify the search index returns description**

Read `apps/web/src/lib/search-index.ts`. Confirm the returned objects from `search()` include a `description` field. If they don't, add `description: doc.description` to the result mapping.

- [ ] **Step 2: Replace docs-search.tsx**

Overwrite `apps/web/src/components/docs-search.tsx`:

```tsx
import { useState, useEffect, useCallback, useRef } from 'react'
import { useNavigate } from '@tanstack/react-router'
import type { DocMeta } from '~/lib/content'
import { buildSearchIndex, search } from '~/lib/search-index'
import { cn } from '~/lib/utils'

interface DocsSearchProps {
  docs: DocMeta[]
}

export function DocsSearch({ docs }: DocsSearchProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<ReturnType<typeof search>>([])
  const [selected, setSelected] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const navigate = useNavigate()

  useEffect(() => {
    buildSearchIndex(docs)
  }, [docs])

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setOpen((prev) => !prev)
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  useEffect(() => {
    if (open) {
      inputRef.current?.focus()
      setQuery('')
      setResults([])
      setSelected(0)
    }
  }, [open])

  const onQueryChange = useCallback((value: string) => {
    setQuery(value)
    setResults(search(value))
    setSelected(0)
  }, [])

  const goTo = useCallback(
    (slug: string) => {
      setOpen(false)
      navigate({ to: '/docs/$', params: { _splat: slug } })
    },
    [navigate],
  )

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelected((s) => Math.min(s + 1, results.length - 1))
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelected((s) => Math.max(s - 1, 0))
      } else if (e.key === 'Enter' && results[selected]) {
        goTo(results[selected].slug)
      } else if (e.key === 'Escape') {
        setOpen(false)
      }
    },
    [results, selected, goTo],
  )

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="flex w-full items-center gap-2 rounded-md border border-border-subtle bg-surface px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-muted"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
        >
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.3-4.3" />
        </svg>
        <span>Search docs</span>
        <kbd className="ml-auto rounded border border-border bg-background px-1.5 py-0.5 text-[10px] font-mono">
          ⌘K
        </kbd>
      </button>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]">
      <div
        className="fixed inset-0 bg-background/70 backdrop-blur-md"
        onClick={() => setOpen(false)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setOpen(false)
        }}
        role="button"
        tabIndex={-1}
        aria-label="Close search"
      />
      <div className="relative z-10 w-full max-w-xl overflow-hidden rounded-xl border border-border-subtle bg-popover shadow-soft-lg">
        <div className="flex items-center gap-3 border-b border-border-subtle px-4">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-muted-foreground"
            aria-hidden
          >
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search docs..."
            className="flex-1 bg-transparent py-4 text-sm outline-none placeholder:text-muted-foreground"
          />
          <kbd className="rounded border border-border bg-background px-1.5 py-0.5 text-[10px] font-mono text-muted-foreground">
            esc
          </kbd>
        </div>
        {results.length > 0 && (
          <ul className="max-h-80 overflow-y-auto py-2">
            {results.map((r, i) => (
              <li key={r.slug}>
                <button
                  type="button"
                  onMouseEnter={() => setSelected(i)}
                  onClick={() => goTo(r.slug)}
                  className={cn(
                    'flex w-full flex-col items-start gap-0.5 px-4 py-2.5 text-left transition-colors',
                    i === selected ? 'bg-surface' : '',
                  )}
                >
                  <span className="text-sm font-medium text-foreground">{r.title}</span>
                  {r.description && (
                    <span className="line-clamp-1 text-xs text-muted-foreground">
                      {r.description}
                    </span>
                  )}
                  <span className="mt-0.5 text-[10px] uppercase tracking-wider text-muted-foreground">
                    {r.category}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
        {query && results.length === 0 && (
          <p className="px-4 py-8 text-center text-sm text-muted-foreground">No results found.</p>
        )}
        <div className="flex items-center gap-3 border-t border-border-subtle bg-surface/50 px-4 py-2 text-[10px] text-muted-foreground">
          <span className="flex items-center gap-1">
            <kbd className="rounded border border-border bg-background px-1 font-mono">↑↓</kbd>{' '}
            navigate
          </span>
          <span className="flex items-center gap-1">
            <kbd className="rounded border border-border bg-background px-1 font-mono">↵</kbd>{' '}
            select
          </span>
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
git add apps/web/src/components/docs-search.tsx apps/web/src/lib/search-index.ts
git commit -m "feat(web): polish docs search modal with snippets and kbd hints"
```

---

### Task 19: Mobile docs nav drawer

**Files:**

- Create: `apps/web/src/components/docs-mobile-nav.tsx`
- Modify: `apps/web/src/components/docs-layout.tsx`

- [ ] **Step 1: Implement the mobile drawer**

Create `apps/web/src/components/docs-mobile-nav.tsx`:

```tsx
import { useState, useEffect } from 'react'
import type { DocMeta } from '~/lib/content'
import { DocsSidebar } from './docs-sidebar'

interface DocsMobileNavProps {
  docs: DocMeta[]
  currentSlug: string
}

export function DocsMobileNav({ docs, currentSlug }: DocsMobileNavProps) {
  const [open, setOpen] = useState(false)

  useEffect(() => {
    setOpen(false)
  }, [currentSlug])

  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden'
      return () => {
        document.body.style.overflow = ''
      }
    }
  }, [open])

  return (
    <div className="lg:hidden">
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="inline-flex h-9 items-center gap-2 rounded-md border border-border bg-card px-3 text-sm font-medium transition-colors hover:bg-muted"
        aria-label="Open documentation menu"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
        >
          <line x1="3" x2="21" y1="6" y2="6" />
          <line x1="3" x2="21" y1="12" y2="12" />
          <line x1="3" x2="21" y1="18" y2="18" />
        </svg>
        Menu
      </button>
      {open && (
        <div className="fixed inset-0 z-50">
          <div
            className="absolute inset-0 bg-background/70 backdrop-blur-sm"
            onClick={() => setOpen(false)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') setOpen(false)
            }}
            role="button"
            tabIndex={-1}
            aria-label="Close menu"
          />
          <aside className="absolute left-0 top-0 h-full w-72 max-w-[80vw] overflow-y-auto border-r border-border bg-background p-6 shadow-soft-lg">
            <div className="mb-4 flex items-center justify-between">
              <span className="text-sm font-semibold">Documentation</span>
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                aria-label="Close menu"
              >
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden
                >
                  <path d="M18 6 6 18" />
                  <path d="m6 6 12 12" />
                </svg>
              </button>
            </div>
            <DocsSidebar docs={docs} currentSlug={currentSlug} />
          </aside>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Update DocsLayout**

Replace `apps/web/src/components/docs-layout.tsx`:

```tsx
import type { ReactNode } from 'react'
import type { DocMeta } from '~/lib/content'
import { DocsSidebar } from './docs-sidebar'
import { DocsToc } from './docs-toc'
import { DocsSearch } from './docs-search'
import { DocsMobileNav } from './docs-mobile-nav'
import type { TocItem } from './docs-toc'

interface DocsLayoutProps {
  docs: DocMeta[]
  currentSlug: string
  headings: TocItem[]
  children: ReactNode
}

export function DocsLayout({ docs, currentSlug, headings, children }: DocsLayoutProps) {
  return (
    <div className="mx-auto max-w-6xl px-6 py-8">
      <div className="mb-6 flex items-center gap-3 lg:hidden">
        <DocsMobileNav docs={docs} currentSlug={currentSlug} />
        <div className="flex-1">
          <DocsSearch docs={docs} />
        </div>
      </div>
      <div className="flex gap-10">
        <div className="hidden w-56 shrink-0 space-y-4 lg:block">
          <DocsSearch docs={docs} />
          <DocsSidebar docs={docs} currentSlug={currentSlug} />
        </div>
        <article className="prose prose-neutral min-w-0 max-w-none flex-1 dark:prose-invert">
          {children}
        </article>
        <DocsToc headings={headings} />
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
pnpm run --filter web build
git add apps/web/src/components/docs-mobile-nav.tsx apps/web/src/components/docs-layout.tsx
git commit -m "feat(web): add mobile docs nav drawer and reflow layout"
```

---

### Task 20: Prose typography polish

**Files:**

- Modify: `apps/web/src/styles/app.css`

- [ ] **Step 1: Add prose overrides**

Append at the bottom of `apps/web/src/styles/app.css`:

```css
@layer base {
  .prose {
    --tw-prose-body: var(--foreground);
    --tw-prose-headings: var(--foreground);
    --tw-prose-links: var(--primary);
    --tw-prose-bold: var(--foreground);
    --tw-prose-code: var(--foreground);
    --tw-prose-pre-bg: var(--surface);
    --tw-prose-pre-code: var(--foreground);
    --tw-prose-quotes: var(--muted-foreground);
    --tw-prose-quote-borders: var(--border);
    --tw-prose-bullets: var(--muted-foreground);
  }
  .prose h2 {
    margin-top: 2.5em;
    margin-bottom: 0.75em;
    scroll-margin-top: 5rem;
  }
  .prose h3 {
    margin-top: 1.75em;
    margin-bottom: 0.5em;
    scroll-margin-top: 5rem;
  }
  .prose pre {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: 1rem 1.25rem;
    font-size: 0.85em;
  }
  .prose :not(pre) > code {
    background-color: var(--surface);
    border: 1px solid var(--border-subtle);
    padding: 0.1rem 0.35rem;
    border-radius: 0.25rem;
    font-size: 0.85em;
  }
  .prose :not(pre) > code::before,
  .prose :not(pre) > code::after {
    content: '';
  }
}
```

- [ ] **Step 2: Verify and commit**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt
pnpm run --filter web build
git add apps/web/src/styles/app.css
git commit -m "feat(web): polish docs prose typography and code blocks"
```

---

### Task 21: Final verification pass

- [ ] **Step 1: Run full verification**

```bash
pnpm run --filter web typecheck
pnpm run --filter web lint
pnpm run --filter web fmt:check
pnpm run --filter web test
pnpm run --filter web build
```

All five must pass cleanly. If `fmt:check` fails, run `pnpm run --filter web fmt` and re-run.

- [ ] **Step 2: Smoke test in dev**

```bash
pnpm run --filter web dev
```

Open the URL Vite reports. Verify:

- Landing page renders all 5 sections without console errors
- Hero mockup is visible and crisp
- Feature rows alternate
- Light/dark theme toggle works site-wide
- `/docs` loads, sidebar shows category groups, active doc has the warm indicator
- Cmd+K opens the polished search modal
- TOC appears on xl+ screens and highlights as you scroll
- Resize to mobile width: hamburger appears, sidebar drawer opens/closes
- `/download` and `/changelog` still work

Stop the dev server with Ctrl-C.

- [ ] **Step 3: No commit needed** — verification only.

---

## Out of scope (do not implement)

- Pricing section
- Comparison vs other markdown viewers
- Real app screenshots (mockups only for now)
- Authentication, analytics, blog
- Versioned docs or category landing pages
- Changing content/markdown files
