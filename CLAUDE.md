# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development

- `npm run dev` — start Electron in development mode (electron-vite)
- `npm run build` — compile and package for production
- `npm run typecheck` — run both main and renderer type checks (uses tsgo)
- `npm run typecheck:node` / `npm run typecheck:web` — check main or renderer separately
- `npm run lint` — oxlint (strict rules for TS, React, async/await, promises)
- `npm run lint:fix` — oxlint with auto-fix
- `npm run fmt` — format with oxfmt
- `npm run fmt:check` — check formatting without writing
- `npm run test` — run tests with Vitest
- `npm run test:watch` — run tests in watch mode

Always use `npm run` scripts — never invoke oxlint, oxfmt, or tsgo directly.

## Code Style

- oxfmt: no semicolons, single quotes, 100-char line width, trailing commas, 2-space indent
- React 19 with JSX transform — no `import React` needed
- Tailwind CSS v4 with CSS variables for theming (light/dark)
- shadcn/ui components live in `src/renderer/src/components/ui/`
- Use `cn()` from `src/renderer/src/lib/utils.ts` for conditional class merging

## Architecture

- **Main process** (`src/main/`): Electron window, IPC handlers, file/folder services, chokidar watchers
- **Preload** (`src/preload/`): IPC bridge exposing `window.api` — types in `index.d.ts`
- **Renderer** (`src/renderer/src/`): React app with Zustand store + TanStack Query
- Markdown rendering uses md4x (WASM) + Shiki (syntax highlighting) + Mermaid (diagrams)
- Path alias: `@renderer/*` maps to `src/renderer/src/*`

## Key Patterns

- State: Zustand for UI state (`store/app-store.ts`), TanStack Query for async data
- IPC: all main↔renderer communication through typed handlers in `ipc.ts` / `preload/index.ts`
- File types: `.md`, `.markdown`, `.mdx` are treated as markdown
