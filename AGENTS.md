# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Repository Structure

This is a pnpm monorepo managed by Turborepo.

- `apps/desktop/` — Electron markdown viewer (the main app)
- `apps/web/` — marketing site and docs (TanStack Start, Cloudflare)
- `apps/native/` — Mdow Native beta (macOS arm64, Linux x64): React on [gpuix](https://gpuix.dev) (Zed's GPUI), run and compiled with Bun
- `packages/` — shared libraries (none yet)

## Build & Development

From the repo root, turbo orchestrates all tasks:

- `pnpm run dev` — start Electron in development mode
- `pnpm run build` — compile all workspaces
- `pnpm run typecheck` — type-check all workspaces (uses tsgo)
- `pnpm run lint` — oxlint across all workspaces
- `pnpm run fmt` — format with oxfmt
- `pnpm run fmt:check` — check formatting
- `pnpm run test` — run tests with Vitest

To target a specific workspace:

- `pnpm run --filter desktop dev`
- `pnpm run --filter desktop build:dist -- --mac`
- `pnpm run --filter native start:hot` — run Mdow Native with hot reload (needs Bun)
- `pnpm run package:native-mac` / `package:native-linux` — build the signed native bundles

Always use `pnpm run` scripts — never invoke oxlint, oxfmt, or tsgo directly.

## Code Style

- oxfmt: no semicolons, single quotes, 100-char line width, trailing commas, 2-space indent
- React 19 with JSX transform — no `import React` needed
- Tailwind CSS v4 with CSS variables for theming (light/dark)
- shadcn/ui components live in `apps/desktop/src/renderer/src/components/ui/`
- When running shadcn CLI commands, always use `-c apps/desktop` flag (e.g. `npx shadcn@latest add -c apps/desktop button`)
- Use `cn()` from `apps/desktop/src/renderer/src/lib/utils.ts` for conditional class merging

## Architecture

- **Main process** (`apps/desktop/src/main/`): Electron window, IPC handlers, file/folder services, chokidar watchers
- **Preload** (`apps/desktop/src/preload/`): IPC bridge exposing `window.api` — types in `index.d.ts`
- **Renderer** (`apps/desktop/src/renderer/src/`): React app with Zustand store + TanStack Query
- Desktop markdown rendering uses comark + `@comark/react` + Shiki + Mermaid
- Website docs and changelog render with md4x
- **Native** (`apps/native/`): gpuix renders the React tree with GPUI (no DOM, no webview). Markdown is split into blocks (`src/lib/markdown.ts`) and each block paints through gpuix's native `<markdown>`/`<code>` inside a windowed `<virtual-list>`. macOS menus, Finder open events and font registration go through the Objective-C runtime with `bun:ffi` (`src/lib/macos.ts`). Tests: `bun test` (headless GPU renderer).
- Path alias: `@renderer/*` maps to `src/renderer/src/*` (within the desktop workspace)

## Key Patterns

- State: Zustand for UI state (`store/app-store.ts`), TanStack Query for async data
- IPC: all main↔renderer communication through typed handlers in `ipc.ts` / `preload/index.ts`
- File types: `.md`, `.markdown`, `.mdx` are markdown; `.html` / `.htm` open in a sandboxed HTML viewer

## Testing

- `pnpm run test` — run all tests (Vitest, single run)
- `pnpm run --filter desktop test -- -t 'test name'` — run a single test by name
- `pnpm run --filter desktop test:watch` — run tests in watch mode
- Test environment: jsdom with `@testing-library/react`
