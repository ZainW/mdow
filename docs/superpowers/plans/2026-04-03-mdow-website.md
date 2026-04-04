# Mdow Website Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the mdow product website — landing page, docs hub, and download portal — as a TanStack Start app deployed to Cloudflare Pages.

**Architecture:** TanStack Start (file-based routing, SSR via Cloudflare Workers) with md4x + Comark for markdown rendering, Tailwind CSS v4 with the same OKLCH theme as the desktop app, and shadcn/ui for components. Content lives as `.md` files in `content/docs/`. The site is prerendered at build time.

**Tech Stack:** `@tanstack/react-start`, `@tanstack/react-router`, `@comark/react`, `md4x`, `shiki`, `tailwindcss` v4, `shadcn`, `@cloudflare/vite-plugin`, `wrangler`, `fuse.js`

**Spec:** `docs/superpowers/specs/2026-04-03-mdow-website-design.md`

---

## File Structure

```
apps/web/
├── public/
│   ├── favicon.ico
│   ├── favicon-16x16.png
│   ├── favicon-32x32.png
│   └── apple-touch-icon.png
├─�� src/
│   ├── routes/
│   │   ├── __root.tsx
│   ��   ├── index.tsx
│   │   ├── download.tsx
│   │   ├── changelog.tsx
│   │   └── docs/
│   │       ├── index.tsx
│   │       └── $.tsx
│   ├── components/
│   │   ├── ui/                  # shadcn components (installed via CLI)
│   │   ├── site-header.tsx
│   │   ├── site-footer.tsx
│   │   ├── theme-toggle.tsx
│   │   ├── docs-layout.tsx
│   │   ├── docs-sidebar.tsx
│   │   ├── docs-toc.tsx
│   │   ├���─ docs-nav.tsx
│   │   ├── docs-search.tsx
���   │   ├── comark-components.tsx
│   │   ├── download-card.tsx
│   │   └── hero.tsx
│   ├── lib/
│   ���   ├── utils.ts
│   │   ├── content.ts
���   │   ├── search-index.ts
│   │   └── seo.ts
│   └── styles/
│       └── app.css
├── content/
│   └── docs/
│       ├── getting-started.md
│       ├── installation.md
│       └── features.md
├── vite.config.ts
├── wrangler.jsonc
├── components.json
├── tsconfig.json
└── package.json
```

---

## Task 1: Scaffold the TanStack Start workspace

**Files:**

- Create: `apps/web/package.json`
- Create: `apps/web/tsconfig.json`
- Create: `apps/web/vite.config.ts`
- Create: `apps/web/wrangler.jsonc`
- Modify: `turbo.json`

- [ ] **Step 1: Create `apps/web/package.json`**

```json
{
  "name": "web",
  "version": "1.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "deploy": "pnpm run build && wrangler deploy",
    "typecheck": "tsc --noEmit",
    "lint": "oxlint",
    "fmt": "oxfmt .",
    "fmt:check": "oxfmt --check .",
    "test": "vitest run"
  },
  "dependencies": {
    "@comark/react": "latest",
    "@tanstack/react-router": "^1.168.0",
    "@tanstack/react-start": "^1.167.0",
    "clsx": "^2.1.1",
    "fuse.js": "^7.0.0",
    "md4x": "^0.0.25",
    "react": "^19.2.4",
    "react-dom": "^19.2.4",
    "shiki": "^4.0.2",
    "tailwind-merge": "^3.5.0"
  },
  "devDependencies": {
    "@cloudflare/vite-plugin": "latest",
    "@tailwindcss/vite": "^4.2.2",
    "@types/react": "^19.2.14",
    "@types/react-dom": "^19.2.3",
    "@vitejs/plugin-react": "^5.1.0",
    "oxfmt": "^0.42.0",
    "oxlint": "^1.57.0",
    "tailwindcss": "^4.2.2",
    "typescript": "^6.0.2",
    "vitest": "^4.1.2",
    "wrangler": "latest"
  }
}
```

- [ ] **Step 2: Create `apps/web/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "paths": {
      "~/*": ["./src/*"]
    }
  },
  "include": ["src/**/*", "content/**/*"],
  "exclude": ["node_modules", ".output"]
}
```

- [ ] **Step 3: Create `apps/web/vite.config.ts`**

```ts
import { defineConfig } from 'vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import { cloudflare } from '@cloudflare/vite-plugin'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { resolve } from 'node:path'

export default defineConfig({
  plugins: [
    cloudflare({ viteEnvironment: { name: 'ssr' } }),
    tanstackStart({
      prerender: {
        routes: ['/', '/download', '/changelog', '/docs', '/docs/*'],
        crawlLinks: true,
      },
    }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '~': resolve(__dirname, './src'),
    },
  },
})
```

- [ ] **Step 4: Create `apps/web/wrangler.jsonc`**

```jsonc
{
  "$schema": "node_modules/wrangler/config-schema.json",
  "name": "mdow-web",
  "compatibility_date": "2026-04-03",
  "compatibility_flags": ["nodejs_compat"],
  "main": "@tanstack/react-start/server-entry",
  "observability": {
    "enabled": true,
  },
}
```

- [ ] **Step 5: Update `turbo.json` to support web workspace outputs**

The current `build` task has `outputs: ["out/**"]` which is specific to electron-vite. TanStack Start outputs to `.output/`. Update the build task to include both:

In `turbo.json`, change the `build` task:

```json
{
  "build": {
    "dependsOn": ["^build"],
    "inputs": [
      "src/**",
      "content/**",
      "tsconfig*.json",
      "vite.config.*",
      "electron.vite.config.*",
      "package.json"
    ],
    "outputs": ["out/**", ".output/**"]
  }
}
```

