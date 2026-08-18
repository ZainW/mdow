---
title: Changelog
description: What's new in Mdow
---

# Changelog

## v1.6.2

Latest release.

- Hold ⌘ (macOS) or Ctrl (Windows/Linux) to glance at a keyboard cheat sheet without dimming the document
- Upgraded remaining desktop and web dependencies
- Polished docs and copy to match the current app

## v1.6.1

- Upgraded the comark markdown engine to 0.6 with API compatibility fixes
- Updated all desktop and web dependencies to their latest compatible versions
- Removed unused dependencies for leaner builds and a deduplicated lockfile

## v1.6.0

- Added a separately distributed Mdow Native GPUI beta for Apple Silicon Macs
- Added a local, read-only AI Companion for asking questions about Markdown documents through installed ACP providers
- Added focused-document, open-folder, and `@`-tagged context with clickable source citations
- Streamed Markdown answers alongside collapsible thinking and tool activity in drawer and expanded views
- Hardened long responses, cancellation, multi-window routing, provider fallback, and custom executable setup

## v1.5.2

- Updated desktop and website dependencies, including Electron, TypeScript, Vite, and build tooling
- Moved installs to pnpm 11's Rust-based engine for faster dependency setup
- Kept Electron native dependency preparation reliable under the new install engine

## v1.5.1

- Fixed Open Recent from the app menu so recent files open reliably
- Added command palette actions for files, folders, search, layout, settings, and shortcuts
- Added folder sidebar filtering with match counts
- Refreshed docs and shortcut references for the current desktop app

## v1.5.0

- Open and render HTML documents alongside Markdown documents
- Added sandboxed HTML previews for local files and document links
- Added side-by-side document panes with pane targeting from tabs
- Expanded coverage for HTML loading, split panes, tab actions, and renderer CSP

## v1.4.1

- Added a native macOS beta build under the download page beta section
- Published native Mac beta ZIP artifacts from the tagged release workflow
- Kept the Electron macOS build as the recommended stable download

## v1.4.0

- Redesigned sidebar mode navigation with a simpler, more focused menu
- Added coverage for menu shortcuts and destroyed-window handling
- Prevented stale folder scans from updating sidebar state after navigation
- Bounded render caches for steadier memory use during long reading sessions
- Fixed Mermaid SVG caching so edited diagrams rerender with the latest source

## v1.3.0

- Improved desktop startup performance and memory behavior
- Reduced large-document rendering work with lazy markdown features
- Tightened folder/sidebar loading behavior and markdown render caching
- Updated desktop and web dependencies

## v1.2.1

- Enabled the automatic update settings flow on macOS
- Documented the in-app update download and restart flow across supported platforms

## v1.2.0

- App-wide error boundary and clearer startup/read error handling
- TTL-based render cache eviction for steadier memory use
- Refined desktop scaling, spacing, typography, and warm paper palette
- Restored the smaller Geist Mono font asset for leaner packages
- Web theme switching fixes with system preference sync
- Markdown rendering split into focused components and hooks

## v1.1.0

- macOS code signing and notarization — no more Gatekeeper warnings
- Multi-window support
- Vault UX and security improvements
- New UI primitives and Lucide icons
- Document loading skeleton and content fade-in
- Performance improvements

## v1.0.5

- Improved tab bar accessibility and keyboard navigation
- Zoom indicator with stable layout across zoom levels
- Refined sidebar modes (Recents, Outline, Folder)
- Command palette and search polish
- Settings for theme, content font, and code font

## v1.0.0

Initial release.

- Markdown rendering with md4x (WASM)
- Syntax highlighting for 30+ languages via Shiki
- Mermaid diagram support
- Light and dark themes
- File tree sidebar with folder browsing
- Tabbed reading for multiple documents
- Drag-and-drop file opening
- Command palette (`Cmd+K`) and find in document (`Cmd+F`)
- Available for macOS, Windows, and Linux
