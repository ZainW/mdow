# Find in document

Find searches the active markdown document, highlights matches, and moves between them. It is not available on HTML tabs.

## Sub-features

- `find-open` opens the search bar from the keyboard or the palette.
- `find-match` highlights matches and shows `N of M`.
- `find-empty` shows `No results` for a query with no matches.
- `find-next` / `find-prev` move the active highlight.
- `find-close` dismisses the bar with Escape or the close button.

## How to get to it (user POV)

- Press `⌘F` (macOS) or `Ctrl+F` while a markdown tab is active.
- Run `Find in Document` from the command palette.
- Use the app menu Find command.

## Driving it with verify-mdow

Preconditions:

- A verify-mdow instance is live on `apps/desktop/perf/fixtures/small.md`.
- The active tab is markdown, not HTML.
- `verify-mdow doctor` reports `surface: reader`.

- **Keyboard open.** Press find. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Meta+f`. A textbox named `Search in document` appears.
- **Match.** Type a word from the fixture. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill --role textbox --name "Search in document" --value "fixture"`. Status text becomes `1 of 1` (or `1 of N` if more). At least one `mark.search-highlight` is visible. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs wait --selector "mark.search-highlight"`.
- **Empty.** Replace the query. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill --role textbox --name "Search in document" --value "zzzxq"`. Status text is `No results`. No `mark.search-highlight` remains.
- **Next / previous.** Restore `fixture`, then click `Next match` and `Previous match`. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill --role textbox --name "Search in document" --value "the"` then `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role button --name "Next match"`. The status index advances when more than one match exists. `mark.search-highlight-active` marks the current hit.
- **Close.** Press Escape. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Escape`. The search textbox is gone. Highlights are removed.
- **Palette entry.** Reopen find from the palette. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Meta+k`, `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill --placeholder "Search files and commands..." --value "Find in Document"`, and `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role option --name "Find in Document"`. The search textbox appears again.
- **Proof.** Capture a matching state (`fixture`, `1 of 1`, highlight visible). Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs screenshot --path .verify-mdow/evidence/find-in-document/match.png` and `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs snapshot --aria --path .verify-mdow/evidence/find-in-document/match.aria.txt`. Both show the search bar, the status, and the reader.

## Gotchas

- Find does not open on HTML documents. An HTML tab is the wrong precondition, not a product bug in find.
- Opening the bar without a query shows no status and no marks. That is not a match proof.
- Highlights live in the rendered markdown, not in the source. Assert `mark.search-highlight`, not the raw file.
- Menu Find goes through IPC (`onMenuFind`). This harness does not click native menus. Use `Meta+f` or the palette.
