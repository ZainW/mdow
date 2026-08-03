# GPUI CI and Beta Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the Apple Silicon GPUI app as a signed and notarized `Mdow Native.app` beta, validate it on every relevant pull request and push, attach it to version-tag GitHub releases, and expose it as the secondary download on the website while Electron remains the stable default.

**Architecture:** The Rust binary discovers assets from its `.app` bundle at runtime and falls back to the Cargo asset directory only for local development. A macOS packaging script owns the bundle contract, signing, notarization, validation, and two ZIP names. A dedicated GPUI workflow exercises the same checks and package path used by the tag-release job. The web release parser models the GPUI artifact explicitly and renders a separately tested beta download section.

**Tech Stack:** Rust 1.93, GPUI, Bash/Zsh, macOS codesign/notarytool/stapler/spctl, GitHub Actions, TypeScript, React 19, TanStack Start, Vitest, pnpm/Turborepo.

## Global Constraints

- Preserve the Electron desktop app, its release matrix, and its position as the primary stable download.
- Replace the distributed SwiftUI beta with GPUI; do not add a second native-beta release job.
- The public app identity is `Mdow Native.app`; the executable is `MdowNative`; the bundle ID is `com.zain.mdow.gpui`.
- The initial GPUI distribution target is Apple Silicon on macOS 14 or newer.
- Produce both `MdowNative-<version>-arm64-mac-beta.zip` and `MdowNative-mac-beta.zip`.
- CI must fail on missing assets, a non-arm64 binary, an invalid bundle, missing Developer ID credentials on a tagged release, or failed notarization.
- Local packaging may ad-hoc sign and skip notarization, but must still build, extract, probe, and validate the bundle.
- Use `pnpm run` scripts for repository JavaScript/TypeScript tooling, per `AGENTS.md`.
- Use test-driven development: add each failing test or validation first, observe the expected failure, then implement the smallest passing change.
- Keep the current UI and in-document `Mdow` branding intact; only distribution-facing application identity becomes `Mdow Native`.

---

## Task 1: Make the GPUI binary bundle-aware and independently verifiable

**Files:**

- Modify: `apps/gpui/src/assets.rs`
- Modify: `apps/gpui/src/main.rs`
- Test: `apps/gpui/src/assets.rs` (`#[cfg(test)]` module)
- Test: `apps/gpui/src/main.rs` (`#[cfg(test)]` module)

- [ ] **Step 1: Add failing tests for packaged and development asset discovery**

Add public, side-effect-free asset discovery tests in `apps/gpui/src/assets.rs`. Construct a temporary bundle with the exact release layout and a separate development directory:

```rust
#[test]
fn discovers_assets_next_to_a_bundled_executable() {
    let dir = tempfile::tempdir().unwrap();
    let executable = dir
        .path()
        .join("Mdow Native.app/Contents/MacOS/MdowNative");
    let bundled_assets = dir
        .path()
        .join("Mdow Native.app/Contents/Resources/assets");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::create_dir_all(&bundled_assets).unwrap();
    fs::write(&executable, b"fixture").unwrap();

    assert_eq!(
        discover_asset_root(&executable, dir.path().join("development-assets")).unwrap(),
        bundled_assets.canonicalize().unwrap(),
    );
}

#[test]
fn falls_back_to_development_assets_outside_an_app_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let executable = dir.path().join("target/debug/mdow-gpui");
    let development_assets = dir.path().join("assets");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::create_dir_all(&development_assets).unwrap();
    fs::write(&executable, b"fixture").unwrap();

    assert_eq!(
        discover_asset_root(&executable, &development_assets).unwrap(),
        development_assets.canonicalize().unwrap(),
    );
}
```

Also add cases that reject a malformed bundle, a missing development root, and a bundle `Resources/assets` symlink that resolves outside `Contents/Resources`. The symlink test preserves the existing asset-boundary guarantee at the distribution boundary.

- [ ] **Step 2: Run the focused tests and confirm the expected compile failure**

Run:

```bash
cargo test --manifest-path apps/gpui/Cargo.toml assets::tests::discovers_assets_next_to_a_bundled_executable
```

Expected: compilation fails because `discover_asset_root` does not exist.

- [ ] **Step 3: Implement bundle-aware discovery and required-asset validation**

