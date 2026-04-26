---
name: Distribution and Auto-Update Design
description: How Mdow ships releases, how the website surfaces them, and how the desktop app updates itself
date: 2026-04-26
status: draft
---

# Distribution and Auto-Update

## Goal

Ship Mdow releases through GitHub Releases as the single source of truth, surface the latest build on the marketing site at `/download`, and enable in-app updates on Windows and Linux. macOS distribution goes through a Homebrew Cask tap so users get one-command installs and `brew upgrade`-driven updates without paying for an Apple Developer ID.

## Non-Goals

- Apple Developer ID / notarization / Mac in-app auto-update. Deferred until there's revenue or a clear user-pain reason to spend $99/year. The `electron-updater` Mac path stays disabled rather than fail loudly.
- Windows code signing certificate. Users will see SmartScreen "Unknown publisher" once on first install; we accept that for now.
- Delta updates, staged rollouts, A/B channels, beta channels. v1 is "everyone gets latest stable."
- License keys, paid gating, or update-server proxying through the website. Out of scope; revisit when monetization lands.

## Source of Truth: GitHub Releases

`apps/desktop/electron-builder.yml` already declares `publish: github` — we keep that. Each release the maintainer cuts produces these assets, named per electron-builder defaults so the website and updater can find them by pattern:

- `Mdow-<version>.dmg` and `Mdow-<version>-mac.zip` (mac, both arm64 and x64 if we build universal; otherwise `-arm64` / `-x64` suffixes)
- `Mdow-Setup-<version>.exe` (Windows NSIS)
- `Mdow-<version>.AppImage` (Linux)
- `latest.yml`, `latest-mac.yml`, `latest-linux.yml` (electron-updater manifests; produced automatically by `electron-builder --publish always`)

Releases are cut manually for now: `pnpm --filter desktop run publish` from a clean main with `GH_TOKEN` set. CI-driven releases are explicitly deferred — we want a human in the loop until v1.x stabilizes.

## Website: Live Download Page

Replace the hardcoded `#` URLs in `apps/web/src/routes/download.tsx` with live data from `api.github.com/repos/ZainW/mdow/releases/latest`.

**Server-side fetch in the route loader.** TanStack Start's `createServerFn` runs on Cloudflare Pages' edge functions. The loader:

1. Fetches `https://api.github.com/repos/ZainW/mdow/releases/latest` with `Accept: application/vnd.github+json` and a `User-Agent` header (GitHub requires it).
2. Returns the existing OS detection plus a structured payload: `{ version, publishedAt, assets: { mac: { dmg, zip }, windows: { exe }, linux: { appImage } } }`.
3. Filename matching is by suffix pattern (`.dmg`, `.AppImage`, `Setup.exe`, etc.), not by full name — version numbers shift.
4. Cached at the Cloudflare edge for 10 minutes via `Cache-Control: public, max-age=600, s-maxage=600`. GitHub's API is rate-limited (60/hr unauthenticated per IP), so caching matters.
5. On fetch failure, returns `null` for the release payload; the page renders a fallback ("Downloads temporarily unavailable — visit the [Releases page](...)").

**UI changes:**

- Each platform card shows the version number and release date.
- Mac card adds a `brew install --cask zainw/mdow/mdow` snippet with a copy button alongside the .dmg/.zip buttons.
- "View all releases" link to the GitHub releases page in the footer of the download grid.

No client-side JS for the fetch. Everything renders server-side; the page works without JS.

## Desktop: In-App Updater (Windows + Linux only)

**What's already in place** (verified in code, not reimplementing):

- `apps/desktop/src/main/updater.ts` — full event wiring with `autoDownload = false`, `autoInstallOnAppQuit = true`, and IPC forwards for `update-available`, `up-to-date`, `download-progress`, `update-downloaded`, `error`. Exports `initAutoUpdater`, `checkForUpdates`, `downloadUpdate`, `installUpdate`.
- `apps/desktop/src/main/ipc.ts` — handlers for `updater:check`, `updater:download`, `updater:install`.
- `apps/desktop/src/main/index.ts` — calls `initAutoUpdater(getMainWindow)` at startup.
- `apps/desktop/src/preload/index.ts` — `window.api` exposes `checkForUpdates`, `downloadUpdate`, `installUpdate`, plus subscriber methods for each updater event.

So the main-process plumbing and IPC bridge are done. What's missing is platform gating, periodic re-checks, the menu item, the renderer UI, the settings toggle, and types for the renderer-side `window.api` updater surface.

**Changes to existing updater module (`updater.ts`):**

- Skip initialization entirely on `process.platform === 'darwin'`. On Mac, `initAutoUpdater` is a no-op so a failed Squirrel.Mac call can't surface as an error event. Reason: a half-working updater is worse than an honest external link.
- Replace the single `setTimeout` startup check with: initial check after 30s, then `setInterval` every 4 hours while the app runs.
- Track a small in-process state (`'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error'`) so manual `checkForUpdates` can give a meaningful response (e.g., distinguish "checking…" from "up to date") for the manual-check UX.

**Changes to menu (`menu.ts`):**

- Add `Help → Check for Updates…` menu item.
- On Win/Linux: triggers `checkForUpdates()` and signals the renderer that this was a manual check (so the toast appears even on the "up to date" outcome — the auto-check stays silent in that case).
- On Mac: opens `https://github.com/ZainW/mdow/releases/latest` via `shell.openExternal`.

**New renderer UI:**

