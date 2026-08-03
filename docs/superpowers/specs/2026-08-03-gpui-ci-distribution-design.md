# GPUI CI and Beta Distribution Design

**Date:** 2026-08-03  
**Status:** Approved design  
**Scope:** Apple Silicon CI, signed/notarized GPUI beta packaging, and website download integration

## Goal

Ship the Rust + GPUI reader as a downloadable Apple Silicon macOS beta. The build must run through
continuous integration, tagged releases must publish a signed and notarized standalone app, and the
website must replace the existing SwiftUI beta download with this GPUI version.

The GPUI beta remains secondary to the stable Electron app. Users must be able to install and run
both at the same time.

## Product Identity

The distributed application is named **Mdow Native**.

- App bundle: `Mdow Native.app`
- Executable: `MdowNative`
- Bundle identifier: `com.zain.mdow.gpui`
- Website name: `Mdow Native`
- Website qualifier: `GPUI beta for Apple Silicon`
- Minimum system: macOS 14
- Architecture: ARM64 only for the initial beta

The distinct bundle identifier, bundle name, executable name, application menu, Quit item, and
window title allow Mdow Native to run beside Electron's `Mdow.app`. The in-product Mdow logo and
established visual language remain unchanged.

## Release Artifacts and Versioning

Tagged releases publish two equivalent zip assets:

- `MdowNative-<version>-arm64-mac-beta.zip`
- `MdowNative-mac-beta.zip`

The version comes from the Git tag after removing the leading `v`; it does not depend on the
Electron package version. `CFBundleShortVersionString` uses that release version and
`CFBundleVersion` uses the GitHub Actions run number.

The versioned artifact is useful for release history and auditing. The stable alias gives the
website and external links a durable latest-download URL.

## Continuous Integration

A dedicated GPUI workflow runs on every pull request and push when GPUI, its packaging scripts, or
the workflow itself changes. It uses the pinned `macos-15` GitHub-hosted ARM64 image rather than a
moving `macos-latest` label. GitHub's current runner catalog maps `macos-15` to its Apple Silicon
image: <https://github.com/actions/runner-images>.

The workflow:

1. Checks out the repository.
2. Installs the pinned Rust 1.93 toolchain with `rustfmt` and `clippy`.
3. Selects full Xcode and verifies the Metal compiler and linker.
4. Downloads the Metal Toolchain when the image does not already contain a usable copy.
5. Restores and saves Cargo caches using the GPUI lockfile as part of the cache key.
6. Runs `cargo fmt --check`.
7. Runs the complete GPUI test suite.
8. Runs Clippy for all targets with warnings denied.
9. Builds the release binary with the lockfile enforced.
10. Runs unsigned/ad-hoc packaging checks that validate the standalone bundle layout and runtime
    asset discovery without requiring repository secrets.

CI does not build Intel artifacts, Electron packages, or deploy the website.

## Tagged Release Workflow

The existing `native-mac-beta` SwiftUI job is replaced by a GPUI release job on the Apple Silicon
`macos-15` runner. The stable Electron matrix remains unchanged and continues to produce the
recommended downloads.

The GPUI release job:

1. Performs the same format, test, Clippy, release-build, and package checks used by continuous
   integration.
2. Imports the existing Developer ID certificate into a temporary keychain.
3. Constructs `Mdow Native.app` with the release binary, app icon, fonts, icons, and plist.
4. Signs the bundle with hardened runtime and a timestamp.
5. Verifies the signature.
6. Submits the bundle to Apple's notary service and waits for success.
7. Staples and validates the notarization ticket.
8. Produces the versioned zip and stable alias.
9. Extracts the final zip and revalidates its architecture, plist, signature, ticket, executable,
   and required assets.
10. Uploads both assets to the draft GitHub release.
11. Deletes the temporary signing keychain even when an earlier step fails.

The final release-publishing job continues to depend on both the Electron release matrix and the
GPUI release job. A GPUI build, signing, notarization, packaging, or validation failure therefore
prevents publication of an incomplete tagged release.

## Standalone App Packaging

A new GPUI packaging script reuses the repository's existing native bundle helpers where the
bundle format is shared, but it does not depend on the Swift package or SwiftUI beta script.

The final bundle contains:

```text
Mdow Native.app/
  Contents/
    Info.plist
    MacOS/
      MdowNative
    Resources/
      MdowNative.icns
      assets/
        fonts/
        icons/
```

