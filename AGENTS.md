# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Repository Structure

This is a pnpm monorepo managed by Turborepo.

- `apps/web/` — marketing site and docs (TanStack Start, Cloudflare)
- `apps/rune/` — Mdow, on Linux and macOS: Rust on Rune (GPU-rendered), one codebase for both, which takes its colours from the Omarchy theme on Omarchy; ships as an AppImage and a Flatpak on Linux and as `Mdow Rune.app` on the Mac (not a pnpm workspace; built with Cargo)
- `packages/` — shared libraries (none yet)

The release version lives in the root `package.json`.

## Build & Development

From the repo root, turbo orchestrates the pnpm workspaces (the website):

- `pnpm run dev` — start the website in development mode
- `pnpm run build` — compile all workspaces
- `pnpm run typecheck` — type-check all workspaces (uses tsgo)
- `pnpm run lint` — oxlint across all workspaces
- `pnpm run fmt` — format with oxfmt
- `pnpm run fmt:check` — check formatting
- `pnpm run test` — run tests with Vitest

The app:

- `pnpm run dev:rune` / `test:rune` / `install:rune` — run, test, or install Mdow (the Rune app) on this machine, Linux or Mac (Rust 1.100.0, pinned in its `rust-toolchain.toml`; fonts and the logo are in `apps/rune/assets/`); `.github/workflows/linux.yml` builds and tests its AppImage and Flatpak (`apps/rune/packaging/`), `macos.yml` does the same on a Mac, and `release.yml` attaches both systems' builds to each `vX.Y.Z` release
- `MDOW_PERF=1` logs how long the app takes to start, open documents, scroll, zoom and restyle, and `MDOW_ACTIONS` scripts it (`apps/rune/src/actions.rs`)

Always use `pnpm run` scripts — never invoke oxlint, oxfmt, or tsgo directly.

## Code Style

- oxfmt: no semicolons, single quotes, 100-char line width, trailing commas, 2-space indent
- React 19 with JSX transform — no `import React` needed
- Tailwind CSS v4 with CSS variables for theming (light/dark)

## Architecture

- Website docs and changelog render with md4x
- **Rune** (`apps/rune/`): the app on both systems. `doc.rs` parses markdown (comrak) or HTML into top-level blocks, `files.rs` scans folders and `highlight.rs` colours code (syntect); `reader/` turns blocks into Rune elements, one virtual row per top-level block; `media.rs` rasterises images, Mermaid (mermaid-rs-renderer) and display math (latex-rust) with resvg off the main thread. `platform.rs` holds what differs by system: Finder or the desktop's file manager, the editor, where settings live, how shortcuts are written (⌘ on the Mac, Ctrl elsewhere) and the AppKit submenus; `omarchy.rs` reads the Omarchy theme into the palette and follows `omarchy theme set`; `webview.rs` is WebKit for HTML files and printing, on the Mac only. On the Mac, menu items queue `Cmd`s for the window in front (`windows.rs`); elsewhere `menu::fallback_key` handles the same chords. Check UI headlessly with `cargo run -- --snap out.png --scene <name> FILE` (scenes in `main.rs`), on either system.

## Key Patterns

- File types: `.md`, `.markdown`, `.mdx` are markdown; `.html` / `.htm` open in a sandboxed HTML viewer

## Testing

- `pnpm run test` — run the website's tests (Vitest, single run)
- `pnpm run --filter web test -- -t 'test name'` — run a single test by name
- Test environment: jsdom with `@testing-library/react`
- `pnpm run test:rune` (Cargo) tests the app