- A small `<UpdateNotification />` component mounted at the App root, subscribing to the existing preload event hooks.
- States: `available` shows "Mdow {version} is available · [Download] · [Dismiss]". `downloading` shows progress. `ready` shows "Update ready · [Restart now] · [Later]". `up-to-date` only shown when triggered by manual check (uses a "wasManual" flag passed from main).
- Errors silently logged via `console.warn`; no UI surface.
- Visual treatment (applying Emil Kowalski's design engineering principles):
  - **Bottom-right fixed toast**, not a top banner. Top banners push content down (layout shift) or cover markdown content; a fixed toast does neither and matches the pattern users know from VS Code / Cursor / Linear.
  - Dismissible with an explicit close affordance. The `available` state can be dismissed to "remind me later" (toast hides until next periodic check brings it back). The `ready` state is dismissible to "later" — meaning install on quit, the electron-updater default.
  - Entrance: `transform: translateY(8px)` + `opacity: 0` → 0,1 over ~180ms, ease-out. Exit: ~120ms, ease-in. Honor `prefers-reduced-motion` (instant, no transform).
  - Action buttons (Download, Restart now, Later, Dismiss) are 44px tap targets minimum. Each icon-only button gets an `aria-label`.
  - No `transition: all` — animate `transform` and `opacity` only.
  - Progress (downloading state) uses `font-variant-numeric: tabular-nums` for the percentage so it doesn't jitter.

**Settings:**

- Add `autoUpdate.enabled` (default `true`) to the existing `electron-store` schema.
- When `false`, skip the periodic interval and the startup check; the manual menu item still works.
- Add an "Updates" section to `settings-dialog.tsx` with a single toggle. Hide the section on Mac.

**Types:**

- Add a `Window.api` interface (or augment the existing one) in a `.d.ts` so the renderer is type-safe when calling `window.api.checkForUpdates()` etc. The preload already exposes these — this just gives them types.

## macOS: Homebrew Cask Tap

Create `github.com/ZainW/homebrew-mdow` (a separate, public repo). It contains one file: `Casks/mdow.rb`. The cask points at the GitHub Release `.dmg` for the latest version and uses `livecheck` against the GitHub releases endpoint, with `auto_updates false` (since brew handles the update, not the app itself).

Each release, the formula needs its `version` and `sha256` updated. v1 approach: a small shell script in `apps/desktop/scripts/update-homebrew-cask.sh` that:

1. Reads the version from `apps/desktop/package.json`.
2. Downloads the published `.dmg` from the GitHub release.
3. Computes its SHA-256.
4. Clones the tap repo, edits the `version` and `sha256` lines, commits, pushes.

The release flow becomes: `pnpm --filter desktop run publish` → wait for assets to upload → `pnpm --filter desktop run release:homebrew`. Two commands, manual but trivial.

Documenting `brew install --cask zainw/mdow/mdow` on the website's download page closes the loop for users.

## Open Questions Resolved

- **Universal vs separate Mac builds:** Build separate `arm64` and `x64` artifacts (electron-builder default) so download size stays small. Filename matching on the website uses arch suffix when present.
- **Why not just GitHub Releases API for the version check from inside the app on Mac?** We could, but then we're reimplementing what `brew upgrade` already does for free. Keep the surface area small.
- **Why not host on R2 like the old website spec said?** GitHub Releases is free, signed-URL-free now that the repo is public, and electron-updater's GitHub provider does manifest publishing for us. Migrate later only if we need access control for paid builds.

## Files Touched

**New:**

- `apps/desktop/scripts/update-homebrew-cask.sh` — homebrew tap updater
- `apps/desktop/src/renderer/src/components/update-notification.tsx` — toast/banner UI
- `apps/web/src/lib/github-releases.ts` — server-side release fetcher

**Modified:**

- `apps/desktop/src/main/updater.ts` — add macOS guard, periodic re-check, manual-check signaling
- `apps/desktop/src/main/menu.ts` — add "Check for Updates…" item under Help
- `apps/desktop/src/main/store.ts` — add `autoUpdate.enabled` setting
- `apps/desktop/src/preload/index.ts` — add types/methods for manual-check signaling if needed
- `apps/desktop/src/renderer/src/App.tsx` — mount `<UpdateNotification />`
- `apps/desktop/src/renderer/src/components/settings-dialog.tsx` — Updates section (hidden on Mac)
- `apps/desktop/src/renderer/src/env.d.ts` (or new `electron.d.ts`) — type `window.api` updater surface
- `apps/desktop/package.json` — add `release:homebrew` script
- `apps/web/src/routes/download.tsx` — replace hardcoded URLs with loader data
- `apps/web/src/components/download-card.tsx` — show version, release date, brew snippet for Mac

**External (separate repo, not in this monorepo):**

- `github.com/ZainW/homebrew-mdow` — `Casks/mdow.rb`

## Testing

- Unit test the GitHub Releases response → asset map function in `apps/web/src/lib/github-releases.ts` against fixture JSON (real shape from a sample release).
- Unit test the updater state machine: given an `update-available` event, the IPC channel emits the right payload; given `download-progress`, progress flows through.
- Manual smoke test: build a `1.0.1-test` release, install `1.0.0`, confirm the toast appears, downloads, restarts cleanly on Windows and Linux. Mac path: confirm the menu item opens the right URL.
- Website: snapshot test the download page rendering with a mock loader payload. Run `pnpm --filter web build` against the live API once before merging.