- [ ] **Step 6: Install dependencies**

Run from repo root:

```bash
pnpm install
```

- [ ] **Step 7: Verify the workspace is recognized**

```bash
pnpm ls --filter web --depth 0
```

Expected: lists the `web` workspace and its direct dependencies.

- [ ] **Step 8: Commit**

```bash
git add apps/web/package.json apps/web/tsconfig.json apps/web/vite.config.ts apps/web/wrangler.jsonc turbo.json pnpm-lock.yaml
git commit -m "feat(web): scaffold TanStack Start workspace with Cloudflare config"
```

---

## Task 2: Root layout, theme, and styles

**Files:**

- Create: `apps/web/src/styles/app.css`
- Create: `apps/web/src/lib/utils.ts`
- Create: `apps/web/src/lib/seo.ts`
- Create: `apps/web/src/routes/__root.tsx`

- [ ] **Step 1: Create `apps/web/src/styles/app.css`**

Port the theme from the desktop app (`apps/desktop/src/renderer/src/assets/styles/index.css`). Keep the OKLCH color tokens and `@theme` block. Remove desktop-specific styles (scrollbar, tree, tab, command palette). The full CSS is below:

```css
@import 'tailwindcss';

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --font-sans:
    'Inter', ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
    'Helvetica Neue', Arial, 'Noto Sans', sans-serif;
  --font-mono:
    'Geist Mono', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
    'Courier New', monospace;
  --color-sidebar-ring: var(--sidebar-ring);
  --color-sidebar-border: var(--sidebar-border);
  --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
  --color-sidebar-accent: var(--sidebar-accent);
  --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
  --color-sidebar-primary: var(--sidebar-primary);
  --color-sidebar-foreground: var(--sidebar-foreground);
  --color-sidebar: var(--sidebar);
  --color-chart-5: var(--chart-5);
  --color-chart-4: var(--chart-4);
  --color-chart-3: var(--chart-3);
  --color-chart-2: var(--chart-2);
  --color-chart-1: var(--chart-1);
  --color-ring: var(--ring);
  --color-input: var(--input);
  --color-border: var(--border);
  --color-border-subtle: var(--border-subtle);
  --color-destructive: var(--destructive);
  --color-accent-foreground: var(--accent-foreground);
  --color-accent: var(--accent);
  --color-muted-foreground: var(--muted-foreground);
  --color-muted: var(--muted);
  --color-secondary-foreground: var(--secondary-foreground);
  --color-secondary: var(--secondary);
  --color-primary-foreground: var(--primary-foreground);
  --color-primary: var(--primary);
  --color-popover-foreground: var(--popover-foreground);
  --color-popover: var(--popover);
  --color-card-foreground: var(--card-foreground);
  --color-card: var(--card);
  --radius-sm: calc(var(--radius) - 4px);
  --radius-md: calc(var(--radius) - 2px);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) + 4px);
}

/* Warm neutral light theme — stone/sand tones */
@layer base {
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

  /* Neutral dark theme — pure gray, no warmth */
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
}

@layer base {
  * {
    @apply border-border;
  }

  body {
    @apply bg-background text-foreground antialiased;
    font-feature-settings: 'cv02', 'cv03', 'cv04', 'cv11';
  }
}
```

- [ ] **Step 2: Create `apps/web/src/lib/utils.ts`**

```ts
import { clsx } from 'clsx'
import type { ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

- [ ] **Step 3: Create `apps/web/src/lib/seo.ts`**

```ts
export function seo({
  title,
  description,
  image,
}: {
  title: string
  description: string
  image?: string
}) {
  const tags = [
    { title },
    { name: 'description', content: description },
    { name: 'og:title', content: title },
    { name: 'og:description', content: description },
    { name: 'twitter:card', content: 'summary_large_image' },
    { name: 'twitter:title', content: title },
    { name: 'twitter:description', content: description },
  ]

  if (image) {
    tags.push({ name: 'og:image', content: image })
    tags.push({ name: 'twitter:image', content: image })
  }

  return tags
}
```

- [ ] **Step 4: Create `apps/web/src/routes/__root.tsx`**

The root route sets up the HTML shell, meta tags, global styles, header, footer, and dark mode initialization. The dark mode script is a static string (no user input) that reads `localStorage` before first paint to prevent flash of wrong theme:

```tsx
/// <reference types="vite/client" />
import type { ReactNode } from 'react'
import { Outlet, createRootRoute, HeadContent, Scripts, Link } from '@tanstack/react-router'
import appCss from '~/styles/app.css?url'
import { seo } from '~/lib/seo'

const THEME_SCRIPT = `(function(){var t=localStorage.getItem('theme');if(t==='dark'||(!t&&matchMedia('(prefers-color-scheme:dark)').matches))document.documentElement.classList.add('dark')})()`

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      ...seo({
        title: 'Mdow — A quiet place to read markdown',
        description:
          'A beautiful, fast markdown viewer for Mac, Windows, and Linux. Syntax highlighting, Mermaid diagrams, and a distraction-free reading experience.',
      }),
    ],
    links: [
      { rel: 'stylesheet', href: appCss },
      { rel: 'icon', href: '/favicon.ico' },
      { rel: 'icon', type: 'image/png', sizes: '32x32', href: '/favicon-32x32.png' },
      { rel: 'icon', type: 'image/png', sizes: '16x16', href: '/favicon-16x16.png' },
      { rel: 'apple-touch-icon', sizes: '180x180', href: '/apple-touch-icon.png' },
    ],
    scripts: [{ children: THEME_SCRIPT }],
  }),
  component: RootComponent,
  notFoundComponent: NotFound,
})

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  )
}

