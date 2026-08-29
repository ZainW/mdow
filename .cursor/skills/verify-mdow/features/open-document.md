# Open a document

Open a document puts a markdown or HTML file into a tab and shows it in the reader, or shows an error tab when the path cannot be read.

## Sub-features

- `open-argv` opens a markdown file passed on the command line (Open With / CLI).
- `open-welcome` shows the empty-window welcome with `Open File` and `Open Folder`.
- `open-recents` reopens a file from the Recents list after it has been opened once.
- `open-reader` renders the document heading inside `.markdown-body`.
- `open-html` opens `.html` / `.htm` in the sandboxed HTML viewer (no find, no outline headings).

## How to get to it (user POV)

- Open a file with the app (Finder Open With, or a path after the Electron app directory).
- Choose `Open File` on the welcome screen.
- Drop a `.md` or `.html` file on the window.
- Choose a row under Recents in the sidebar or on the welcome Recent column.
- Choose a file in the command palette Files group.
- Choose a file in the Folder tree after a folder is open.

## Driving it with verify-mdow

Preconditions:

- No other verify-mdow run is live.
- `apps/desktop/perf/fixtures/small.md` exists.
- `verify-mdow doctor` is not yet running; launch creates the instance.

- **Argv open.** Start the app on the small fixture. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs launch --file apps/desktop/perf/fixtures/small.md`. Ready when `.markdown-body h1` is visible. `doctor` reports `surface: reader` and `heading: Small Baseline`.
- **Reader state.** Confirm the selected tab and the document heading. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs wait --role heading --name "Small Baseline"` and `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs text --selector ".markdown-body h1"`. The text is `Small Baseline`. A tab named with `small.md` is selected under `Open documents`.
- **Welcome entry.** Quit this run and launch with no file. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs cleanup` then `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs launch`. Doctor reports `surface: welcome`. Buttons `Open File` and `Open Folder` are visible. Choosing either opens a native dialog — stop; that path is unreachable.
- **Recents entry.** After an argv open in the same isolated profile, choose Recents if it is not already selected. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role radio --name "Recents"` then `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role button --name "small.md"`. The reader heading stays `Small Baseline` (or returns to it if another tab was active).
- **Proof.** Capture the reader after argv open. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs screenshot --path .verify-mdow/evidence/open-document/reader.png` and `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs snapshot --aria --path .verify-mdow/evidence/open-document/reader.aria.txt`. Both show Mdow, the `small.md` tab, and `Small Baseline`.

## Gotchas

- `Open File`, menu Open, and palette `Open File` use a native dialog. Do not click them in this harness. Do not count `launch --file` as proof of those buttons.
- Drop uses Electron `getPathForFile`. Playwright cannot synthesize a real filesystem `File` here. Treat drop as unreachable unless a later helper learns a working drop.
- `Dev samples` exists only in `electron-vite dev`. The built verify instance does not show it.
- HTML files render in a sandboxed iframe. Find and outline do not apply. Do not use `small.md` proof as HTML proof.
- `file:opened` IPC (used by the perf suite) is not a user path. Do not use it for this feature.
