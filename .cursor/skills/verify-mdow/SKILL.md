---
name: verify-mdow
description: >-
  Drives the Mdow Electron desktop markdown viewer with an isolated Playwright
  _electron harness. Use when proving a desktop feature, verifying UI after a
  change, or running the verify-mdow launch/doctor/drive/cleanup loop. Does not
  drive GPUI Native or the marketing site.
---

# Verify Mdow (Electron desktop)

Primary surface is the **Electron app** in `apps/desktop` (product name Mdow). A user reads markdown and HTML, opens folders, searches the current document, and changes reading settings.

Other surfaces in this repo, not driven by this skill:

- `apps/gpui` — native Apple Silicon beta. No Playwright attach. Do not treat GPUI as a substitute proof for Electron.
- `apps/web` — marketing site. Out of scope.
- Cursor IDE browser MCP — a web tab without `window.api`. It cannot open this app.

Read `features/README.md` before driving. Prove the entry points the map lists; one convenient path is not coverage for the others.

## Launch

Never start verification with `pnpm run --filter desktop dev` or `pnpm run dev`. In dev, Electron forces userData to `~/Library/Application Support/Mdow Development` and takes the single-instance lock. That is the user's live session.

Use the built app plus an isolated profile, the same way `apps/desktop/perf/electron-render.perf.mjs` already does:

```bash
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs launch \
  --file apps/desktop/perf/fixtures/small.md
```

Welcome (no document):

```bash
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs launch
```

Folder tree without the native Open Folder dialog (verification scaffolding — writes `lastFolder` into the isolated store before start):

```bash
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs launch \
  --file apps/desktop/perf/fixtures/small.md \
  --folder apps/desktop/perf/fixtures
```

Ready means:

- `--file` — renderer shows `.markdown-body h1` (for `small.md` that heading is `Small Baseline`).
- no `--file` — heading `Mdow` on the welcome screen.

Launch builds `apps/desktop` with `pnpm run --filter desktop build` when `apps/desktop/out/main/index.js` is missing. Pass `--rebuild` after renderer or main-process edits.

The helper unsets `ELECTRON_RUN_AS_NODE` (Cursor sets this to `1`; Electron then starts as Node and dies on `BrowserWindow`). Do not put that variable back.

If a run is already live, launch refuses. Cleanup first. Do not attach to a window you did not start.

## Doctor

Run this first whenever anything looks off:

```bash
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs doctor
```

Worth driving only when all of these hold:

- The daemon answers.
- `userData` is under `.verify-mdow/runs/<id>/user-data`.
- `userData` is not `Mdow Development` and not the packaged `…/Mdow` profile.
- `isolated` is `true`.
- `surface` is `reader` or `welcome` as the launch intended.
- `version` matches `apps/desktop/package.json`.

If doctor fails, cleanup. Do not keep clicking.

## Drive

Harness is the `verify-mdow` CLI (Playwright `_electron` behind a Unix socket). Prefer roles and accessible names from this app:

| What     | Handle                                                                                                                                          |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Welcome  | heading `Mdow`; buttons `Open File`, `Open Folder`                                                                                              |
| Reader   | `role=tabpanel` / `.markdown-body`; heading from the document                                                                                   |
| Tabs     | `role=tablist` name `Open documents`; tab name includes the filename and path                                                                   |
| Sidebar  | `role=complementary`/`aside` name `Sidebar`; radiogroup `Sidebar mode`; radios `Recents`, `Folder`, `Outline`; button `Settings`                |
| Palette  | dialog `Command Palette` (sr-only); placeholder `Search files and commands…`; groups `Actions` / `Files`; empty `No matching files or commands` |
| Settings | dialog `Settings`; description `Tune how markdown reads.`; group `Theme` with `System` / `Light` / `Dark`                                       |
| Find     | textbox `Search in document`; live status `N of M` or `No results`; `mark.search-highlight`                                                     |

Shortcuts (renderer, after the window is focused): `Meta+k` palette, `Meta+f` find, `Meta+,` settings, `Meta+b` sidebar.

```bash
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Meta+k
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill \
  --placeholder "Search files and commands..." --value "Settings"
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role option --name "Settings"
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs wait --role dialog --name "Settings"
```

`Open File` and `Open Folder` (welcome, menu, and palette) open a **native** dialog. The harness cannot complete those. Open documents with `launch --file` (real Open With / argv path). Recents and palette Files work after that. Native-dialog entry points stay unproven — do not mark them verified via argv.

Do not send `file:opened` or other main-process IPC to skip the user path. The perf suite does that for timing; this skill does not.

## Evidence

Write proof under `.verify-mdow/evidence/<feature>/`. Cleanup must not delete that tree.

Standards:

- Exercise a user path from the feature map, not store setters or test-only IPC.
- Capture the action and the resulting state (screenshot + ARIA snapshot), not only the last frame.
- For find, assert `mark.search-highlight` and the `N of M` status, not just that the search box opened.
- For settings, assert the pressed theme option and a visible change (`html.dark` / preview), not only that the dialog title exists.
- Side effects that exist: recents persist in the isolated store after a file open; session tabs restore on the next launch of **this** run's profile. Do not inspect the user's `Mdow Development` store.
- No mocks. The built app talks to the real filesystem inside the isolated userData dir.

```bash
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs screenshot \
  --path .verify-mdow/evidence/open-document/reader.png
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs snapshot --aria \
  --path .verify-mdow/evidence/open-document/reader.aria.txt
```

Record the feature id and entry point in the artifact names or a `proof.json` next to them.

## Cleanup

```bash
node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs cleanup
```

This closes the Playwright-launched app and deletes `.verify-mdow/runs/<id>/` (profile, socket, daemon log). It never deletes `.verify-mdow/evidence/`.

Kill only the daemon and Electron PIDs written in that run's `run.json`. Never `pkill Electron` or `killall Mdow`.

After a failed launch or a hung drive, run cleanup before the next launch.

## Helpers

The CLI is `.cursor/skills/verify-mdow/scripts/verify-mdow.mjs` (wrapper: `scripts/verify-mdow`). Invoke it from the repo root with `node` as shown above.

| Command                                        | Purpose                                                    |
| ---------------------------------------------- | ---------------------------------------------------------- |
| `launch [--file] [--folder] [--rebuild]`       | Build if needed, start isolated Electron, wait until ready |
| `doctor` / `status`                            | Read-only health of the live instance                      |
| `click` / `fill` / `press` / `wait` / `text`   | Drive the renderer                                         |
| `screenshot --path` / `snapshot --aria --path` | Proof artifacts                                            |
| `cleanup`                                      | Tear down the instance this run started                    |

Two instances cannot share a profile. Isolated runs may coexist with the user's own Mdow window; still launch only through this helper so doctor can prove isolation.
