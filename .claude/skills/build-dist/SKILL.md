---
name: build-dist
description: Build distributable Electron app packages. Use when the user wants to create a release build, .dmg, installer, or packaged app.
disable-model-invocation: true
---

Build distributable packages for the desktop app. Pass a platform flag via `$ARGUMENTS` (e.g., `/build-dist --mac`, `/build-dist --win`, `/build-dist --linux`). Defaults to `--mac` if no argument given.

```bash
pnpm run --filter desktop build:dist -- ${ARGUMENTS:---mac}
```

Before building, run `/verify` to ensure the codebase is clean. Report the output path from `out/` when done.