function NotFound() {
  return (
    <RootDocument>
      <div className="flex min-h-[60vh] flex-col items-center justify-center gap-4">
        <h1 className="text-4xl font-bold">404</h1>
        <p className="text-muted-foreground">Page not found.</p>
        <Link to="/" className="text-primary underline underline-offset-4">
          Go home
        </Link>
      </div>
    </RootDocument>
  )
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body className="min-h-screen font-sans">
        {children}
        <Scripts />
      </body>
    </html>
  )
}
```

Note: the dark mode script uses the `scripts` array in `head()` which TanStack Start injects into `<head>` via `<HeadContent />`. If this approach does not work (the script needs to run before body paint), fall back to a `<script>` tag with `dangerouslySetInnerHTML` directly in the `<head>` element of `RootDocument`. The content is a static string constant, not user input.

- [ ] **Step 5: Verify dev server starts**

```bash
pnpm run --filter web dev
```

Expected: dev server starts, visiting `http://localhost:5173` shows a blank page with no errors in the console.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/
git commit -m "feat(web): add root layout, theme, and base styles"
```

---

## Task 3: Install shadcn and add base components

**Files:**

- Create: `apps/web/components.json`
- Create: `apps/web/src/components/ui/button.tsx` (via CLI)
- Create: `apps/web/src/components/site-header.tsx`
- Create: `apps/web/src/components/site-footer.tsx`
- Create: `apps/web/src/components/theme-toggle.tsx`
- Modify: `apps/web/src/routes/__root.tsx`

- [ ] **Step 1: Initialize shadcn in the web workspace**

```bash
cd apps/web && npx shadcn@latest init
```

When prompted:

- Style: base-nova (to match desktop)
- CSS variables: yes
- Tailwind config: (leave empty, uses @theme in CSS)
- Components alias: `~/components`
- Utils alias: `~/lib/utils`
- UI alias: `~/components/ui`
- RSC: no

This creates `apps/web/components.json`.

- [ ] **Step 2: Install button component**

```bash
npx shadcn@latest add -c apps/web button
```

- [ ] **Step 3: Create `apps/web/src/components/theme-toggle.tsx`**

```tsx
import { useCallback, useEffect, useState } from 'react'

export function ThemeToggle() {
  const [dark, setDark] = useState(false)

  useEffect(() => {
    setDark(document.documentElement.classList.contains('dark'))
  }, [])

  const toggle = useCallback(() => {
    const next = !dark
    setDark(next)
    document.documentElement.classList.toggle('dark', next)
    localStorage.setItem('theme', next ? 'dark' : 'light')
  }, [dark])

  return (
    <button
      type="button"
      onClick={toggle}
      className="rounded-md p-2 text-muted-foreground transition-colors hover:text-foreground"
      aria-label="Toggle theme"
    >
      {dark ? (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="5" />
          <line x1="12" y1="1" x2="12" y2="3" />
          <line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" />
          <line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </svg>
      ) : (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      )}
    </button>
  )
}
```

- [ ] **Step 4: Create `apps/web/src/components/site-header.tsx`**

```tsx
import { Link } from '@tanstack/react-router'
import { ThemeToggle } from './theme-toggle'

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-50 border-b bg-background/80 backdrop-blur-sm">
      <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-6">
        <div className="flex items-center gap-8">
          <Link to="/" className="text-lg font-semibold tracking-tight">
            Mdow
          </Link>
          <nav className="hidden items-center gap-6 text-sm md:flex">
            <Link
              to="/docs"
              className="text-muted-foreground transition-colors hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Docs
            </Link>
            <Link
              to="/changelog"
              className="text-muted-foreground transition-colors hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Changelog
            </Link>
            <Link
              to="/download"
              className="text-muted-foreground transition-colors hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Download
            </Link>
          </nav>
        </div>
        <ThemeToggle />
      </div>
    </header>
  )
}
```

- [ ] **Step 5: Create `apps/web/src/components/site-footer.tsx`**

```tsx
import { Link } from '@tanstack/react-router'

export function SiteFooter() {
  return (
    <footer className="border-t py-8">
      <div className="mx-auto flex max-w-6xl flex-col items-center gap-4 px-6 text-sm text-muted-foreground md:flex-row md:justify-between">
        <p>&copy; {new Date().getFullYear()} Mdow</p>
        <nav className="flex gap-6">
          <Link to="/docs" className="transition-colors hover:text-foreground">
            Docs
          </Link>
          <Link to="/changelog" className="transition-colors hover:text-foreground">
            Changelog
          </Link>
          <Link to="/download" className="transition-colors hover:text-foreground">
            Download
          </Link>
        </nav>
      </div>
    </footer>
  )
}
```

- [ ] **Step 6: Update `__root.tsx` to include header and footer**

Add imports at the top of `apps/web/src/routes/__root.tsx`:

```tsx
import { SiteHeader } from '~/components/site-header'
import { SiteFooter } from '~/components/site-footer'
```

Replace the `<body>` in `RootDocument` with:

```tsx
<body className="min-h-screen font-sans">
  <SiteHeader />
  <main className="flex-1">{children}</main>
  <SiteFooter />
  <Scripts />
