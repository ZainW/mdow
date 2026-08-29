# Command palette

The command palette searches actions and known files and runs the selected row without leaving the keyboard.

## Sub-features

- `palette-open` opens the palette from the keyboard.
- `palette-actions` lists Actions when the query is empty or matches a command title.
- `palette-files` lists Files from the open folder and from recents.
- `palette-run-action` runs a non-dialog action (Settings, Find, Toggle Sidebar).
- `palette-open-file` opens a file from the Files group.
- `palette-empty` shows `No matching files or commands` when nothing matches.

## How to get to it (user POV)

- Press `⌘K` (macOS) or `Ctrl+K`.
- Run an action by typing part of its title (`Settings`, `Find in Document`, `Toggle Sidebar`) and pressing Enter.
- Open a file by typing part of its filename and choosing it under Files.

## Driving it with verify-mdow

Preconditions:

- A verify-mdow instance is live on `apps/desktop/perf/fixtures/small.md` (`surface: reader`).
- `verify-mdow doctor` reports the isolated profile.
- For Files from a folder, the instance was launched with `--folder apps/desktop/perf/fixtures`. Recents-only Files work after argv open of `small.md` without that flag.

- **Keyboard open.** Focus the window and press the palette shortcut. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Meta+k`. A dialog named `Command Palette` appears. The field placeholder is `Search files and commands...`. Groups include `Actions`.
- **Action match.** Type `Settings`. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill --placeholder "Search files and commands..." --value "Settings"`. An option named `Settings` remains. `Open File` may still match; do not choose it.
- **Run action.** Choose Settings. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role option --name "Settings"`. The palette closes and a dialog named `Settings` with `Tune how markdown reads.` is visible.
- **File match.** Dismiss Settings (`press --key Escape`), reopen the palette, and type `small`. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Meta+k` then `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill --placeholder "Search files and commands..." --value "small"`. The Files group contains `small.md`.
- **Empty state.** Replace the query with a nonsense string. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill --placeholder "Search files and commands..." --value "zzzxq-no-match"`. The list shows `No matching files or commands`.
- **Proof.** Capture the populated Files state (query `small`, before choosing a row). Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs screenshot --path .verify-mdow/evidence/command-palette/files.png` and `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs snapshot --aria --path .verify-mdow/evidence/command-palette/files.aria.txt`. Both identify the Command Palette, the query, and `small.md`.

## Gotchas

- Palette `Open File` and `Open Folder` open native dialogs. Running those actions is not a completable proof in this harness.
- Empty query lists commands first, then files. A Files proof needs a typed filename (or a recent/folder file visible without scrolling past every action).
- `--folder` seeds `lastFolder` in the isolated store. That is scaffolding for Folder/Files, not proof that the user chose Open Folder.
- `⌘K` is swallowed only when the renderer handles it. If the palette does not open, click the document tabpanel first, then press again.
- cmdk options may include the hint text in the accessible name. Prefer `--name "Settings"` (substring) over an exact full-row string.
