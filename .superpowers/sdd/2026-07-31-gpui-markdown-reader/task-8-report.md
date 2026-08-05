# Task 8 Report — Local Run Script and Visual Verification

Base commit: `0671f53 fix(gpui): restore native File menu`

## Implementation

- Added executable `script/build_and_run_gpui.sh` with the exact interface
  `./script/build_and_run_gpui.sh [--verify] [path]`.
- The launcher resolves the repository from its own location, runs Cargo from that root with the
  GPUI target directory fixed to `apps/gpui/target`, and works from arbitrary caller directories
  with spaces in both repository and document paths.
- Full Xcode and the installed Metal toolchain are discovered dynamically. A candidate is accepted
  only when both `xcrun -sdk macosx metal -v` and `metallib -v` succeed with the selected
  `DEVELOPER_DIR`/`TOOLCHAINS`; failures include an actionable install command and the last xcrun
  diagnostic.
- Earlier processes are enumerated with `lsof`/`pgrep` but signalled only after `lsof` proves their
  canonical executable path exactly equals the built binary. Normal launch uses foreground `exec`.
  Verify launch owns `$!`, requires repeated exact-PID/executable observations before a hard
  ten-second deadline, prints exactly `Mdow GPUI verification passed`, and terminates/reaps only its
  child. EXIT and signal traps prevent orphans.
- Added executable behavioral coverage in `script/test_build_and_run_gpui.zsh` and ignored only the
  generated GPUI `/target/` directory in `apps/gpui/.gitignore`.
- No theme or UI source was changed: fixed-size screenshot inspection found no evidence-backed
  mismatch. Previously documented GPUI 0.2.2 rendering limitations remain unchanged.

## Launcher Contract Tests

`zsh script/test_build_and_run_gpui.zsh` passes 7/7 behavioral scenarios:

1. Invalid grammar returns EX_USAGE before Cargo.
2. Arbitrary CWD, spaces, relative leading-dash paths, owned Cargo target, and both Metal tools.
3. Verify default fixture, exact success text, child termination, and reap.
4. Exact executable-path prior-process termination while a same-name decoy survives.
5. Foreground `exec` PID/signal behavior without an orphan.
6. Actionable missing-Metal failure before Cargo.
7. Cargo failure propagation without application launch.

## Fresh Verification

Every Cargo command used:

```text
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
TOOLCHAINS=com.apple.dt.toolchain.Metal.32023.883
```

Results:

```text
cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check
# exit 0

cargo check --manifest-path apps/gpui/Cargo.toml --all-targets
# exit 0, no warnings

cargo test --manifest-path apps/gpui/Cargo.toml
# 87 library + 2 binary = 89 passed, 0 failed

cargo build --manifest-path apps/gpui/Cargo.toml
# exit 0, no warnings

./script/build_and_run_gpui.sh --verify apps/gpui/tests/fixtures/showcase.md
# Finished dev profile; Mdow GPUI verification passed

pnpm run test
# 50 desktop files / 340 tests and 8 web files / 28 tests passed (Turbo cache replay)

rg -n -i "companion|chat|ai[-_ ]|openai|anthropic|ACP" \
  apps/gpui script/build_and_run_gpui.sh
# no matches (rg exit 1)
```

The first sandboxed Cargo test run passed 82 tests but five genuine FSEvents integration tests did
not receive filesystem events, matching Task 7's documented managed-sandbox limitation. The exact
same command rerun outside that filesystem sandbox passed all 89 tests. The live native runtime
also reloaded, reported deletion, and recovered successfully.

## Required Window-only Screenshots

All captures are uncommitted temporary PNGs and were verified as exactly 1120×760:

- Welcome, light: `/private/tmp/mdow-task8-welcome-light.png`
- Showcase + folder, light: `/private/tmp/mdow-task8-showcase-folder-light.png`
- Showcase + folder, dark: `/private/tmp/mdow-task8-showcase-folder-dark.png`
- Two tabs, second active: `/private/tmp/mdow-task8-two-tabs-second-active.png`

