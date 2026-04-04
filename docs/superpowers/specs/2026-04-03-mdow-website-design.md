# Mdow Website — Design Spec

## Overview

A product website for Mdow ("A quiet place to read markdown") — an Electron-based markdown viewer. The site serves as the landing page, documentation hub, and download portal for the desktop application.

## Stack

| Layer               | Choice                                         |
| ------------------- | ---------------------------------------------- |
| Framework           | TanStack Start (`@tanstack/react-start`)       |
| Routing             | TanStack Router (file-based)                   |
| Content             | `.md` files with MDC (Comark) component syntax |
| Markdown engine     | md4x + Comark (shared with desktop app)        |
| Syntax highlighting | Shiki (shared with desktop app)                |
| Styling             | Tailwind CSS v4 + shadcn/ui                    |
| Search              | Fuse.js (client-side, over titles/headings)    |
| Binary hosting      | Cloudflare R2                                  |
| Deployment          | Cloudflare Pages                               |
| Monorepo location   | `apps/web/`                                    |

### Key dependency versions (as of 2026-04-03)

- `@tanstack/react-start` — ^1.167
- `@tanstack/react-router` — ^1.168
- `md4x` — ^0.0.25
- `tailwindcss` — ^4.2
- `shadcn` CLI — ^4.1
- `shiki` — ^4.0
- `comark` — latest (React renderer for MDC syntax, from unjs/comark)

## Monorepo Setup

Adding the web workspace requires:

- **`pnpm-workspace.yaml`** — add `apps/web` to the workspace list (already covered by `apps/*` glob if present)
- **`turbo.json`** — add web-specific task outputs (TanStack Start/Vinxi outputs to `.output/` not `out/`)

## Monorepo Structure

```
mdow/
├── apps/
│   ├── desktop/          # existing Electron app
│   └── web/              # new TanStack Start site
│       ├── app/
│       │   ├── routes/
│       │   │   ├── index.tsx           # landing page
│       │   │   ├── download.tsx        # download page
│       │   │   ├── changelog.tsx       # changelog
│       │   │   ├── pricing.tsx         # (future)
│       │   │   └── docs/
│       │   │       ├── index.tsx       # docs index
│       │   │       └── $.tsx           # splat route for docs
│       │   ├── components/             # site components
│       │   └── lib/                    # utils, mdx loader
│       ├── content/
│       │   └── docs/
│       │       ├── getting-started.md
│       │       ├── installation.md
│       │       └── ...
│       └── app.config.ts
├── packages/             # (future: shared components/utils)
├── turbo.json
└── package.json
```

Turborepo orchestrates both `desktop` and `web` workspaces.

## Pages & Routes

| Route             | Purpose                                                    |
| ----------------- | ---------------------------------------------------------- |
| `/`               | Landing page — hero, features, screenshots, download CTA   |
| `/download`       | OS-detected download page with all platform options        |
| `/docs`           | Documentation index                                        |
| `/docs/[...slug]` | Individual doc pages (rendered from `.md` via splat route) |
| `/changelog`      | Version history                                            |
| `/pricing`        | (future) Plans, trial info, license purchase               |

## Docs Infrastructure

### Content pipeline

md4x + Comark replaces the traditional MDX compile-time pipeline:

- **md4x** parses markdown to HTML or AST (WASM, shared with desktop app)
- **Comark** `<ComarkRenderer>` maps MDC component names to React components at runtime
- **Comark** (`comark` on npm) is the unjs project that provides the MDC parser and React renderer. md4x has built-in MDC/Comark support via its AST output.
- **No Vite plugin needed** for content — rendering happens at runtime via Comark's React renderer
- **Frontmatter** extracted via md4x's built-in `parseMeta()`
- **Shiki** for syntax highlighting (shared with desktop app)

### MDC component syntax

```markdown
::alert{type="info"}
This is important information.
::

:badge[New]{color="blue"}
```

Components are resolved from a provided map — no JS imports in markdown files.

### Doc file frontmatter

```markdown
---
title: Getting Started
description: Install and set up Mdow
category: Basics
order: 1
---
```

### Docs layout components

- **Sidebar nav** — auto-generated from `content/docs/` file structure + frontmatter `order`/`category`
- **Table of contents** — extracted from headings, sticky on the right
- **Prev/Next navigation** — derived from sidebar order
- **Search** — client-side fuzzy search via Fuse.js over doc titles and headings; upgradeable to Algolia/Pagefind later

### Not building day-one

- Versioned docs
- i18n
- Full-text search

## Landing Page

- **Hero** — tagline, app screenshot/preview, download CTA
- **Features** — 3-4 key features with icons (fast rendering, syntax highlighting, Mermaid diagrams, light/dark theme)
- **Screenshot/demo** — app in action
- **Footer** — links to docs, changelog

Minimal, quiet, matches the app's aesthetic. No heavy animations.

## Download Page

- Server-side OS detection via user-agent, presents the right binary first
- All three platforms listed: Mac (dmg/zip), Windows (nsis), Linux (AppImage)
- Binaries served from **Cloudflare R2**
- Future: gated behind license key validation

## Changelog

- Rendered from a single `content/changelog.md` file maintained manually
- Reverse-chronological list of versions with notes
- Can be automated from GitHub Releases later if needed

## Theming & Design

- Carries over the desktop app's design language — quiet, reader-focused
- Tailwind CSS v4 with CSS-variable-based theming (light/dark)
- shadcn/ui installed fresh in `apps/web/` (`npx shadcn@latest add -c apps/web`)
- `cn()` utility in web app's own `lib/utils.ts`
- Dark mode via `class` strategy, respecting system preference
- No shared `packages/ui` yet — extract when real duplication justifies it

## Deployment

- **Cloudflare Pages** for the site (via Vinxi/Nitro `cloudflare-pages` preset)
- **Cloudflare R2** for binary distribution
- All under one Cloudflare account for cohesion (billing, DNS, storage)
- R2 binaries uploaded manually for now; CI-driven uploads are a future improvement

### Rendering strategy

- `/`, `/download`, `/changelog` — **prerendered** at build time (static content, fast)
- `/docs/*` — **prerendered** at build time (content is all in git, no dynamic data)
- `/pricing` — (future) SSR when it needs auth/dynamic pricing

All pages are static for the initial build. SSR is available via Cloudflare Workers when needed.

### SEO & meta

- Open Graph tags and meta descriptions on all pages
- Auto-generated `sitemap.xml`
- Custom 404 page
- Favicons derived from existing app icon assets

## Future Considerations

These are out of scope for the initial build but the architecture accommodates them:

- **Paid product** — license key validation, trial management, Stripe integration
- **Auth** — email-based authentication for license holders
- **Pricing page** — plans, trial info, purchase flow
- **Versioned docs** — when breaking changes warrant it
- **Full-text search** — Algolia or Pagefind upgrade path
- **Analytics** — Cloudflare Web Analytics or Plausible
- **CI-driven R2 uploads** — automate binary publishing from release pipeline
