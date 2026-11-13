# Mdow

_A quiet place to read markdown._

Mdow is a fast markdown viewer for macOS and Linux. Open files, browse folders, and read rendered markdown with syntax highlighting, Mermaid diagrams, and a local AI companion.

## Features

- **Tabbed reading** — open multiple files at once
- **Folder browsing** — sidebar with full directory tree
- **Syntax highlighting** — for the languages bat ships grammars for
- **Mermaid diagrams** — rendered inline
- **Command palette** — quick navigation with `Cmd+K`
- **Search** — find text across your documents with `Cmd+F`
- **Dark & light themes** — follows your system preference
- **File watching** — live updates when files change on disk
- **Drag & drop** — drop `.md`, `.markdown`, `.mdx`, `.html`, or `.htm` files to view them
- **AI companion** — ask about open documents through an installed ACP provider

## Install

Download the latest release from the [Releases](https://github.com/ZainW/mdow/releases) page.

| Platform | Format                                                                |
| -------- | --------------------------------------------------------------------- |
| macOS    | `Mdow-Rune-<version>.zip`                                              |
| Linux    | AppImage or Flatpak, from the `linux-latest` pre-release (a preview)   |

Mdow is one app on both (`apps/rune`): Rust on [Rune](https://github.com/anomalyco/rune), drawn
by its own GPU renderer, for Apple Silicon Macs on macOS 15 or newer and for x64 Linux. On
Omarchy it takes its colours from the current theme.

The apps it replaced are still in the tree: the Swift app (`apps/macos`), which releases keep
building so the copies of it that update themselves still can, and the GTK app
(`apps/native-linux`).

## Development

This is a pnpm monorepo. The website lives in `apps/web` and the app in `apps/rune` (a Cargo
crate), with the earlier Swift app in `apps/macos` and the earlier GTK app in
`apps/native-linux`.

```sh
# Install dependencies
pnpm install

# Website: dev server, type check, lint, format, tests
pnpm run dev
pnpm run typecheck
pnpm run lint
pnpm run fmt
pnpm run test

# The app (Linux or Mac): run, test, install
pnpm run dev:rune README.md
pnpm run test:rune
pnpm run install:rune

# The earlier Swift and GTK apps: run, test
pnpm run dev:mac
pnpm run test:mac
pnpm run dev:gtk
pnpm run test:gtk
```

## License

MIT