Additional native interaction evidence includes:

- `/private/tmp/mdow-task8-copy-feedback-cgevent.png`
- `/private/tmp/mdow-task8-folder-disclosure.png`
- `/private/tmp/mdow-task8-tab-switch.png`
- `/private/tmp/mdow-task8-tab-close.png`
- `/private/tmp/mdow-task8-sidebar-hidden.png`
- `/private/tmp/mdow-task8-wide-mode.png`
- `/private/tmp/mdow-task8-local-link.png`
- `/private/tmp/mdow-task8-live-reload.png`
- `/private/tmp/mdow-task8-delete-banner.png`
- `/private/tmp/mdow-task8-recovery.png`

## Visual Audit

- Raster rules measure sidebar 244 px, titlebar 28 px, tab bar 36 px, active tab 28 px, and
  breadcrumb 28 px. The constrained card is about 672 px plus two 48 px insets, matching the
  768 px reader. Wide mode begins at sidebar 244 + inset 48 and expands to the right inset.
- Light samples are warm (`#fbf8f5`, muted/code `#f4f1ed`); dark samples are neutral (`#090909`,
  muted/code `#141414`).
- Active row accent, lifted active tab, borders, full table grid, blockquote rule, code card,
  welcome hierarchy, H1–H6 hierarchy, task/list styles, and intrinsic 32×32 local image are visible.
- No chat, companion, AI, split-view control, or reserved companion space is visible.
- Raster evidence distinguishes proportional and mono faces. Exact identity is backed by startup
  loading `InterVariable.ttf` and `GeistMono-Variable.ttf` and theme families `Inter Variable` /
  `Geist Mono`. Source and tests also pin body typography to 15.5 px at 1.65 line height.

## Native Interaction Audit

The raw GPUI PID/window was controlled directly with PID-scoped macOS events; Computer Use exposes
the installed Electron app rather than this unbundled debug process.

- Open File and Open Folder displayed native panels; the folder selection populated the sidebar.
- Folder disclosure, file activation, tab switching, Command-W tab close, Command-B sidebar
  toggle, Command-Shift-W wide mode, and Command-Q all worked in the native window.
- A local Markdown link opened `nested/guide.md` in a new active tab. The local image rendered at
  its intrinsic 32×32 size.
- Code copy displayed `Copied` plus a check without reflow; `pbpaste` returned exactly
  `exact clipboard proof\nsecond line`.
- On-disk heading edit reloaded in the same window. Moving the file produced the readable
  `Couldn't reload this file` banner while keeping the last good document; restoration cleared the
  banner and restored content.
- Raw external drag automation was not safely targetable because the unbundled GPUI process is not
  exposed to Computer Use. The passing native integration test
  `external_drag_enter_and_exit_update_the_rendered_drop_state` plus `open_paths` coverage verifies
  the drop route and visual enter/leave state.
- The HTTP-link click was deliberately not executed after the host rejected it as unauthorized
  browser network egress. The passing `link_routes_distinguish_mdow_documents_web_urls_and_local_files`
  test verifies HTTP classification/routing without claiming a browser launch.

No visual correction was warranted by the captured evidence.

## Post-completion Review Fix

A final broad review found that nested `read_dir`, directory-entry, metadata, and canonicalization
failures could be silently converted into a successful but incomplete workspace tree. A focused TDD
fix now propagates the first genuine nested failure as `WorkspaceError::Read` with the failing path.
The app-model regression proves the previous workspace and open tab remain intact while the concise
sidebar error is set. Ignored names, broken symlinks, and symlink-cycle handling remain non-fatal and
covered. The original RED returned a partial tree containing only `visible.md`; the focused GREEN
workspace group passes 5/5. Fresh final verification passes 90 library + 2 binary tests, Cargo
format check, and Cargo check for all targets.
