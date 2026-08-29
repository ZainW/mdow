# Sidebar

The sidebar switches between Recents, the open Folder tree, and the current document Outline, and it houses the Settings button.

## Sub-features

- `sidebar-toggle` shows or hides the sidebar with `⌘B`.
- `sidebar-recents` lists recently opened files or the empty `No recents yet` state.
- `sidebar-folder` shows the folder tree, or `No folder open` when none is open.
- `sidebar-outline` lists document headings, or `No document open` / `No headings`.
- `sidebar-settings` opens Settings from the footer.

## How to get to it (user POV)

- Press `⌘B` (macOS) or `Ctrl+B` to show or hide the sidebar.
- Choose `Recents`, `Folder`, or `Outline` in the Sidebar mode group.
- Choose a recent file, a folder-tree file, or an outline heading.
- Choose `Settings` at the bottom of the sidebar.

## Driving it with verify-mdow

Preconditions:

- A verify-mdow instance is live on `apps/desktop/perf/fixtures/small.md` for Outline and Recents.
- For a populated Folder tree, launch included `--folder apps/desktop/perf/fixtures` (scaffolding, not an Open Folder proof).
- `verify-mdow doctor` reports the isolated profile.

- **Visible.** Confirm the sidebar. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs wait --role radio --name "Recents"`. The radiogroup name is `Sidebar mode`.
- **Toggle.** Hide and show. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Meta+b` then again. After the second press, the Recents radio is visible again. Do not leave the sidebar hidden for the rest of the recipe.
- **Outline.** Choose Outline. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role radio --name "Outline"`. A link or row `Small Baseline` is visible. Choose it. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role link --name "Small Baseline"`. The heading stays in view.
- **Folder empty.** On a launch without `--folder`, choose Folder. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role radio --name "Folder"`. Empty copy is `No folder open`.
- **Folder seeded.** On a launch with `--folder apps/desktop/perf/fixtures`, choose Folder. The tree includes `small.md`. That proves restore-from-store, not the Open Folder dialog.
- **Recents.** Choose Recents. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role radio --name "Recents"`. After argv open, `small.md` is listed. On a fresh welcome launch with no prior opens in this profile, copy is `No recents yet` / `Files you open will appear here.`
- **Proof.** Capture Outline on `small.md`. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs screenshot --path .verify-mdow/evidence/sidebar/outline.png` and `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs snapshot --aria --path .verify-mdow/evidence/sidebar/outline.aria.txt`. Both show Sidebar mode Outline and the `Small Baseline` heading row.

## Gotchas

- Outline anchors are links, not buttons. Use `--role link --name "Small Baseline"`.
- Folder scan skips empty directories and stops at depth 8 / 5000 files. A missing nested file may be the scanner, not the sidebar.
- `launch --folder` writes `lastFolder` into the isolated store. Report that as scaffolding. The welcome `Open Folder` button remains unproven.
- Recents keep at most 20 existing paths. A proof that opens 21 files must say which one dropped off.
- When the sidebar is hidden it is `inert`. Clicks on Recents/Folder/Outline will fail; toggle it back with `Meta+b`.
