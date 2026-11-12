# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Structure

This is a pnpm monorepo managed by Turborepo.

- `apps/web/` — marketing site and docs (TanStack Start, Cloudflare)
- `apps/native-linux/` — Mdow for Linux: a Rust app on GTK 4 + libadwaita with the same layout and features as Mdow for Mac, which takes its look from the Omarchy theme when it runs on Omarchy; no web view or cross-platform UI layer (not a pnpm workspace; built with Cargo)
- `apps/macos/` — Mdow for Mac: a Swift app on AppKit and TextKit with its own custom look, shipped as `Mdow.app` (not a pnpm workspace; built with SwiftPM)
- `apps/rune-mac/` — Mdow for Mac rebuilt on Rune (Rust, GPU-rendered), aiming for parity with the Swift app; ships as `Mdow Rune.app` (not a pnpm workspace; built with Cargo)
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

- `pnpm run dev:linux` / `test:linux` / `install:linux` — run, test, or install Mdow for Linux (needs Rust, GTK ≥ 4.14, libadwaita ≥ 1.5); `.github/workflows/linux.yml` builds its AppImage and Flatpak (`apps/native-linux/packaging/`)
- `pnpm run perf:linux` — time Mdow for Linux starting up, opening documents, scrolling, zooming and restyling, without a display (`--against <binary>` compares two builds; `MDOW_PERF=1` logs the same timings in normal use)
- `pnpm run dev:mac` / `test:mac` / `install:mac` — run, test, or install Mdow for Mac (needs Xcode 26+; macOS 15+ to run)
- `pnpm run dev:rune-mac` / `test:rune-mac` / `install:rune-mac` — run, test, or install the Rune build of Mdow for Mac (Rust 1.99.1, pinned in its `rust-toolchain.toml`)

Always use `pnpm run` scripts — never invoke oxlint, oxfmt, or tsgo directly.

## Code Style

- oxfmt: no semicolons, single quotes, 100-char line width, trailing commas, 2-space indent
- React 19 with JSX transform — no `import React` needed
- Tailwind CSS v4 with CSS variables for theming (light/dark)

## Architecture

- Website docs and changelog render with md4x
- **Linux** (`apps/native-linux/`): `doc.rs` parses markdown (comrak) or HTML (via htmd → markdown) into top-level rows; `reader.rs` shows them in a virtualized `GtkListView`, one row per block, built into native widgets by `render.rs`. Code is highlighted with syntect, Mermaid is laid out by mermaid-rs-renderer and rasterised with resvg, math is typeset by RaTeX, and `print.rs` sets print and PDF output in Pango. `theme.rs` holds the palettes and builds the GTK stylesheet; `omarchy.rs` reads the current Omarchy theme (`~/.local/state/omarchy/current/theme`), follows `omarchy theme set` live, and calls Omarchy's helpers. Window actions, tabs and per-pane history live in `window.rs`; the OpenCode companion is in `src/companion/`. Fonts and the logo are in `assets/`. Rust style: `cargo fmt` (100 columns) and `cargo clippy`.
- **Mac** (`apps/macos/`): `MdowKit` renders markdown (Apple's swift-markdown) into an `NSAttributedString` built from TextKit 1 parts: paragraph styles, `NSTextBlock` subclasses for code and quotes, `NSTextTable` for tables, attachments for images. The app target shows it in a read-only `NSTextView` (selection, printing; find is its own floating bar) on a rounded card, with Mdow's own chrome drawn in AppKit views (`Sources/Mdow/Chrome`): a header with in-window tabs (`DocumentTab`), a flat sidebar, and colours from one `Palette` (warm paper light, neutral ink dark, clay accent) shared with the reader. Charter for text, bundled Inter and Geist Mono for the interface and code. Code is highlighted by a hand-written Swift scanner, Mermaid is laid out by Pele in JavaScriptCore and drawn as SVG, HTML files open in a `WKWebView` with JavaScript off. Math is typeset by SwiftMath, updates come through Sparkle 2 (released with every `vX.Y.Z` Mdow release), and the OpenCode companion is a trailing panel (`Sources/*/Companion`). Swift style: 2-space indent, 100 columns.

- **Rune Mac** (`apps/rune-mac/`): shares `doc.rs`, `files.rs` and `highlight.rs` with the Linux app; `reader/` turns blocks into Rune elements, one virtual row per top-level block; `media.rs` rasterises images, Mermaid (mermaid-rs-renderer) and display math (latex-rust) with resvg off the main thread. Menu items queue `Cmd`s for the window in front (`windows.rs`). Check UI headlessly with `cargo run -- --snap out.png --scene <name> FILE` (scenes in `main.rs`).

## Key Patterns

- File types: `.md`, `.markdown`, `.mdx` are markdown; `.html` / `.htm` open in a sandboxed HTML viewer

## Testing

- `pnpm run test` — run the website's tests (Vitest, single run)
- `pnpm run --filter web test -- -t 'test name'` — run a single test by name
- Test environment: jsdom with `@testing-library/react`
- `pnpm run test:linux` (Cargo) and `pnpm run test:mac` (SwiftPM) test the native apps
