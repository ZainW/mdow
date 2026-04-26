# Mdow

_A quiet place to read markdown._

Mdow is a fast, native markdown viewer for macOS, Windows, and Linux. Open files, browse folders, and read beautifully rendered markdown — with syntax highlighting, Mermaid diagrams, and a clean, distraction-free interface.

## Features

- **Tabbed reading** — open multiple files at once
- **Folder browsing** — sidebar with full directory tree
- **Syntax highlighting** — powered by Shiki
- **Mermaid diagrams** — rendered inline
- **Command palette** — quick navigation with `Cmd+K`
- **Search** — find text across your documents with `Cmd+F`
- **Dark & light themes** — follows your system preference
- **File watching** — live updates when files change on disk
- **Drag & drop** — drop `.md`, `.markdown`, or `.mdx` files to view them

## Install

Download the latest release from the [Releases](https://github.com/zain/mdow/releases) page.

| Platform | Format                  |
| -------- | ----------------------- |
| macOS    | `.dmg`, `.zip`          |
| Windows  | `.exe` (NSIS installer) |
| Linux    | `.AppImage`             |

## Development

```sh
# Install dependencies
npm install

# Start in development mode
npm run dev

# Type check
npm run typecheck

# Lint
npm run lint

# Format
npm run fmt

# Run tests
npm run test

# Build for distribution
npm run build:dist
```

## Stack

- Electron + electron-vite
- React 19 + Zustand + TanStack Query
- Tailwind CSS v4
- md4x (WASM) for markdown parsing
- Shiki for syntax highlighting
- Mermaid for diagrams

## License

MIT