Add these interfaces to `apps/gpui/src/assets.rs`:

```rust
pub const REQUIRED_ASSETS: &[&str] = &[
    "fonts/InterVariable.ttf",
    "fonts/GeistMono-Variable.ttf",
    "icons/alert-circle.svg",
    "icons/check.svg",
    "icons/chevron-right.svg",
    "icons/copy.svg",
    "icons/expand.svg",
    "icons/file.svg",
    "icons/folder-open.svg",
    "icons/folder.svg",
    "icons/mdow-logo.svg",
    "icons/sidebar.svg",
    "icons/x.svg",
];

pub fn discover_asset_root(
    executable: impl AsRef<Path>,
    development_assets: impl AsRef<Path>,
) -> Result<PathBuf>;

pub fn validate_required_assets(root: impl AsRef<Path>) -> Result<()>;
```

`discover_asset_root` must:

1. Canonicalize the executable path.
2. Recognize only `.../<name>.app/Contents/MacOS/<executable>` as a packaged layout.
3. Resolve `.../Contents/Resources/assets` and require its canonical path to remain under canonical `.../Contents/Resources`.
4. Return the packaged root when inside a bundle; never silently fall back if the bundle is malformed or incomplete.
5. Otherwise canonicalize and return the development root.

`validate_required_assets` must identify every missing required file in one readable error. Keep the existing `MdowAssets::resolve` traversal and symlink checks unchanged.

- [ ] **Step 4: Add a failing test for the package probe CLI**

In `apps/gpui/src/main.rs`, add argument parsing tests for a headless validation flag:

```rust
#[test]
fn verify_assets_flag_is_not_treated_as_a_document_path() {
    let args = launch_args(["MdowNative", "--verify-assets"]);
    assert!(args.verify_assets);
    assert_eq!(args.document_path, None);
}
```

Also update existing menu tests to expect `Mdow Native`, `Quit Mdow Native`, and the default window title `Mdow Native`.

Run:

```bash
cargo test --manifest-path apps/gpui/Cargo.toml main
```

Expected: tests fail because the parser has no `verify_assets` field and the product strings still say `Mdow`.

- [ ] **Step 5: Wire discovery, verification, fonts, and product identity into startup**

Replace the compile-time-only root:

```rust
PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
```

with discovery based on:

```rust
let asset_root = discover_asset_root(
    std::env::current_exe().context("locating Mdow Native executable")?,
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
)?;
validate_required_assets(&asset_root)?;
```

Parse `--verify-assets` before creating `gpui::Application`. When present, resolve and validate assets, print the canonical asset root, and exit successfully without opening a window. Keep the first non-flag argument behavior for markdown files. Load fonts through the discovered root and make startup failures include the missing relative asset name.

Update only the application-level identity strings:

```rust
Menu::new("Mdow Native")
MenuItem::action("Quit Mdow Native", Quit)
WindowOptions { titlebar: Some(TitlebarOptions { title: Some("Mdow Native".into()), .. }) }
```

Do not rename the in-product wordmark or markdown-view UI.

- [ ] **Step 6: Run the GPUI unit gate**

Run:

```bash
cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check
cargo test --manifest-path apps/gpui/Cargo.toml
cargo clippy --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings
```

Expected: all tests pass, formatting is clean, and Clippy emits no warnings.

- [ ] **Step 7: Commit the runtime changes**

```bash
git add apps/gpui/src/assets.rs apps/gpui/src/main.rs
git commit -m "feat(gpui): support packaged native assets"
```

---

## Task 2: Build and validate the `Mdow Native.app` artifact locally

**Files:**

- Create: `script/package_gpui_mac_beta.sh`
- Create: `script/test_package_gpui_mac_beta.zsh`
- Modify: `script/native_mac_bundle.sh`
- Modify: `package.json`
- Delete: `script/package_native_mac_beta.sh`

- [ ] **Step 1: Add a failing bundle-helper test for distinct display and executable names**

Extend `script/test_package_gpui_mac_beta.zsh` to source `script/native_mac_bundle.sh`, write a plist, and assert:

```zsh
write_native_mac_info_plist \
  "$plist" \
  "MdowNative" \
  "Mdow Native" \
  "com.zain.mdow.gpui" \
  "14.0" \
  "1.2.3" \
  "456"

[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$plist")" == "MdowNative" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleDisplayName' "$plist")" == "Mdow Native" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist")" == "com.zain.mdow.gpui" ]]
```

Run:

```bash
zsh script/test_package_gpui_mac_beta.zsh
```

Expected: the helper test fails because the current positional interface hard-codes `Mdow` and interprets the arguments differently.

- [ ] **Step 2: Generalize the shared bundle helper without breaking its callers**

Change the helper contract to:

```bash
write_native_mac_info_plist \
  plist_path executable_name display_name bundle_id minimum_macos version build_number
```

Write `CFBundleExecutable` and `CFBundleName` from `executable_name`, `CFBundleDisplayName` from `display_name`, and the icon name from `executable_name`. Update any surviving callers at the same time. Keep the markdown document declarations and `LSHandlerRank=Alternate` so installing the beta does not forcibly take over file associations.

- [ ] **Step 3: Add failing packaging-contract tests**

Make `script/test_package_gpui_mac_beta.zsh` execute the packager with an isolated output directory. Commands that contact Apple or inspect a Mach-O file must be injectable through environment variables (`CARGO`, `CODESIGN`, `DITTO`, `LIPO`, `XCRUN`, and `SPCTL`) so the test can use deterministic fakes while still exercising the real bundle assembly logic.

Assert all of the following:

- `Mdow Native.app/Contents/MacOS/MdowNative` exists and is executable.
- `Info.plist` has executable `MdowNative`, display name `Mdow Native`, bundle ID `com.zain.mdow.gpui`, and minimum macOS `14.0`.
- `Resources/MdowNative.icns`, `Resources/assets/fonts`, and `Resources/assets/icons` exist.
- Both required ZIP names are created for `VERSION=1.2.3` and `ARCH=arm64`.
- A fake `x86_64` result from `lipo -archs` makes the script fail before signing.
- `CI=true` without a real signing identity fails.
- Local mode without credentials uses ad-hoc signing.
- Release mode submits to notarytool, staples the app, extracts the finished ZIP, invokes `spctl`, and runs `MdowNative --verify-assets` from outside the repository.

Run:

```bash
zsh script/test_package_gpui_mac_beta.zsh
```

Expected: the test fails because `script/package_gpui_mac_beta.sh` does not exist.

- [ ] **Step 4: Implement the GPUI packager**

Create `script/package_gpui_mac_beta.sh` with these fixed values:

```bash
APP_BUNDLE_NAME="Mdow Native"
EXECUTABLE_NAME="MdowNative"
BUNDLE_ID="com.zain.mdow.gpui"
MIN_SYSTEM_VERSION="14.0"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/gpui-mac}"
VERSIONED_ZIP="$DIST_DIR/MdowNative-$VERSION-arm64-mac-beta.zip"
ALIAS_ZIP="$DIST_DIR/MdowNative-mac-beta.zip"
```

The script must perform this ordered pipeline:

1. Require Darwin and an arm64 runner unless a test-only `ARCH` override is set.
2. Resolve `VERSION` from the environment, falling back locally to `apps/gpui/Cargo.toml`; use `GITHUB_RUN_NUMBER` as the build number.
3. Run `cargo build --release --locked --manifest-path apps/gpui/Cargo.toml`.
4. Use `lipo -archs` to require an arm64-only binary.
5. Assemble `Mdow Native.app`, copy `apps/desktop/resources/icon.icns`, and recursively copy `apps/gpui/assets` to `Contents/Resources/assets`.
6. Write the plist through the shared helper.
7. Find `NATIVE_MAC_CODESIGN_IDENTITY`, then `CSC_NAME`, then a Developer ID Application identity in `KEYCHAIN_PATH`. CI must fail if none exists; local builds use `-`.
8. Sign with hardened runtime and verify with `codesign --verify --deep --strict`.
9. When using Developer ID, require all three notary credentials in CI, submit a temporary ZIP with `xcrun notarytool --wait`, staple, and validate.
10. Create the versioned ZIP and copy it to the stable alias.
11. Extract the versioned ZIP into a fresh temporary directory outside the repository, verify with `codesign`, require arm64 with `lipo`, run `spctl` for notarized packages, and execute `MdowNative --verify-assets` with the current directory outside the repository.

