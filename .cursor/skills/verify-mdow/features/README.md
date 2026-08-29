# Mdow verification map

This directory is the maintained source for verifying the user-facing behavior of the Mdow Electron desktop app. Read the index before driving the app, then use the matching feature file as the recipe.

## Baseline preconditions

- Launch through `verify-mdow`, never `pnpm run --filter desktop dev`.
- Prefer `apps/desktop/perf/fixtures/small.md` unless a recipe names another file.
- Run `verify-mdow doctor` and require an isolated `.verify-mdow/runs/<id>/user-data` profile, the expected desktop version, and the intended surface (`reader` or `welcome`).
- Never drive a window that this run did not start.
- Unset `ELECTRON_RUN_AS_NODE` is the helper's job. Do not export it for the Electron child.

## Driving conventions

- Start every recipe from the baseline state unless its preconditions say otherwise.
- Prefer ARIA roles and accessible names over CSS selectors or coordinates.
- Treat every command as literal. Keep quoted names and flags unchanged.
- Run all UI actions through `verify-mdow`.
- Native file and folder dialogs are unreachable. Do not report them verified via `launch --file` or a seeded `lastFolder`.
- Restore nothing in the user's `Mdow Development` or packaged `Mdow` profile. Cleanup removes only this run's instance.

## Proof and skip reporting

- Capture the user action and the resulting state, not only the final screen.
- UI proof includes an ARIA snapshot and a screenshot with Mdow identity visible (welcome heading, tab, or Settings title).
- Write artifacts under `.verify-mdow/evidence/<feature>/`. Cleanup must leave them.
- Record the feature ID and entry point used with every artifact.
- Report an unreachable path with the attempted command and the unmet precondition.
- Do not report a skipped entry point as verified through a different path.

## Feature entry contract

Each feature file starts with an H1 title and one paragraph describing the user-visible behavior. It then uses exactly four H2 sections in this order.

1. `Sub-features` lists short IDs with one line for each behavior.
2. `How to get to it (user POV)` lists every user entry point.
3. `Driving it with verify-mdow` starts with `Preconditions:` and uses labeled bullets that pair each user action with an exact command and observable result.
4. `Gotchas` lists traps that can waste or invalidate a verification run.

Keep implementation details out of the map. Name only user paths, stable handles, required state, commands, and observable proof.

## Features

- [Open a document](./open-document.md) covers argv / Open With, welcome, recents, and the rendered reader.
- [Command palette](./command-palette.md) covers opening the palette, running an action, and opening a file from Files.
- [Find in document](./find-in-document.md) covers opening find, matching, empty results, and closing.
- [Settings](./settings.md) covers opening Settings and changing theme from each entry point.
- [Sidebar](./sidebar.md) covers Recents, Folder, and Outline modes.

GPUI Native and the marketing site are not in this map.