</body>
```

- [ ] **Step 7: Verify header/footer render**

```bash
pnpm run --filter web dev
```

Expected: site loads with the Mdow header and footer visible, nav links present, theme toggle works.

- [ ] **Step 8: Commit**

```bash
git add apps/web/components.json apps/web/src/components/ apps/web/src/routes/__root.tsx
git commit -m "feat(web): add site header, footer, theme toggle, and shadcn setup"
```

---

## Task 4: Content loading and markdown rendering

**Files:**

- Create: `apps/web/src/lib/content.ts`
- Create: `apps/web/src/components/comark-components.tsx`
- Create: `apps/web/content/docs/getting-started.md`
- Create: `apps/web/content/docs/installation.md`
- Create: `apps/web/content/docs/features.md`

- [ ] **Step 1: Create `apps/web/src/lib/content.ts`**

This module loads markdown files from the `content/docs/` directory, parses frontmatter, and builds a navigation tree:

```ts
import { readdir, readFile } from 'node:fs/promises'
import { join, basename } from 'node:path'

export interface DocMeta {
  slug: string
  title: string
  description: string
  category: string
  order: number
}

export interface DocEntry {
  meta: DocMeta
  raw: string
}

const CONTENT_DIR = join(process.cwd(), 'content', 'docs')

export async function getDocSlugs(): Promise<string[]> {
  const files = await readdir(CONTENT_DIR)
  return files.filter((f) => f.endsWith('.md')).map((f) => basename(f, '.md'))
}

function parseFrontmatter(raw: string): {
  frontmatter: Record<string, string>
  body: string
} {
  const match = raw.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/)
  if (!match) return { frontmatter: {}, body: raw }

  const frontmatter: Record<string, string> = {}
  for (const line of match[1].split('\n')) {
    const [key, ...rest] = line.split(':')
    if (key && rest.length) {
      frontmatter[key.trim()] = rest.join(':').trim()
    }
  }
  return { frontmatter, body: match[2] }
}

export async function getDoc(slug: string): Promise<DocEntry | null> {
  try {
    const raw = await readFile(join(CONTENT_DIR, `${slug}.md`), 'utf-8')
    const { frontmatter, body } = parseFrontmatter(raw)
    return {
      meta: {
        slug,
        title: frontmatter.title || slug,
        description: frontmatter.description || '',
        category: frontmatter.category || 'General',
        order: parseInt(frontmatter.order || '99', 10),
      },
      raw: body,
    }
  } catch {
    return null
  }
}

export async function getAllDocs(): Promise<DocMeta[]> {
  const slugs = await getDocSlugs()
  const docs: DocMeta[] = []

  for (const slug of slugs) {
    const doc = await getDoc(slug)
    if (doc) docs.push(doc.meta)
  }

  return docs.sort((a, b) => a.order - b.order)
}

export function groupByCategory(docs: DocMeta[]): { category: string; docs: DocMeta[] }[] {
  const map = new Map<string, DocMeta[]>()
  for (const doc of docs) {
    const list = map.get(doc.category) || []
    list.push(doc)
    map.set(doc.category, list)
  }
  return [...map.entries()].map(([category, docs]) => ({ category, docs }))
}
```

- [ ] **Step 2: Create `apps/web/src/components/comark-components.tsx`**

Custom MDC components for docs content:

```tsx
import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface AlertProps {
  type?: 'info' | 'warning' | 'error' | 'success'
  children: ReactNode
}

function Alert({ type = 'info', children }: AlertProps) {
  const styles = {
    info: 'border-primary/30 bg-primary/5 text-primary',
    warning: 'border-accent/30 bg-accent/5 text-accent-foreground',
    error: 'border-destructive/30 bg-destructive/5 text-destructive',
    success: 'border-green-500/30 bg-green-500/5 text-green-700 dark:text-green-400',
  }

  return <div className={cn('my-4 rounded-lg border p-4', styles[type])}>{children}</div>
}

interface CalloutProps {
  title?: string
  children: ReactNode
}

function Callout({ title, children }: CalloutProps) {
  return (
    <div className="my-4 rounded-lg border border-border bg-muted/50 p-4">
      {title && <p className="mb-2 font-semibold">{title}</p>}
      <div className="text-sm text-muted-foreground">{children}</div>
    </div>
  )
}

export const docsComponents = {
  alert: Alert,
  callout: Callout,
}
```

- [ ] **Step 3: Create sample docs content**

Create `apps/web/content/docs/getting-started.md`:

```markdown
---
title: Getting Started
description: Get up and running with Mdow in minutes
category: Basics
order: 1
---

# Getting Started

Mdow is a quiet, beautiful markdown viewer for Mac, Windows, and Linux. Open any `.md`, `.markdown`, or `.mdx` file and read it the way it was meant to be read.

## Quick start

1. Download Mdow for your platform from the [download page](/download)
2. Install and launch the app
3. Open a markdown file or drag a folder into the sidebar

::alert{type="info"}
Mdow is a viewer, not an editor. Pair it with your favorite text editor for the best workflow.
::

## Supported formats

- `.md` — standard Markdown
- `.markdown` — alternative extension
- `.mdx` �� MDX files (rendered as Markdown)
```

Create `apps/web/content/docs/installation.md`:

```markdown
---
title: Installation
description: Platform-specific installation instructions
category: Basics
order: 2
---

# Installation

## macOS

Download the `.dmg` file, open it, and drag Mdow to your Applications folder. Alternatively, download the `.zip` for a portable version.

## Windows

Download and run the installer (`.exe`). Mdow will be added to your Start menu.

## Linux

Download the `.AppImage` file, make it executable, and run it:

\`\`\`bash
chmod +x Mdow-_.AppImage
./Mdow-_.AppImage
\`\`\`
```

Create `apps/web/content/docs/features.md`:

