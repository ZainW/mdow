# Native macOS Markdown Reader — Design Spec

Build a separate native macOS version of Mdow that can open and read Markdown files without Electron, React, browser runtimes, or `WKWebView`.

## Scope

The first native build is a reader-focused macOS app. It opens `.md`, `.markdown`, and `.mdx` files, reads UTF-8 content from disk, displays the document in a native SwiftUI window, and includes the main reading conveniences from Mdow where they can be implemented without web technology. It is separate from the existing Electron app and does not change the existing `apps/desktop` workflow.

## Architecture

Create a SwiftPM project under `apps/native-mac/` with two products:

- `MdowNativeCore`: a testable Swift library for supported Markdown file detection, file loading, and lightweight Markdown block parsing.
- `MdowNative`: a SwiftUI executable app that uses native AppKit file dialogs, menu commands, drag/drop, and SwiftUI views.

The app uses `WindowGroup` as the primary scene and an app-wide document store owned by the root view. The SwiftUI executable is staged into a local `.app` bundle by `script/build_and_run.sh` so it launches as a normal foreground macOS app.

## Native UI

The app starts in a single document window:

- Empty state with an Open button.
- Native menu item and keyboard shortcut for File > Open (`Command-O`).
- Native menu item and keyboard shortcut for File > Open Folder (`Command-Shift-O`).
- Drag/drop support for supported Markdown files.
- Sidebar with open documents, recents, and scanned folder Markdown files.
- Sidebar outline generated from document headings, with native scroll-to-heading behavior.
- Multiple open documents with native tab-like switching in the sidebar.
- Command-K quick-open palette across open documents, recents, and scanned folder files.
- Search field with match count for the active document and `Command-F` focus.
- Active-file watching and automatic reload when the file changes on disk.
- Scrollable native reading view after a file opens.
- Header showing the current file name and full path.
- Error state for unsupported extensions, missing files, permission failures, or invalid UTF-8.

The visual treatment should be quiet and system-native: semantic colors, automatic Light/Dark mode, no custom web CSS, no custom titlebar, no card-heavy marketing layout.

## Markdown Rendering

The MVP renders common Markdown into native SwiftUI content:

- Headings (`#`, `##`, `###`)
- Paragraphs
- Bulleted and numbered list lines
- Task list items
- Blockquotes
- Fenced code blocks
- Code block copy button and lightweight native keyword coloring for common languages
- Simple pipe tables
- Local image blocks (`![alt](path)`) resolved relative to the Markdown file
- Horizontal rules
- Inline Markdown emphasis, strong text, code, and links through native `AttributedString(markdown:)`

Unsupported Markdown remains readable as plain text. The native parser intentionally avoids a full HTML pipeline because the goal is no web tech. Mermaid diagrams, Shiki highlighting, HTML injection, and md4x WASM are out of scope unless native-only replacements are added later.

## File Handling

Supported extensions are `.md`, `.markdown`, and `.mdx`, matched case-insensitively. Files are read as UTF-8. Recents persist through `UserDefaults`. Folder browsing scans recursively for supported Markdown files and skips hidden files.

## Build And Run

Add a project-local `script/build_and_run.sh` that:

1. Kills any running `MdowNative` process.
2. Runs `swift build` from `apps/native-mac`.
3. Stages `dist/MdowNative.app` with a minimal `Info.plist`.
4. Launches the app bundle with `/usr/bin/open -n`.
5. Supports `--debug`, `--logs`, `--telemetry`, and `--verify`.

Add `.codex/environments/environment.toml` so the Codex desktop app Run action points at that script.

## Verification

The local Command Line Tools Swift install does not expose `XCTest` or Swift `Testing`, so the first build uses a framework-free Swift executable check target for core behavior:

- Supported extension detection.
- Unsupported extension rejection.
- UTF-8 file loading.
- Markdown block parser behavior for headings, paragraphs, lists, blockquotes, code fences, and horizontal rules.
- Task list parsing.
- Simple pipe table parsing.
- Recursive folder scan behavior.

Use `swift run MdowNativeCoreChecks` and `swift build` in `apps/native-mac` as the primary verification. Use `./script/build_and_run.sh --verify` to stage the app bundle, launch it with a generated Markdown file path, and prove the app runs as a macOS process.

## Out Of Scope

- Editing Markdown.
- Mermaid rendering.
- Shiki-style syntax highlighting.
- HTML rendering or `WKWebView`.
- Code signing, notarization, or distribution packaging.
