---
name: verify
description: Run full project verification — typecheck, lint, format check, and tests. Use before marking work complete or committing.
---

Run all verification checks and report results:

```bash
pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test
```

If any check fails, report the specific errors clearly. Do not claim success until all four pass.