```markdown
---
title: Features
description: What makes Mdow special
category: Guide
order: 3
---

# Features

## Syntax highlighting

Mdow uses Shiki for accurate, editor-quality syntax highlighting across 30+ languages. Both light and dark themes are supported.

## Mermaid diagrams

Mermaid diagram blocks are rendered inline — flowcharts, sequence diagrams, class diagrams, and more.

## Light and dark themes

Mdow follows your system preference automatically, or you can set a preference manually. The warm light theme uses stone/sand tones, while the dark theme is pure neutral gray.

## File tree sidebar

Open a folder and browse its contents in a collapsible tree view. Click any markdown file to view it instantly.
```

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/lib/content.ts apps/web/src/components/comark-components.tsx apps/web/content/
git commit -m "feat(web): add content loader, MDC components, and sample docs"
```

---

## Task 5: Docs routes and layout

**Files:**

- Create: `apps/web/src/routes/docs/index.tsx`
- Create: `apps/web/src/routes/docs/$.tsx`
- Create: `apps/web/src/components/docs-layout.tsx`
- Create: `apps/web/src/components/docs-sidebar.tsx`
- Create: `apps/web/src/components/docs-toc.tsx`
- Create: `apps/web/src/components/docs-nav.tsx`

- [ ] **Step 1: Create `apps/web/src/components/docs-sidebar.tsx`**

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
    <nav className="space-y-6 text-sm">
      {groups.map((group) => (
        <div key={group.category}>
          <h4 className="mb-2 font-semibold text-foreground">{group.category}</h4>
          <ul className="space-y-1">
            {group.docs.map((doc) => (
              <li key={doc.slug}>
                <Link
                  to="/docs/$"
                  params={{ _splat: doc.slug }}
                  className={cn(
                    'block rounded-md px-3 py-1.5 transition-colors',
                    doc.slug === currentSlug
                      ? 'bg-muted font-medium text-foreground'
                      : 'text-muted-foreground hover:text-foreground',
                  )}
                >
                  {doc.title}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  )
}
```

- [ ] **Step 2: Create `apps/web/src/components/docs-toc.tsx`**

```tsx
export interface TocItem {
  id: string
  text: string
  level: number
}

interface DocsTocProps {
  headings: TocItem[]
}

export function DocsToc({ headings }: DocsTocProps) {
  if (headings.length === 0) return null

  return (
    <nav className="hidden w-48 shrink-0 xl:block">
      <div className="sticky top-20">
        <h4 className="mb-3 text-sm font-semibold">On this page</h4>
        <ul className="space-y-1.5 text-sm">
          {headings.map((h) => (
            <li key={h.id} style={{ paddingLeft: `${(h.level - 2) * 12}px` }}>
              <a
                href={`#${h.id}`}
                className="block text-muted-foreground transition-colors hover:text-foreground"
              >
                {h.text}
              </a>
            </li>
          ))}
        </ul>
      </div>
    </nav>
  )
}
```

- [ ] **Step 3: Create `apps/web/src/components/docs-nav.tsx`**

```tsx
import { Link } from '@tanstack/react-router'
import type { DocMeta } from '~/lib/content'

interface DocsNavProps {
  docs: DocMeta[]
  currentSlug: string
}

export function DocsNav({ docs, currentSlug }: DocsNavProps) {
  const currentIndex = docs.findIndex((d) => d.slug === currentSlug)
  const prev = currentIndex > 0 ? docs[currentIndex - 1] : null
  const next = currentIndex < docs.length - 1 ? docs[currentIndex + 1] : null

  return (
    <div className="mt-12 flex justify-between border-t pt-6">
      {prev ? (
        <Link
          to="/docs/$"
          params={{ _splat: prev.slug }}
          className="text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          &larr; {prev.title}
        </Link>
      ) : (
        <span />
      )}
      {next ? (
        <Link
          to="/docs/$"
          params={{ _splat: next.slug }}
          className="text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          {next.title} &rarr;
        </Link>
      ) : (
        <span />
      )}
    </div>
  )
}
```

- [ ] **Step 4: Create `apps/web/src/components/docs-layout.tsx`**

```tsx
import type { ReactNode } from 'react'
import type { DocMeta } from '~/lib/content'
import { DocsSidebar } from './docs-sidebar'
import { DocsToc } from './docs-toc'
import type { TocItem } from './docs-toc'

interface DocsLayoutProps {
  docs: DocMeta[]
  currentSlug: string
  headings: TocItem[]
  children: ReactNode
}

export function DocsLayout({ docs, currentSlug, headings, children }: DocsLayoutProps) {
  return (
    <div className="mx-auto flex max-w-6xl gap-8 px-6 py-8">
      <div className="w-56 shrink-0">
        <DocsSidebar docs={docs} currentSlug={currentSlug} />
      </div>
      <article className="min-w-0 flex-1 prose prose-neutral dark:prose-invert max-w-none">
        {children}
      </article>
      <DocsToc headings={headings} />
    </div>
  )
}
```

- [ ] **Step 5: Create `apps/web/src/routes/docs/$.tsx`**

The splat route loads and renders individual doc pages:

```tsx
import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { Comark } from '@comark/react'
import { getDoc, getAllDocs } from '~/lib/content'
import { DocsLayout } from '~/components/docs-layout'
import { DocsNav } from '~/components/docs-nav'
import { docsComponents } from '~/components/comark-components'
import { seo } from '~/lib/seo'

