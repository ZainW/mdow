# Mdow for Mac roadmap

This is a working plan, not a promise. It comes from a review of `apps/macos` in October 2026,
after the Swift app replaced Electron in v1.16.1. Paths are relative to `apps/macos/Sources/`.

The base is solid: the reader renders natively, there are good debugging hooks (`MDOW_ACTIONS`,
snapshots, benchmarks), and the OpenCode companion is something the Linux app doesn't have. What's
left falls into three groups: rough edges, how well the app fits into macOS, and how it handles
real-world READMEs and large folders.

## 1.17: Polish

Small fixes, mostly bugs a reviewer would notice in the first few minutes.

- [ ] Tabs that don't fit the strip are hidden and can't be reached, and the selected tab can be
      one of them (`Mdow/Chrome/TabStrip.swift:80`). Add an overflow menu, or shrink tabs further
      and scroll the strip.
- [ ] Add a right-click menu on tabs, as Linux has: Close Other Tabs, Close Tabs to the Right,
      Copy Path, Show in Finder, Move to Split.
- [ ] Watch the open folder, so the Files tree updates when files are added or removed. Today
      only the open file is watched and the tree changes only on ⌘R.
- [ ] Keep the outline's collapsed sections across live reloads
      (`Mdow/Sidebar/SidebarViewController.swift:244`; `outlineViewItemDidCollapse` is a stub).
- [ ] Stop ⌘1–9 from switching tabs while typing in the companion (`Mdow/App/AppDelegate.swift:143`).
- [ ] Jump to `other.md#heading` once the document has rendered, not after a fixed 250 ms
      (`Mdow/Window/DocumentPane.swift:348`).
- [ ] Say so when a link points to a missing file, instead of only beeping
      (`Mdow/Window/DocumentPane.swift:359`).
- [ ] Resolve `<a href>` around HTML images against the document's folder
      (`MdowKit/Render/HTMLSnippet.swift:71`).
- [ ] Give each window its own saved frame instead of one shared autosave name
      (`Mdow/Window/DocumentWindowController.swift:104`).
- [ ] Export `com.zain.mdow.mdx` instead of importing it (`Resources/Info.plist`).
- [ ] List every command in the Keyboard Shortcuts window, and add the hold-⌘ cheat sheet the
      website's shortcuts page already promises.
- [ ] Accessibility: keyboard focus and a close action on tabs, on/off state for header toggle
      buttons (`Mdow/Chrome/ChromeButton.swift`), and Reduce Motion on the Welcome cards
      (`Mdow/Panels/WelcomeView.swift:116`).

## 1.18: Reading

- [ ] Footnote popovers on hover, and link previews for `.md` links (title and first paragraph).
      Link tooltips are currently turned off (`Mdow/Window/ReaderTextView.swift:59`).
- [ ] Image lightbox on click; animate GIFs; cap the height of tall images.
- [ ] Wide tables scroll sideways, or break out wider like code blocks, instead of squeezing
      every column into the text width (`MdowKit/Render/MarkdownRenderer.swift:522`).
- [ ] `<details>` as a real disclosure that starts closed.
- [ ] Show frontmatter as a small folded metadata card. Recognise TOML `+++` frontmatter, which
      shows up as body text today (`MdowKit/Document/SourceFile.swift:46`).
- [ ] Common extensions: `==highlight==`, `<mark>`, emoji shortcodes, `[[wiki links]]`, and
      custom alert titles (`> [!NOTE] Title`).
- [ ] Find options: Match Case, Whole Word, Regular Expression; a match count for HTML files.
- [ ] A print style: page margins, page breaks before top-level headings, link URLs.

## 1.19: Fitting into macOS

- [ ] A Quick Look extension, so pressing Space on a `.md` file in Finder shows Mdow's rendering.
      MdowKit is already separate from the app, so it can be shared.
- [ ] An `mdow` command (`mdow README.md`, `cat x.md | mdow`) with an Install Command Line
      Tool… menu item.
- [ ] Offer to become the default app for markdown. The document types are registered as
      `Alternate` today.
- [ ] App Intents (Open in Mdow, Export as PDF) and Spotlight entries for recent documents.
- [ ] Drag a tab out into a new window, or into another window.

## 1.20: Docs browser

- [ ] Search across the folder (⇧⌘F): full-text results grouped by file.
- [ ] Export as HTML (themed, self-contained), Copy as Rich Text, and Copy as Markdown for a
      selection.
- [ ] Session restore of tabs and split view that doesn't depend on the system setting "Close
      windows when quitting an application", as on Linux.

## 2.0: Companion and speed

- [ ] Review the companion's edits inside the document, as tracked changes, rather than only in
      the companion panel.
- [ ] Re-render by block instead of rebuilding the whole `NSAttributedString` on every reload,
      resize or theme change (`Mdow/Window/DocumentPane.swift:263`).
- [ ] Cache decoded images, load each remote image independently instead of waiting for all of
      them, and cap download size (`Mdow/App/AssetStore.swift`).

## Outside the app

- The website's features page still mentions Shiki and ACP/Codex providers
  (`apps/web/content/docs/features.md`), and the changelog has no entry for the GTK Linux app.
