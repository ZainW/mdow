# Mdow for Mac

Mdow, built the way Apple intends Mac apps to be built: Swift on AppKit and TextKit, using the
system's own controls throughout. There's no web view for markdown, no custom theme and no
cross-platform UI layer. It uses system fonts, semantic colours and the system's light and
dark appearance, and on macOS 26 and later it gets Liquid Glass.

## Requirements

- macOS 15 or later to run
- Xcode 26 or later to build (an older SDK builds too, but the app then runs in the
  pre-Liquid Glass look)

## Run, test, install

From the repo root:

```sh
pnpm run dev:mac README.md docs/   # debug build, run in the foreground
pnpm run test:mac                  # swift test
pnpm run install:mac               # release build into ~/Applications (DEST=… to change)
```

Or in this folder: `swift build`, `swift test`, and `scripts/bundle.sh`, which writes
`dist/Mdow for Mac.app`, signed ad hoc unless `SIGN_IDENTITY` is set. The first build
downloads Pele (pinned and checksummed) into `Resources/vendor/` with `scripts/fetch-pele.sh`.

Run the bundle (`scripts/bundle.sh` or `run.sh`) rather than `swift run`. A bare SwiftPM binary
has no Info.plist, and SwiftPM stamps it with the deployment target as its SDK version, so
macOS shows it in the compatibility look. `bundle.sh` fixes both.

## What's in it

- **Reader**: GitHub-flavored markdown (Apple's [swift-markdown](https://github.com/swiftlang/swift-markdown))
  with tables, task lists, alerts, footnotes, frontmatter titles, and the raw HTML that READMEs
  use (centred logos, badges, `<br>`, `<kbd>`, `<details>`). It renders to an `NSAttributedString`
  made of TextKit parts: `NSTextBlock` for code and quotes, `NSTextTable` for tables, and
  attachments for images. This means selection, copy, **the system find bar**, Look Up, Services,
  Speech, printing and Export as PDF all come from AppKit.
- **Code**: a hand-written Swift highlighter for about 50 languages, in Xcode's default light and
  dark colours. Hovering a code block shows a copy button.
- **Mermaid**: laid out by [Pele](https://pele.run) in JavaScriptCore, then drawn as vector
  SVG through `NSImage`. No browser engine is involved.
- **HTML files**: shown in a `WKWebView` with JavaScript turned off.
- **Windows**:
  - Native window tabs (⌘T, the tab bar's +, Merge All Windows) and back/forward history per tab.
  - A unified toolbar with Share.
  - An `NSSplitViewController` sidebar showing the Outline (it follows your scroll position),
    the Files in the folder (with a filter), or Recents.
  - Quick Open (⌘K) for headings, files and commands.
  - A SwiftUI Settings window, and the proxy icon and path menu in the title.
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
Mdow › Check for Updates…). Its feed is
`https://github.com/ZainW/mdow/releases/download/mac-latest/appcast.xml` (`SUFeedURL` in
`Resources/Info.plist`), and updates are verified against `SUPublicEDKey`, the same EdDSA key
pair Mdow Native uses (`apps/native/updater/sparkle-public-ed-key.txt`; the private half is the
`SPARKLE_PRIVATE_ED_KEY` repo secret). Only builds signed with a Developer ID update: ad hoc
and debug builds leave Sparkle off and the menu item disabled.

`bundle.sh` embeds `Sparkle.framework` (from SwiftPM's artifacts) in `Contents/Frameworks`,
drops its XPC services (the app isn't sandboxed) and signs it inside out. With `SIGN_IDENTITY`
set it signs with the hardened runtime and a secure timestamp, ready for notarization.

The version lives in `VERSION` (`VERSION=… scripts/bundle.sh` overrides it); the build number is
the commit count. To release:

1. Bump `VERSION` (e.g. `0.2.0`) and commit it on `main`.
2. Tag that commit `mac-0.2.0` and push the tag.

`.github/workflows/macos.yml` then builds and tests, checks the tag matches `VERSION`, signs with
the Developer ID (`CSC_LINK`, `CSC_KEY_PASSWORD`), notarizes and staples (`APPLE_ID`,
`APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`), zips the app, and writes `appcast.xml` with
Sparkle's `generate_appcast`, signed with `SPARKLE_PRIVATE_ED_KEY`. It publishes
`Mdow-for-Mac-<version>.zip` on the `mac-<version>` pre-release, then uploads `appcast.xml` to the
rolling `mac-latest` release (creating it the first time), which is what installed copies read.

## Not included

- The OpenCode companion.
- KaTeX math: TeX source is shown as written.
- Find in HTML files.
- A split view inside one window. Use macOS window tiling instead.

## Layout

| Path                         | What it does                                                   |
| ---------------------------- | -------------------------------------------------------------- |
| `Sources/MdowKit/Document`   | file kinds, frontmatter, footnotes, heading slugs              |
| `Sources/MdowKit/Render`     | markdown → `NSAttributedString`, text blocks, styles, HTML bits |
| `Sources/MdowKit/Highlight`  | syntax highlighter and Xcode-style code colours                |
| `Sources/MdowKit/Files`      | folder scanning, file watching, fuzzy matching                 |
| `Sources/Mdow/App`           | app delegate, menus, settings storage, images and diagrams     |
| `Sources/Mdow/Window`        | window controller (tabs, toolbar, history), reader, text view  |
| `Sources/Mdow/Sidebar`       | the source list                                                |
| `Sources/Mdow/Panels`        | welcome screen, Settings, Quick Open, shortcuts (SwiftUI)      |
| `Sources/Mdow/Reader`        | Mermaid through Pele                                           |

Settings live in the `com.zain.mdow.Mac` user defaults.

### Checking changes without clicking around

- `MDOW_SNAPSHOT=file.md swift test --filter snapshot` renders a document offscreen to
  `/tmp/mdow-snapshot.png`. Add `MDOW_SNAPSHOT_DARK=1` for dark mode.
- `MDOW_ACTIONS` runs steps once the app has launched, for screenshots. For example:

  ```sh
  MDOW_ACTIONS="appearance=dark;sidebar=files;palette;find=Pele;scroll=diagrams" \
    "dist/Mdow for Mac.app/Contents/MacOS/Mdow" -ApplePersistenceIgnoreState YES README.md
  ```

  The steps are `appearance`, `sidebar`, `palette`, `find`, `scroll`, `settings`, `shortcuts`,
  `newtab`, `open` and `zoom`.
