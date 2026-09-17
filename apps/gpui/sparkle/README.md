# Sparkle keys for Mdow Native

Mac Native (`apps/gpui`, bundle id `com.zain.mdow.gpui`) uses **Sparkle 2** with EdDSA. Electron keeps `electron-updater` and `latest-mac.yml`. Native must never read that YAML feed.

## Feed URL

Shipped `Info.plist` `SUFeedURL`:

```
https://github.com/ZainW/mdow/releases/latest/download/appcast-native-mac.xml
```

Each `v*` release uploads `appcast-native-mac.xml` next to `MdowNative-$VERSION-arm64-mac-beta.zip`. The enclosure URL is the versioned GitHub download, not `latest-mac.yml`.

## One-time key setup

On a Mac, after `bash script/fetch_sparkle.sh`:

```bash
./apps/gpui/vendor/Sparkle/bin/generate_keys
```

That prints the **public** key and stores the private key in your login keychain.

1. Put the public key (one base64 line) in `apps/gpui/sparkle/public-ed-key.txt`. Commit that file. Packaging embeds it as `SUPublicEDKey`.
2. Export the private key and add it as a GitHub Actions repository secret named **`SPARKLE_PRIVATE_ED_KEY`**. Do not commit it.

Export:

```bash
./apps/gpui/vendor/Sparkle/bin/generate_keys -x /tmp/mdow-sparkle-ed-key
# The file contents (base64) are the secret value.
# Add GitHub secret SPARKLE_PRIVATE_ED_KEY, then shred the file.
```

The linked Sparkle framework is always bundled, including local builds without updater configuration. Release packaging requires a public key, and appcast generation requires the private signing key. A missing updater configuration must not silently produce a release without updates.

## Local / debug

- `cargo run` outside a `.app` does not start Sparkle.
- Override the feed with `MDOW_SPARKLE_FEED_URL` (for isolated test hosts; Info.plist ships the production URL).
- Local packages re-sign every embedded Sparkle executable with the same ad-hoc identity.

## CI fetch

`script/fetch_sparkle.sh` downloads Sparkle **2.9.6** (`Sparkle-2.9.6.tar.xz`) from GitHub Releases and verifies `sha256:52bf9e88cdd972fc0c81501377a880e90d47031bd8ca5462488f843e2609e192`. It copies the **root** `Sparkle.framework` (the one with `Headers/Sparkle.h`), not the nested copy inside Sparkle Test App.app. The framework is not vendored in git (`apps/gpui/vendor/` is gitignored). Mac CI `build.rs` fetches it before compiling the ObjC bridge.

## Integration tests

`pnpm run test:native-updater` builds a small host using the production Objective-C bridge. It serves a local feed, creates a disposable Ed25519 key, and verifies current-version checks, rejection of an invalid signature, downloading a valid signed archive, installation, and relaunch into version 2. Test bundles use `com.zain.mdow.updater-test` and temporary directories; production keys and the installed Mdow app are untouched.

The Mac packager also launches the extracted ZIP with `--smoke-test` and requires a live window before accepting the package. Smoke tests use an in-memory session.
