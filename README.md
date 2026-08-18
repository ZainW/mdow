# Mdow

_A quiet place to read markdown._

Mdow is a fast markdown viewer for macOS, Windows, and Linux. Open files, browse folders, and read rendered markdown with syntax highlighting, Mermaid diagrams, and a local AI companion.

## Features

- **Tabbed reading** — open multiple files at once
- **Folder browsing** — sidebar with full directory tree
- **Syntax highlighting** — powered by Shiki
- **Mermaid diagrams** — rendered inline
- **Command palette** — quick navigation with `Cmd+K`
- **Search** — find text across your documents with `Cmd+F`
- **Dark & light themes** — follows your system preference
- **File watching** — live updates when files change on disk
- **Drag & drop** — drop `.md`, `.markdown`, `.mdx`, `.html`, or `.htm` files to view them
- **AI companion** — ask about open documents through an installed ACP provider

## Install

Download the latest release from the [Releases](https://github.com/ZainW/mdow/releases) page.

| Platform        | Format                    |
| --------------- | ------------------------- |
| macOS           | `.dmg`, `.zip`            |
| macOS GPUI beta | `MdowNative-mac-beta.zip` |
| Windows         | `.exe` (NSIS installer)   |
| Linux           | `.AppImage`               |

Mdow Native is a GPUI beta for Apple Silicon Macs running macOS 14 or newer. It installs as a
separate app and runs alongside regular Mdow. The Electron app remains the recommended stable
build.

## Development

This is a pnpm monorepo. The Electron app lives in `apps/desktop`, the website in `apps/web`, and the GPUI beta in `apps/gpui`.

```sh
# Install dependencies
pnpm install

# Start in development mode
pnpm run dev

# Type check
pnpm run typecheck

# Lint
pnpm run lint

# Format
pnpm run fmt

# Run tests
pnpm run test

# Build the desktop app for distribution
pnpm run --filter desktop build:dist
```

## Stack

- Electron + electron-vite
- React 19 + Zustand + TanStack Query
- Tailwind CSS v4
- comark for markdown rendering
- Shiki for syntax highlighting
- Mermaid for diagrams

## License

MIT
