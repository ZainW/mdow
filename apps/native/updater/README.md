# Mdow Native updates

Mdow Native updates through gpuix's `checkUpdate`, a port of the cargo-packager updater. It reads
`https://api.github.com/repos/ZainW/mdow/releases/latest` and picks the asset for this platform:

| Platform | Asset                                              |
| -------- | -------------------------------------------------- |
| macOS    | `Mdow Native.app.tar.gz` (+ `.sig`)                |
| Linux    | `mdow-native_<version>_x86_64.AppImage` (+ `.sig`) |

Every asset needs a sibling `.sig` from the packager signer. A missing or bad signature is rejected.

## One-time key setup

```bash
pnpm --filter native exec packager signer generate --path ~/.mdow-native-updater.key
```

1. Put the **public** key (the `.pub` file's contents) in `public-key.txt`. It is compiled into the app.
2. Add the private key and its password as repo secrets `CARGO_PACKAGER_SIGN_PRIVATE_KEY` and
   `CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD`.

`bun scripts/test-updater.ts` runs a real signed update end to end with a throwaway key.

While `public-key.txt` is empty the app reports "Updates are unavailable in this build" and the release
workflow skips signing.

## Moving Sparkle users over

Builds before the gpuix rewrite update through Sparkle and poll `appcast-native-mac.xml`. The release
workflow still publishes that appcast, pointing at the new notarized zip, so those installs update once
into the gpuix build. After that they use the updater above.