const fetchDoc = createServerFn({ method: 'GET' })
  .inputValidator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    const [doc, allDocs] = await Promise.all([getDoc(slug), getAllDocs()])
    if (!doc) throw new Error(`Doc not found: ${slug}`)
    return { doc, allDocs }
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
  const { doc, allDocs } = Route.useLoaderData()

  return (
    <DocsLayout docs={allDocs} currentSlug={doc.meta.slug} headings={[]}>
      <Comark components={docsComponents}>{doc.raw}</Comark>
      <DocsNav docs={allDocs} currentSlug={doc.meta.slug} />
    </DocsLayout>
  )
}
```

Note: `headings` is `[]` for now. TOC extraction from Comark's runtime rendering can be added as a refinement — parse the markdown AST on the server to extract h2/h3 headings and pass them down.

- [ ] **Step 6: Create `apps/web/src/routes/docs/index.tsx`**

Redirects `/docs` to the first doc:

```tsx
import { createFileRoute, redirect } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getAllDocs } from '~/lib/content'

const fetchFirstSlug = createServerFn({ method: 'GET' }).handler(async () => {
  const docs = await getAllDocs()
  return docs[0]?.slug || 'getting-started'
})

export const Route = createFileRoute('/docs/')({
  beforeLoad: async () => {
    const slug = await fetchFirstSlug()
    throw redirect({ to: '/docs/$', params: { _splat: slug } })
  },
})
```

- [ ] **Step 7: Verify docs render**

```bash
pnpm run --filter web dev
```

Navigate to `http://localhost:5173/docs/getting-started`. Expected: the getting-started doc renders with the sidebar showing all three docs, and prev/next nav at the bottom.

- [ ] **Step 8: Commit**

```bash
git add apps/web/src/routes/docs/ apps/web/src/components/docs-*.tsx
git commit -m "feat(web): add docs routes, sidebar, TOC, and prev/next navigation"
```

---

## Task 6: Landing page

**Files:**

- Create: `apps/web/src/routes/index.tsx`
- Create: `apps/web/src/components/hero.tsx`

- [ ] **Step 1: Create `apps/web/src/components/hero.tsx`**

```tsx
import { Link } from '@tanstack/react-router'

export function Hero() {
  return (
    <section className="mx-auto max-w-4xl px-6 py-24 text-center md:py-32">
      <h1 className="text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl">
        A quiet place to read markdown
      </h1>
      <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
        Beautiful, fast, and distraction-free. Mdow renders your markdown with syntax highlighting,
        Mermaid diagrams, and a reading experience that gets out of the way.
      </p>
      <div className="mt-10 flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
        <Link
          to="/download"
          className="inline-flex h-11 items-center rounded-lg bg-primary px-8 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
        >
          Download for free
        </Link>
        <Link
          to="/docs"
          className="inline-flex h-11 items-center rounded-lg border px-8 text-sm font-medium transition-colors hover:bg-muted"
        >
          Read the docs
        </Link>
      </div>
    </section>
  )
}
```

- [ ] **Step 2: Create `apps/web/src/routes/index.tsx`**

```tsx
import { createFileRoute } from '@tanstack/react-router'
import { Hero } from '~/components/hero'

export const Route = createFileRoute('/')({
  component: HomePage,
})

const features = [
  {
    title: 'Syntax highlighting',
    description: 'Editor-quality highlighting for 30+ languages powered by Shiki.',
  },
  {
    title: 'Mermaid diagrams',
    description: 'Flowcharts, sequence diagrams, and more rendered inline.',
  },
  {
    title: 'Light & dark themes',
    description: 'Warm stone tones in light mode, pure neutrals in dark. Follows your system.',
  },
  {
    title: 'File tree sidebar',
    description: 'Open a folder and browse your markdown files in a collapsible tree view.',
  },
]

function HomePage() {
  return (
    <>
      <Hero />
      <section className="mx-auto max-w-5xl px-6 py-16">
        <div className="grid gap-8 sm:grid-cols-2">
          {features.map((f) => (
            <div key={f.title} className="rounded-lg border bg-card p-6">
              <h3 className="mb-2 font-semibold">{f.title}</h3>
              <p className="text-sm text-muted-foreground">{f.description}</p>
            </div>
          ))}
        </div>
      </section>
    </>
  )
}
```

- [ ] **Step 3: Verify landing page renders**

```bash
pnpm run --filter web dev
```

Expected: `http://localhost:5173/` shows hero with tagline, two CTAs, and 4 feature cards.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/routes/index.tsx apps/web/src/components/hero.tsx
git commit -m "feat(web): add landing page with hero and features"
```

---

## Task 7: Download page

**Files:**

- Create: `apps/web/src/routes/download.tsx`
- Create: `apps/web/src/components/download-card.tsx`

- [ ] **Step 1: Create `apps/web/src/components/download-card.tsx`**

```tsx
import { cn } from '~/lib/utils'

interface DownloadCardProps {
  platform: string
  icon: string
  formats: { label: string; url: string }[]
  recommended?: boolean
}