Use explicit validated paths for cleanup; never recursively delete an unresolved environment variable. Retain all temporary paths in a trap-owned array.

- [ ] **Step 5: Replace the old package entry point**

In root `package.json`, replace the Swift package script with:

```json
"package:gpui-mac-beta": "bash script/package_gpui_mac_beta.sh",
"test:package:gpui-mac-beta": "zsh script/test_package_gpui_mac_beta.zsh"
```

Delete the now-unused `script/package_native_mac_beta.sh`. Keep `apps/native-mac/` source untouched in this task; the requirement is to replace its distributed beta, not perform an unrelated source-tree deletion.

- [ ] **Step 6: Run shell tests and build a real local artifact**

Run:

```bash
pnpm run test:package:gpui-mac-beta
VERSION=0.1.0-local pnpm run package:gpui-mac-beta
```

Expected: the shell contract tests pass. The local build reports ad-hoc signing when Developer ID credentials are absent and creates:

```text
dist/gpui-mac/MdowNative-0.1.0-local-arm64-mac-beta.zip
dist/gpui-mac/MdowNative-mac-beta.zip
```

- [ ] **Step 7: Independently inspect the real artifact**

Run:

```bash
probe_dir="$(mktemp -d)"
ditto -x -k dist/gpui-mac/MdowNative-0.1.0-local-arm64-mac-beta.zip "$probe_dir"
/usr/libexec/PlistBuddy -c 'Print :CFBundleDisplayName' "$probe_dir/Mdow Native.app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$probe_dir/Mdow Native.app/Contents/Info.plist"
lipo -archs "$probe_dir/Mdow Native.app/Contents/MacOS/MdowNative"
codesign --verify --deep --strict --verbose=2 "$probe_dir/Mdow Native.app"
(cd /tmp && "$probe_dir/Mdow Native.app/Contents/MacOS/MdowNative" --verify-assets)
```

Expected: display name `Mdow Native`, bundle ID `com.zain.mdow.gpui`, architecture `arm64`, valid code signature, and a canonical path below the extracted app's `Contents/Resources/assets`.

- [ ] **Step 8: Commit the packaging changes**

```bash
git add package.json script/native_mac_bundle.sh script/package_gpui_mac_beta.sh script/test_package_gpui_mac_beta.zsh script/package_native_mac_beta.sh
git commit -m "build(gpui): package native mac beta"
```

---

## Task 3: Add GPUI pull-request and push CI

**Files:**

- Create: `.github/workflows/gpui.yml`
- Create: `script/test_gpui_workflows.zsh`
- Modify: `package.json`

- [ ] **Step 1: Write failing static workflow contract tests**

Create `script/test_gpui_workflows.zsh` and make it assert the dedicated workflow has:

- both `pull_request` and `push` triggers;
- path filters for `apps/gpui/**`, `script/package_gpui_mac_beta.sh`, `script/native_mac_bundle.sh`, `script/test_package_gpui_mac_beta.zsh`, and `.github/workflows/gpui.yml`;
- `runs-on: macos-15`;
- Rust `1.93.0` with `rustfmt` and `clippy`;
- Metal toolchain verification or installation;
- locked fmt, test, Clippy, release build, package-test, and local-package commands;
- an uploaded ZIP artifact with `if-no-files-found: error`.

Prefer exact `grep -F` assertions for required contract lines and a small `ruby -e` YAML parse using macOS system Ruby to catch syntax errors. Quote the YAML key `on` in Ruby because YAML 1.1 can otherwise coerce it to a boolean.

Add a root script:

```json
"test:gpui-workflows": "zsh script/test_gpui_workflows.zsh"
```

Run:

```bash
pnpm run test:gpui-workflows
```

Expected: failure because `.github/workflows/gpui.yml` does not exist.

- [ ] **Step 2: Create the dedicated workflow**

Create `.github/workflows/gpui.yml` with one `verify` job on `macos-15`. Use `actions/checkout@v5`, install pinned Rust 1.93 with rustfmt and Clippy, and cache Cargo registry/git data plus `apps/gpui/target` using `actions/cache@v4` keyed by `Cargo.lock`.

