# Mdow for Mac

Mdow in Swift, on AppKit and TextKit, with a look of its own rather than the stock Mac one:
the document sits on a rounded sheet of paper inside a flat window, with tabs in the header,
a quiet sidebar and one clay accent. Light mode is warm paper, dark mode neutral ink. Text is set
in Charter, the interface in Inter and code in Geist Mono (both bundled, `Resources/Fonts`).
There's no web view for markdown and no cross-platform UI layer; the chrome is drawn with AppKit
views, and the colours live in one place (`Palette`, shared by the reader and the window).

## Requirements

- macOS 15 or later to run
- Xcode 26 or later to build

## Run, test, install

From the repo root:

```sh
pnpm run dev:mac README.md docs/   # debug build, run in the foreground
pnpm run test:mac                  # swift test
pnpm run install:mac               # release build into ~/Applications (DEST=… to change)
```

Or in this folder: `swift build`, `swift test`, and `scripts/bundle.sh`, which writes
`dist/Mdow.app`, signed ad hoc unless `SIGN_IDENTITY` is set. The first build
downloads Pele (pinned and checksummed) into `Resources/vendor/` with `scripts/fetch-pele.sh`.

Run the bundle (`scripts/bundle.sh` or `run.sh`) rather than `swift run`. A bare SwiftPM binary
has no Info.plist or bundled fonts, and SwiftPM stamps it with the deployment target as its SDK
version, so macOS shows it in the compatibility look. `bundle.sh` fixes all three.

## What's in it

