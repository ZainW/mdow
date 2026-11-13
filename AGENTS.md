# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Repository Structure

This is a pnpm monorepo managed by Turborepo.

- `apps/web/` — marketing site and docs (TanStack Start, Cloudflare)
- `apps/rune/` — Mdow, the app the site offers, on Linux and macOS: Rust on Rune (GPU-rendered), one codebase for both, which takes its colours from the Omarchy theme on Omarchy; ships as an AppImage and a Flatpak on Linux and as `Mdow Rune.app` on the Mac (not a pnpm workspace; built with Cargo)
- `apps/native-linux/` — the earlier Mdow for Linux, on GTK 4 + libadwaita: no longer packaged, still checked in CI, and the Rune app takes its fonts and icon from its `assets/` (not a pnpm workspace; built with Cargo)
- `apps/macos/` — the earlier Mdow for Mac: a Swift app on AppKit and TextKit, shipped as `Mdow.app`. The site no longer offers it, but releases still build it and its Sparkle feed for the copies that update from it (not a pnpm workspace; built with SwiftPM)
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

The native apps:

- `pnpm run dev:rune` / `test:rune` / `install:rune` — run, test, or install Mdow (the Rune app) on this machine, Linux or Mac (Rust 1.100.0, pinned in its `rust-toolchain.toml`); `.github/workflows/linux.yml` builds its AppImage and Flatpak (`apps/rune/packaging/`), and `macos.yml` builds and tests it on a Mac
- `pnpm run dev:gtk` / `test:gtk` / `install:gtk` — run, test, or install the GTK app (needs Rust, GTK ≥ 4.14, libadwaita ≥ 1.5)
- `pnpm run perf:gtk` — time the GTK app starting up, opening documents, scrolling, zooming and restyling, without a display (`--against <binary>` compares two builds). For the Rune app, `MDOW_PERF=1` logs the same timings and `MDOW_ACTIONS` scripts it (`apps/rune/src/actions.rs`)
- `pnpm run dev:mac` / `test:mac` / `install:mac` — run, test, or install the Swift app (needs Xcode 26+; macOS 15+ to run)

Always use `pnpm run` scripts — never invoke oxlint, oxfmt, or tsgo directly.

## Code Style

- oxfmt: no semicolons, single quotes, 100-char line width, trailing commas, 2-space indent
- React 19 with JSX transform — no `import React` needed
- Tailwind CSS v4 with CSS variables for theming (light/dark)

## Architecture

- Website docs and changelog render with md4x
- **GTK** (`apps/native-linux/`): `doc.rs` parses markdown (comrak) or HTML (via htmd → markdown) into top-level rows; `reader.rs` shows them in a virtualized `GtkListView`, one row per block, built into native widgets by `render.rs`. Code is highlighted with syntect, Mermaid is laid out by mermaid-rs-renderer and rasterised with resvg. The design tokens in `theme.rs` are OKLCH values turned into a GTK stylesheet; window actions live in `window.rs`. Fonts and the logo are in `assets/`. Rust style: `cargo fmt` (100 columns) and `cargo clippy`.
- **Mac** (`apps/macos/`): `MdowKit` renders markdown (Apple's swift-markdown) into an `NSAttributedString` built from TextKit 1 parts: paragraph styles, `NSTextBlock` subclasses for code and quotes, `NSTextTable` for tables, attachments for images. The app target shows it in a read-only `NSTextView` (selection, printing; find is its own floating bar) on a rounded card, with Mdow's own chrome drawn in AppKit views (`Sources/Mdow/Chrome`): a header with in-window tabs (`DocumentTab`), a flat sidebar, and colours from one `Palette` (warm paper light, neutral ink dark, clay accent) shared with the reader. Charter for text, bundled Inter and Geist Mono for the interface and code. Code is highlighted by a hand-written Swift scanner, Mermaid is laid out by Pele in JavaScriptCore and drawn as SVG, HTML files open in a `WKWebView` with JavaScript off. Math is typeset by SwiftMath, updates come through Sparkle 2 (released with every `vX.Y.Z` Mdow release), and the OpenCode companion is a trailing panel (`Sources/*/Companion`). Swift style: 2-space indent, 100 columns.

## Key Patterns

- File types: `.md`, `.markdown`, `.mdx` are markdown; `.html` / `.htm` open in a sandboxed HTML viewer

## Testing

- `pnpm run test` — run the website's tests (Vitest, single run)
- `pnpm run --filter web test -- -t 'test name'` — run a single test by name
- Test environment: jsdom with `@testing-library/react`
- `pnpm run test:rune` and `test:gtk` (Cargo) and `pnpm run test:mac` (SwiftPM) test the native apps