The script builds with `cargo build --release --locked`, confirms the executable is ARM64, copies
only the required GPUI assets, writes the versioned plist, signs/notarizes when credentials are
provided, and supports ad-hoc signing for local or untrusted CI package validation.

In trusted release CI, missing Developer ID or notarization credentials are fatal. Local packaging
may use ad-hoc signing and clearly reports that notarization was skipped.

## Runtime Asset Discovery

The current binary uses `CARGO_MANIFEST_DIR` to locate assets, embedding the build machine's
absolute checkout path. That is valid for local development but invalid after distribution.

Asset discovery becomes an explicit, testable resolver:

1. When the executable is inside an app bundle, use
   `../Resources/assets` relative to `Contents/MacOS/MdowNative`.
2. For local Cargo execution, fall back to `<CARGO_MANIFEST_DIR>/assets`.
3. Canonicalize the selected root and retain the existing protection against absolute paths,
   parent traversal, and escaping symlinks.
4. Produce a readable startup failure when required fonts or assets are missing instead of using a
   CI checkout path or failing later during rendering.

Tests cover packaged and development layouts, missing resources, and escape rejection. Package
validation launches or otherwise probes the extracted bundle from outside the repository so a
checkout-relative asset dependency cannot pass unnoticed.

## Website Download Experience

Electron remains the primary and recommended download on the homepage and `/download`. The primary
homepage CTA and platform-selection behavior do not change.

The existing SwiftUI beta section on `/download` is replaced in place with:

- Heading: `Mdow Native`
- Badge: `Beta`
- Description: `A GPUI beta for Apple Silicon Macs running macOS 14 or newer.`
- Compatibility note: `Runs alongside the regular Mdow app.`
- Button: `Download Mdow Native (.zip)`

The release parser identifies the GPUI beta assets explicitly and exposes them under a GPUI/native
field rather than SwiftUI-oriented naming. It recognizes both the versioned artifact and the stable
alias, prefers the release asset returned by GitHub, and falls back to:

`https://github.com/ZainW/mdow/releases/latest/download/MdowNative-mac-beta.zip`

The installation documentation is updated to describe the GPUI beta, Apple Silicon and macOS 14
requirements, side-by-side installation, lack of Electron auto-update behavior, and manual upgrade
through the download page. Changelog and generated public Markdown copies are updated through the
existing content-generation scripts.

## Error Handling

- Continuous CI fails when formatting, tests, Clippy, release compilation, or package validation
  fails.
- Tagged release CI additionally fails when signing, notarization, stapling, extraction, or final
  validation fails.
- Artifacts are uploaded only after final validation.
- The draft release is published only after Electron and GPUI jobs both succeed.
- Website release parsing remains resilient to unrelated assets and missing GitHub API data.
- The GPUI beta card uses the durable latest-download alias when a parsed asset is unavailable.

## Testing and Acceptance

### Rust and packaging

- Existing GPUI tests remain green.
- Unit tests cover packaged/development asset-root selection.
- Shell tests cover version parsing, architecture rejection, bundle identity, plist fields,
  required resources, stable/versioned filenames, signing modes, and failure propagation.
- The extracted artifact runs outside the repository and opens the deterministic showcase file.

### Website

- Release-parser tests distinguish Electron zips from both GPUI artifact names.
- Download-link tests prefer the parsed GPUI asset and verify the stable fallback.
- Download-page tests verify the `Mdow Native` label, Apple Silicon/macOS 14 copy, side-by-side note,
  and final href.
- Existing Electron primary-download tests remain unchanged and green.

### Release acceptance

- CI proves the GPUI app on every relevant push and pull request.
- A tagged release contains both GPUI zip names and the existing Electron artifacts.
- The downloaded app reports `Mdow Native`, uses `com.zain.mdow.gpui`, contains ARM64 code, passes
  Gatekeeper validation, loads fonts/icons without the repository, and runs beside Electron Mdow.
- The public `/download` page exposes the GPUI beta without replacing the Electron primary CTA.

## Non-Goals

- Intel or universal GPUI builds.
- Replacing Electron as the recommended download.
- GPUI auto-update support.
- App Store distribution.
- Windows or Linux GPUI builds.
- Website deployment as part of the release workflow.
- Restoring or maintaining the replaced SwiftUI beta artifact.