export function DownloadCard({ platform, icon, formats, recommended }: DownloadCardProps) {
  return (
    <div
      className={cn(
        'rounded-lg border p-6 text-center',
        recommended && 'border-primary bg-primary/5 ring-1 ring-primary/20',
      )}
    >
      <div className="mb-3 text-3xl">{icon}</div>
      <h3 className="mb-1 text-lg font-semibold">{platform}</h3>
      {recommended && (
        <p className="mb-3 text-xs font-medium text-primary">Recommended for your OS</p>
      )}
      <div className="flex flex-col gap-2">
        {formats.map((f) => (
          <a
            key={f.label}
            href={f.url}
            className={cn(
              'inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium transition-colors',
              recommended
                ? 'bg-primary text-primary-foreground hover:bg-primary/90'
                : 'border hover:bg-muted',
            )}
          >
            {f.label}
          </a>
        ))}
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Create `apps/web/src/routes/download.tsx`**

```tsx
import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { DownloadCard } from '~/components/download-card'
import { seo } from '~/lib/seo'

const detectOS = createServerFn({ method: 'GET' }).handler(({ request }) => {
  const ua = request?.headers.get('user-agent') || ''
  if (ua.includes('Mac')) return 'mac'
  if (ua.includes('Windows')) return 'windows'
  if (ua.includes('Linux')) return 'linux'
  return 'mac'
})

export const Route = createFileRoute('/download')({
  loader: () => detectOS(),
  head: () => ({
    meta: seo({
      title: 'Download Mdow',
      description: 'Download Mdow for Mac, Windows, or Linux.',
    }),
  }),
  component: DownloadPage,
})

// Replace with actual Cloudflare R2 URLs once binaries are uploaded
const DOWNLOAD_BASE = '#'

const platforms = [
  {
    id: 'mac',
    platform: 'macOS',
    icon: '\u{1F4BB}',
    formats: [
      { label: 'Download .dmg', url: `${DOWNLOAD_BASE}/Mdow.dmg` },
      { label: 'Download .zip', url: `${DOWNLOAD_BASE}/Mdow-mac.zip` },
    ],
  },
  {
    id: 'windows',
    platform: 'Windows',
    icon: '\u{1FAA9}',
    formats: [{ label: 'Download Installer', url: `${DOWNLOAD_BASE}/Mdow-Setup.exe` }],
  },
  {
    id: 'linux',
    platform: 'Linux',
    icon: '\u{1F427}',
    formats: [{ label: 'Download .AppImage', url: `${DOWNLOAD_BASE}/Mdow.AppImage` }],
  },
]

function DownloadPage() {
  const detectedOS = Route.useLoaderData()

  const sorted = [...platforms].sort((a, b) => {
    if (a.id === detectedOS) return -1
    if (b.id === detectedOS) return 1
    return 0
  })

  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      <div className="mb-12 text-center">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Download Mdow</h1>
        <p className="mt-3 text-muted-foreground">Available for Mac, Windows, and Linux.</p>
      </div>
      <div className="grid gap-6 sm:grid-cols-3">
        {sorted.map((p) => (
          <DownloadCard
            key={p.id}
            platform={p.platform}
            icon={p.icon}
            formats={p.formats}
            recommended={p.id === detectedOS}
          />
        ))}
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Verify download page**

```bash
pnpm run --filter web dev
```

Expected: `http://localhost:5173/download` shows three platform cards, with the detected OS card first and highlighted.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/routes/download.tsx apps/web/src/components/download-card.tsx
git commit -m "feat(web): add download page with OS detection"
```

---

## Task 8: Changelog page

**Files:**

- Create: `apps/web/content/changelog.md`
- Create: `apps/web/src/routes/changelog.tsx`

- [ ] **Step 1: Create `apps/web/content/changelog.md`**

```markdown
---
title: Changelog
description: What's new in Mdow
---

# Changelog

## v1.0.0

Initial release.

- Markdown rendering with md4x (WASM)
- Syntax highlighting for 30+ languages via Shiki
- Mermaid diagram support
- Light and dark themes
- File tree sidebar
- Drag-and-drop file opening
- Available for macOS, Windows, and Linux
```

- [ ] **Step 2: Create `apps/web/src/routes/changelog.tsx`**

```tsx
import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { Comark } from '@comark/react'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { seo } from '~/lib/seo'

const fetchChangelog = createServerFn({ method: 'GET' }).handler(async () => {
  const raw = await readFile(join(process.cwd(), 'content', 'changelog.md'), 'utf-8')
  const match = raw.match(/^---\n[\s\S]*?\n---\n([\s\S]*)$/)
  return match ? match[1] : raw
})

export const Route = createFileRoute('/changelog')({
  loader: () => fetchChangelog(),
  head: () => ({
    meta: seo({
      title: 'Changelog ��� Mdow',
      description: "What's new in Mdow.",
    }),
  }),
  component: ChangelogPage,
})

function ChangelogPage() {
  const content = Route.useLoaderData()

  return (
    <div className="mx-auto max-w-3xl px-6 py-16">
      <article className="prose prose-neutral dark:prose-invert max-w-none">
        <Comark>{content}</Comark>
      </article>
    </div>
  )
}
```

- [ ] **Step 3: Verify changelog page**

```bash
pnpm run --filter web dev
```

Expected: `http://localhost:5173/changelog` renders the changelog markdown.

- [ ] **Step 4: Commit**

```bash
git add apps/web/content/changelog.md apps/web/src/routes/changelog.tsx
git commit -m "feat(web): add changelog page"
```

---

## Task 9: Client-side search

**Files:**

- Create: `apps/web/src/lib/search-index.ts`
- Create: `apps/web/src/components/docs-search.tsx`
- Modify: `apps/web/src/components/docs-layout.tsx`

- [ ] **Step 1: Create `apps/web/src/lib/search-index.ts`**

```ts
import Fuse from 'fuse.js'
import type { DocMeta } from './content'

export interface SearchEntry {
  slug: string
  title: string
  description: string
  category: string
}

let fuse: Fuse<SearchEntry> | null = null

export function buildSearchIndex(docs: DocMeta[]) {
  const entries: SearchEntry[] = docs.map((d) => ({
    slug: d.slug,
    title: d.title,
    description: d.description,
    category: d.category,
  }))

  fuse = new Fuse(entries, {
    keys: [
      { name: 'title', weight: 2 },
      { name: 'description', weight: 1 },
      { name: 'category', weight: 0.5 },
    ],
    threshold: 0.4,
  })

  return fuse
}

export function search(query: string): SearchEntry[] {
  if (!fuse || !query.trim()) return []
  return fuse.search(query).map((r) => r.item)
}
```

- [ ] **Step 2: Create `apps/web/src/components/docs-search.tsx`**

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
        className="flex w-full items-center gap-2 rounded-md border bg-muted/50 px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-muted"
      >
        <span>Search docs...</span>
        <kbd className="ml-auto rounded border bg-background px-1.5 py-0.5 text-xs">&#8984;K</kbd>
      </button>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]">
      <div
        className="fixed inset-0 bg-background/60 backdrop-blur-sm"
        onClick={() => setOpen(false)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setOpen(false)
        }}
        role="button"
        tabIndex={-1}
        aria-label="Close search"
      />
      <div className="relative z-10 w-full max-w-lg rounded-lg border bg-popover shadow-lg">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Search docs..."
          className="w-full rounded-t-lg border-b bg-transparent px-4 py-3 text-sm outline-none"
        />
        {results.length > 0 && (
          <ul className="max-h-64 overflow-y-auto py-2">
            {results.map((r, i) => (
              <li key={r.slug}>
                <button
                  type="button"
                  onClick={() => goTo(r.slug)}
                  className={cn(
                    'w-full px-4 py-2 text-left text-sm transition-colors',
                    i === selected ? 'bg-muted text-foreground' : 'text-muted-foreground',
                  )}
                >
                  <span className="font-medium">{r.title}</span>
                  <span className="ml-2 text-xs text-muted-foreground">{r.category}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
        {query && results.length === 0 && (
          <p className="px-4 py-6 text-center text-sm text-muted-foreground">No results found.</p>
        )}
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Update `apps/web/src/components/docs-layout.tsx` to include search**

Replace the full file with:

```tsx
import type { ReactNode } from 'react'
import type { DocMeta } from '~/lib/content'
import { DocsSidebar } from './docs-sidebar'
import { DocsToc } from './docs-toc'
import { DocsSearch } from './docs-search'
import type { TocItem } from './docs-toc'

interface DocsLayoutProps {
  docs: DocMeta[]
  currentSlug: string
  headings: TocItem[]
  children: ReactNode
}

export function DocsLayout({ docs, currentSlug, headings, children }: DocsLayoutProps) {
  return (
    <div className="mx-auto flex max-w-6xl gap-8 px-6 py-8">
      <div className="w-56 shrink-0 space-y-4">
        <DocsSearch docs={docs} />
        <DocsSidebar docs={docs} currentSlug={currentSlug} />
      </div>
      <article className="min-w-0 flex-1 prose prose-neutral dark:prose-invert max-w-none">
        {children}
      </article>
      <DocsToc headings={headings} />
    </div>
  )
}
```

- [ ] **Step 4: Verify search works**

```bash
pnpm run --filter web dev
```

Expected: on any docs page, the search trigger shows at the top of the sidebar. Clicking it or pressing Cmd+K opens the search dialog. Typing "install" finds the installation doc.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/lib/search-index.ts apps/web/src/components/docs-search.tsx apps/web/src/components/docs-layout.tsx
git commit -m "feat(web): add Fuse.js client-side docs search"
```

---

## Task 10: Favicons and public assets

**Files:**

- Create: `apps/web/public/favicon.ico`
- Create: `apps/web/public/favicon-16x16.png`
- Create: `apps/web/public/favicon-32x32.png`
- Create: `apps/web/public/apple-touch-icon.png`

- [ ] **Step 1: Copy and convert icons from desktop resources**

The desktop app has icons at `apps/desktop/resources/`. Copy the `.ico` directly and generate web sizes from `icon.png`:

```bash
cp apps/desktop/resources/icon.ico apps/web/public/favicon.ico
sips -z 16 16 apps/desktop/resources/icon.png --out apps/web/public/favicon-16x16.png
sips -z 32 32 apps/desktop/resources/icon.png --out apps/web/public/favicon-32x32.png
sips -z 180 180 apps/desktop/resources/icon.png --out apps/web/public/apple-touch-icon.png
```

If `sips` is not available (non-macOS), copy `icon.png` for all sizes as a fallback:

```bash
cp apps/desktop/resources/icon.ico apps/web/public/favicon.ico
cp apps/desktop/resources/icon.png apps/web/public/apple-touch-icon.png
cp apps/desktop/resources/icon.png apps/web/public/favicon-32x32.png
cp apps/desktop/resources/icon.png apps/web/public/favicon-16x16.png
```

- [ ] **Step 2: Verify favicons load in browser**

```bash
pnpm run --filter web dev
```

Expected: favicon appears in the browser tab.

- [ ] **Step 3: Commit**

```bash
git add apps/web/public/
git commit -m "feat(web): add favicons from desktop app icon"
```

---

## Task 11: Build verification and Cloudflare Pages test

**Files:** (no new files)

- [ ] **Step 1: Run typecheck**

```bash
pnpm run --filter web typecheck
```

Expected: no type errors.

- [ ] **Step 2: Run lint**

```bash
pnpm run --filter web lint
```

Expected: no lint errors (or only warnings).

- [ ] **Step 3: Run build**

```bash
pnpm run --filter web build
```

Expected: builds successfully, output in `apps/web/.output/`.

- [ ] **Step 4: Run preview**

```bash
cd apps/web && pnpm run preview
```

Expected: preview server starts, site works at the preview URL. Test: landing page, docs pages, download page, changelog, 404 page, dark mode toggle, search.

- [ ] **Step 5: Run full monorepo checks**

```bash
pnpm run build && pnpm run typecheck && pnpm run lint
```

Expected: both desktop and web workspaces pass all checks.

- [ ] **Step 6: Commit any fixes from verification**

If there were issues to fix:

```bash
git add -A
git commit -m "fix(web): address build/lint/typecheck issues"
```

Skip if everything passed clean.
