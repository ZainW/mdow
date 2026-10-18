# Mdow Native updates

Mdow Native updates through gpuix's `checkUpdate`, a port of the cargo-packager updater. It reads
`https://api.github.com/repos/ZainW/mdow/releases/latest` and downloads
`mdow-native_<version>_x86_64.AppImage`, which needs a sibling `.sig` from the packager signer. A
missing or bad signature is rejected. Mdow Native is only released for Linux; on the Mac, Mdow for
Mac (`apps/macos`) replaced it.

## One-time key setup

```bash
pnpm --filter native exec packager signer generate --path ~/.mdow-native-updater.key
```

1. Put the **public** key (the `.pub` file's contents) in `public-key.txt`. It is compiled into the app.
2. Add the private key and its password as repo secrets `CARGO_PACKAGER_SIGN_PRIVATE_KEY` and
   `CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD`.

While `public-key.txt` is empty the app reports "Updates are unavailable in this build" and the release
workflow skips signing.