The job order must be:

```yaml
- name: Ensure Metal toolchain is available
  run: xcrun metal -v || xcodebuild -downloadComponent MetalToolchain
- run: cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check
- run: cargo test --locked --manifest-path apps/gpui/Cargo.toml
- run: cargo clippy --locked --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings
- run: cargo build --release --locked --manifest-path apps/gpui/Cargo.toml
- run: pnpm run test:package:gpui-mac-beta
- run: VERSION=0.0.0-ci pnpm run package:gpui-mac-beta
```

Install pnpm/Node only if needed to invoke the root script; alternatively call the Bash/Zsh scripts directly and keep the workflow Rust-only. Do not use `CI=true` for the PR package smoke test because local ad-hoc signing is intentional there.

Upload `dist/gpui-mac/*.zip` as `mdow-native-ci-arm64` so reviewers can manually inspect PR artifacts.

- [ ] **Step 3: Run workflow tests and local equivalents**

Run:

```bash
pnpm run test:gpui-workflows
cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check
cargo test --locked --manifest-path apps/gpui/Cargo.toml
cargo clippy --locked --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings
VERSION=0.0.0-ci pnpm run package:gpui-mac-beta
```

Expected: the workflow contract passes and all commands mirrored by CI pass locally.

- [ ] **Step 4: Commit the CI workflow**

```bash
git add .github/workflows/gpui.yml script/test_gpui_workflows.zsh package.json
git commit -m "ci(gpui): verify native beta on changes"
```

---

## Task 4: Replace the SwiftUI tag-release job with the GPUI beta

**Files:**

- Modify: `.github/workflows/release.yml`
- Modify: `script/test_gpui_workflows.zsh`

- [ ] **Step 1: Add failing release-workflow contract tests**

Extend `script/test_gpui_workflows.zsh` to assert:

- the release job ID is `gpui-mac-beta`, not `native-mac-beta`;
- it runs on `macos-15` and pins Rust 1.93 with rustfmt/Clippy;
- it runs the complete GPUI fmt/test/Clippy gate before packaging;
- it derives `VERSION` from `${GITHUB_REF_NAME#v}` and exports `CI=true`;
- it imports the Developer ID certificate into an explicit temporary keychain;
- it passes `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, and `APPLE_TEAM_ID`;
- it uploads `dist/gpui-mac/MdowNative-*.zip` to both workflow artifacts and the GitHub release;
- the final `publish` job needs both `release` and `gpui-mac-beta`;
- the Electron matrix and `pnpm run --filter desktop publish` remain present;
- no release command references Swift, `package:native-mac-beta`, or `dist/native-mac`.

Run:

```bash
pnpm run test:gpui-workflows
```

Expected: failure on the current `native-mac-beta` SwiftUI job.

- [ ] **Step 2: Convert the tag-release job**

Rename the job to `gpui-mac-beta` and keep `runs-on: macos-15`. Replace its Node/pnpm install and Swift package command with the same pinned Rust, Metal, cache, fmt, test, and Clippy steps as the dedicated GPUI workflow.

After certificate import, package with:

```yaml
- name: Build signed and notarized Mdow Native beta
  shell: bash
  env:
    CI: 'true'
    VERSION: ${{ github.ref_name }}
    GITHUB_RUN_NUMBER: ${{ github.run_number }}
    NATIVE_MAC_CODESIGN_IDENTITY: ${{ secrets.CSC_NAME }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_APP_SPECIFIC_PASSWORD: ${{ secrets.APPLE_APP_SPECIFIC_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
  run: |
    VERSION="${VERSION#v}" bash script/package_gpui_mac_beta.sh
```

If `CSC_NAME` is not configured, omit that environment mapping and let the package script discover the imported Developer ID identity from `KEYCHAIN_PATH`. Do not assume the p12 display name is available as a secret.

Upload:

```yaml
path: dist/gpui-mac/*.zip
```

and:

```bash
gh release upload "$TAG" dist/gpui-mac/MdowNative-*.zip --clobber
```

Keep the existing draft-release creation and keychain cleanup. Update `publish.needs` to `gpui-mac-beta`. Do not change the Electron release matrix or publishing step.

- [ ] **Step 3: Run the workflow contract and inspect the diff**

Run:

```bash
pnpm run test:gpui-workflows
git diff --check
git diff -- .github/workflows/release.yml
```

Expected: tests pass; the diff shows only the native-beta job changed while Electron remains intact.

- [ ] **Step 4: Commit the release conversion**

```bash
git add .github/workflows/release.yml script/test_gpui_workflows.zsh
git commit -m "ci(release): publish gpui mac beta"
```

---

## Task 5: Expose the GPUI beta explicitly on the download site

**Files:**

- Modify: `apps/web/src/lib/github-releases.ts`
- Modify: `apps/web/src/lib/github-releases.test.ts`
- Modify: `apps/web/src/lib/download-links.ts`
- Modify: `apps/web/src/lib/download-links.test.ts`
- Create: `apps/web/src/components/native-download-section.tsx`
- Create: `apps/web/src/components/__tests__/native-download-section.test.tsx`
- Modify: `apps/web/src/routes/download.tsx`
- Modify: `apps/web/content/docs/installation.md`
- Modify: `apps/web/content/changelog.md`
- Regenerate: `apps/web/public/docs/installation.md`
- Regenerate: generated changelog/RSS output produced by the existing web scripts

- [ ] **Step 1: Add failing release-parser tests for the GPUI artifact contract**

Rename the model field from the SwiftUI-era generic name to:

```ts
mac: {
  dmg: ReleaseAsset[]
  zip: ReleaseAsset[]
  gpuiBeta: ReleaseAsset | null
}
```

Before implementation, update `apps/web/src/lib/github-releases.test.ts` with these cases:

```ts
it('classifies the GPUI alias separately from Electron mac archives', () => {
  const parsed = parseRelease(releaseWithAssets([
    asset('Mdow-2.0.0-arm64-mac.zip', 'electron'),
    asset('MdowNative-mac-beta.zip', 'gpui'),
  ]))!

  expect(parsed.assets.mac.zip).toEqual([{ arch: 'arm64', url: 'electron' }])
  expect(parsed.assets.mac.gpuiBeta).toEqual({ url: 'gpui' })
})

it('prefers the stable GPUI alias when both beta names are present', () => {
  const parsed = parseRelease(releaseWithAssets([
    asset('MdowNative-2.0.0-arm64-mac-beta.zip', 'versioned'),
    asset('MdowNative-mac-beta.zip', 'alias'),
  ]))!

  expect(parsed.assets.mac.gpuiBeta?.url).toBe('alias')
})
```

Also test the reverse asset order so alias preference is deterministic.

Run:

```bash
pnpm run --filter web test -- github-releases.test.ts
```

Expected: type/test failure because `gpuiBeta` does not exist.

- [ ] **Step 2: Implement explicit GPUI release parsing and links**

Rename:

```ts
nativeBeta -> gpuiBeta
NATIVE_MAC_BETA_DOWNLOAD_URL -> GPUI_MAC_BETA_DOWNLOAD_URL
nativeMacBetaDownloadUrl -> gpuiMacBetaDownloadUrl
```

Recognize only the two supported shapes, case-insensitively:

```ts
const GPUI_ALIAS = 'mdownative-mac-beta.zip'
const GPUI_VERSIONED = /^mdownative-[^-]+-arm64-mac-beta\.zip$/
```

If semantic versions can contain a prerelease hyphen, use a suffix/prefix check instead of the single-segment regex:

```ts
normalized.startsWith('mdownative-') &&
normalized.endsWith('-arm64-mac-beta.zip')
```

Always prefer the exact alias over the versioned file regardless of GitHub asset order. Keep the fallback URL exactly:

```ts
https://github.com/ZainW/mdow/releases/latest/download/MdowNative-mac-beta.zip
```

Update `download-links.test.ts` to prove parsed-release preference and fallback behavior.

- [ ] **Step 3: Add a failing component test for the full beta message**

Move the beta section out of the route into `apps/web/src/components/native-download-section.tsx` so it can be rendered without TanStack route context. Add a Testing Library test that asserts:

```ts
expect(screen.getByRole('heading', { name: 'Mdow Native' })).toBeInTheDocument()
expect(screen.getByText('A GPUI beta for Apple Silicon Macs running macOS 14 or newer.')).toBeInTheDocument()
expect(screen.getByText('Runs alongside the regular Mdow app.')).toBeInTheDocument()
expect(screen.getByRole('link', { name: 'Download Mdow Native (.zip)' })).toHaveAttribute(
  'href',
  betaUrl,
)
```

Run:

```bash
pnpm run --filter web test -- native-download-section.test.tsx
```

Expected: failure because the component does not exist.

- [ ] **Step 4: Implement and integrate the secondary download card**

The new component must retain the existing spacing, typography, border, and `DownloadCard` patterns while using the approved copy:

```tsx
<p className="text-xs font-semibold uppercase tracking-wide text-primary">Beta</p>
<h2 className="mt-2 text-2xl font-semibold tracking-tight">Mdow Native</h2>
<p className="mt-2 text-sm text-muted-foreground">
  A GPUI beta for Apple Silicon Macs running macOS 14 or newer.
</p>
<p className="mt-1 text-sm text-muted-foreground">
  Runs alongside the regular Mdow app.
</p>
```

The card label is `Mdow Native — GPUI beta`; its only format label is `Download Mdow Native (.zip)`. The Electron platform grid, OS-recommended primary button, unavailable-download fallback, and release metadata stay unchanged.

- [ ] **Step 5: Update installation and changelog content**

In `apps/web/content/docs/installation.md`, add a `Mdow Native (GPUI beta)` subsection that states:

- Apple Silicon and macOS 14+ only;
- it installs as `Mdow Native.app` and runs alongside regular Mdow;
- it is signed/notarized on tagged releases;
- upgrades are manual by downloading the latest beta ZIP and replacing the old app;
- it has no in-app updater yet.

Remove SwiftUI-beta language. Add a concise changelog entry announcing the GPUI beta distribution without implying Electron has been replaced.

Run the existing web generation scripts discovered in `apps/web/package.json` through `pnpm run --filter web ...`; do not hand-edit generated public/RSS copies when a script owns them.

- [ ] **Step 6: Run web and repository gates**

Run:

```bash
pnpm run --filter web test
pnpm run --filter web typecheck
pnpm run --filter web build
pnpm run fmt:check
pnpm run lint
git diff --check
```

Expected: web tests, typecheck, production build, formatting, lint, and whitespace checks all pass.

- [ ] **Step 7: Commit the web and documentation changes**

```bash
git add apps/web
git commit -m "feat(web): add Mdow Native beta download"
```

---

## Completion Gate: Verify the exact release candidate path

- [ ] Run every local gate from a clean working tree:

```bash
pnpm run test:gpui-workflows
pnpm run test:package:gpui-mac-beta
cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check
cargo test --locked --manifest-path apps/gpui/Cargo.toml
cargo clippy --locked --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings
cargo build --release --locked --manifest-path apps/gpui/Cargo.toml
VERSION=0.1.0-rc pnpm run package:gpui-mac-beta
pnpm run --filter web test
pnpm run --filter web typecheck
pnpm run --filter web build
pnpm run fmt:check
pnpm run lint
git diff --check
git status --short
```

- [ ] Extract `MdowNative-0.1.0-rc-arm64-mac-beta.zip` into a fresh temporary directory outside the repository and repeat plist, `lipo`, `codesign`, and `--verify-assets` checks. From `/tmp`, start the extracted `Contents/MacOS/MdowNative` directly with the absolute path to `apps/gpui/tests/fixtures/showcase.md`, retain the exact child PID, confirm the showcase renders, then terminate and reap only that PID. Confirm the process/application menu identity is `Mdow Native` while the existing Electron app remains open.

- [ ] Inspect `.github/workflows/release.yml` one final time to confirm the tag publish waits for both the unchanged Electron release matrix and the new GPUI job.

- [ ] Confirm the website parser test covers both GitHub asset orderings and that the rendered link resolves to the alias asset when present.

- [ ] Run `git log --oneline -7` and confirm each task is represented by an intentional commit and the worktree is clean.

- [ ] Apply `superpowers:verification-before-completion` before claiming success. Report the commands and observed results, including whether the local package was ad-hoc signed; do not claim notarization was locally proven unless the Apple credentials were actually available and `notarytool`, stapler, and `spctl` all succeeded.