- **Reader**: GitHub-flavored markdown (Apple's [swift-markdown](https://github.com/swiftlang/swift-markdown))
  with tables, task lists, alerts, footnotes, frontmatter titles, and the raw HTML that READMEs
  use (centred logos, badges, `<br>`, `<kbd>`, `<details>`). It renders to an `NSAttributedString`
  made of TextKit parts: `NSTextBlock` for code and quotes, `NSTextTable` for tables, and
  attachments for images. This means selection, copy, Look Up, Services, Speech, printing and
  Export as PDF all come from AppKit. Find (⌘F) is a small floating bar that highlights every
  match and counts them, for markdown and HTML alike.
- **Code**: a hand-written Swift highlighter for about 50 languages, in soft colours tuned to the
  paper and ink themes. Hovering a code block shows a copy button.
- **Math**: `$…$`, ``$`…`$``, `$$…$$` and ` ```math ` blocks, following GitHub's rules, are
  typeset natively by [SwiftMath](https://github.com/mgriebling/SwiftMath) (CoreText, Latin
  Modern Math) and shown as attachments that follow light and dark. Formulas that don't parse
  show their TeX. Each formula carries its source in `.mdowMathSource`.
- **Mermaid**: laid out by [Pele](https://pele.run) in JavaScriptCore, then drawn as vector
  SVG through `NSImage`, in the same palette as the text. No browser engine is involved.
- **HTML files**: shown in a `WKWebView` with JavaScript turned off.
- **Windows**:
  - Tabs in the window's header (⌘T, ⌘W, ⌘1–⌘9, ⇧⌘[ and ⇧⌘], drag to reorder, middle-click to
    close), each with its own folder, split view and back/forward history.
  - A header with the sidebar, history, Quick Open, companion and ⋯ (Share, Open in Editor,
    Export, Print) buttons. The window buttons sit in it; there's no system toolbar look.
  - A sidebar showing the Outline (it follows your scroll position), the Files in the folder
    (with a filter), or Recents, with Settings and a light/dark switch at the bottom.
  - Quick Open (⌘K) for headings, files and commands.
  - The companion (⌥⌘0, or the sparkle button): chat with OpenCode about the
    document and its folder. It runs a private `opencode serve` (OpenCode 2, found on PATH, the
    login shell's PATH or the usual install folders) and shows every edit as a diff to accept
    or decline; shell commands are declined automatically.
  - A Settings window and a Keyboard Shortcuts window in the same style.
- **Behaviour**:
  - Live reload that keeps your place, and drag and drop onto the window or the Dock.
  - Links to other documents open in place (⌘-click for a new tab), and an Open Recent menu.
  - Windows are restored when the system setting "Close windows when quitting an application"
    is off, as in other document apps.

Shortcuts follow macOS conventions where one exists: Show Sidebar is ⌃⌘S, Back and Forward are
⌘[ and ⌘], and Find uses ⌘F, ⌘G and ⌘E. Otherwise they follow Mdow's: ⌘K for Quick Open, ⌘1–⌘9
for tabs, and ⇧⌘O for Open Folder. Help › Keyboard Shortcuts lists them all.

## Updates and releases

The app updates itself with [Sparkle 2](https://sparkle-project.org) (`Sources/Mdow/App/Updater.swift`,
Mdow › Check for Updates…). Its feed is `mdow-appcast.xml` on the latest GitHub release
(`SUFeedURL` in `Resources/Info.plist`), and updates are verified against `SUPublicEDKey`
(`Resources/sparkle-public-ed-key.txt`; the private half is the `SPARKLE_PRIVATE_ED_KEY` repo
secret). Only builds signed with a Developer ID update: ad hoc and debug builds leave Sparkle off
and the menu item disabled.

`bundle.sh` embeds `Sparkle.framework` (from SwiftPM's artifacts) in `Contents/Frameworks`,
drops its XPC services (the app isn't sandboxed) and signs it inside out. With `SIGN_IDENTITY`
set it signs with the hardened runtime and a secure timestamp, ready for notarization.

It ships with every Mdow release, with the same version, as Mdow on macOS (the Electron app is
released for Windows and Linux only): `bundle.sh` reads it from
`apps/desktop/package.json` (`VERSION=…` overrides it), and the build number is the commit count.
Cut releases the usual way (bump the version, tag `vX.Y.Z`; see `.claude/skills/release`). The
`mac-app` job in `.github/workflows/release.yml` then tests the app, signs it with the Developer
ID (`CSC_LINK`, `CSC_KEY_PASSWORD`), notarizes and staples it (`APPLE_ID`,
`APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`), and uploads `Mdow-<version>.zip`, a copy
named `Mdow.zip`, and `mdow-appcast.xml` (written by Sparkle's `generate_appcast`
and signed with `SPARKLE_PRIVATE_ED_KEY`) to the release. The release stays a draft until the
feed is on it. `.github/workflows/macos.yml` builds and tests every pull request.

## Not included

- The companion's inline review inside the document; edits are reviewed in the companion.
- Some of MathJax's TeX: `\boxed`, `\xrightarrow`, `\overset`, `\substack` and colours.

## Layout

| Path                         | What it does                                                   |
| ---------------------------- | -------------------------------------------------------------- |
| `Sources/MdowKit/Document`   | file kinds, frontmatter, footnotes, heading slugs              |
| `Sources/MdowKit/Render`     | markdown → `NSAttributedString`, text blocks, styles, palette  |
| `Sources/MdowKit/Highlight`  | syntax highlighter and its colours                             |
| `Sources/MdowKit/Files`      | folder scanning, file watching, fuzzy matching                 |
| `Sources/MdowKit/Companion`  | OpenCode client, event translation, conversation state         |
| `Sources/Mdow/App`           | app delegate, menus, settings storage, images and diagrams     |
| `Sources/Mdow/Chrome`        | fonts, header, tab strip, buttons, document card, find bar     |
| `Sources/Mdow/Window`        | window controller, tabs, panes and history, reader, text view  |
| `Sources/Mdow/Sidebar`       | the sidebar                                                    |
| `Sources/Mdow/Panels`        | welcome screen, Settings, Quick Open, shortcuts (SwiftUI)      |
| `Sources/Mdow/Reader`        | Mermaid through Pele                                           |
| `Sources/Mdow/Companion`     | the OpenCode server process, companion service and panel       |

Settings live in the `com.zain.mdow.Mac` user defaults.

### Checking changes without clicking around

- `MDOW_SNAPSHOT=file.md swift test --filter snapshot` renders a document offscreen to
  `/tmp/mdow-snapshot.png`. Add `MDOW_SNAPSHOT_DARK=1` for dark mode.
- `MDOW_ACTIONS` runs steps once the app has launched, for screenshots. For example:

  ```sh
  MDOW_ACTIONS="appearance=dark;sidebar=files;palette;find=Pele;scroll=diagrams" \
    "dist/Mdow.app/Contents/MacOS/Mdow" -ApplePersistenceIgnoreState YES README.md
  ```

  The steps are `appearance`, `sidebar`, `palette`, `find`, `scroll`, `settings`, `shortcuts`,
  `split`, `htmlfind`, `newtab`, `open`, `zoom` and `companion` (`companion=hide`, or
  `companion=ask:text` to send).
- Performance: `scrollbench=14` scrolls the document 14 points per frame and `resizebench`
  narrows and widens the window; both log `MDOWBENCH` with frame times and hitches (frames over
  one and a half refresh intervals). `MDOW_PERF=1` logs how long rendering, showing, asset
  loading and find take, and when the first document reached the screen. A release build
  (`CONFIGURATION=release scripts/bundle.sh`) gives representative numbers.
