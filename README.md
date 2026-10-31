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

| Platform | Format                 |
| -------- | ---------------------- |
| macOS    | `Mdow-<version>.zip`   |

On macOS, Mdow is a native app built in Swift on AppKit and TextKit, for Apple Silicon Macs on
macOS 15 or newer (`apps/macos`). It updates itself.

Mdow for Linux (`apps/native-linux`) is a Rust app on GTK 4 and libadwaita. Its AppImage and
Flatpak are published as pre-releases (`linux-latest`).

## Development

This is a pnpm monorepo. The website lives in `apps/web`, the Mac app in `apps/macos` (a SwiftPM
package), and the Linux app in `apps/native-linux` (a Cargo crate).

```sh
# Install dependencies
pnpm install

# Website: dev server, type check, lint, format, tests
pnpm run dev
pnpm run typecheck
pnpm run lint
pnpm run fmt
pnpm run test

# Mac app: run, test
pnpm run dev:mac
pnpm run test:mac

# Linux app: run, test
pnpm run dev:linux
pnpm run test:linux
```

## License

MIT
