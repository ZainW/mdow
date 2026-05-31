---
name: update-web-changelog
description: Update the website changelog and RSS feed when releasing a new version. Use after bumping the desktop app version and before pushing a release tag.
---

When releasing a new version of the desktop app, update the website changelog:

1. **Edit `apps/web/content/changelog.md`** — add a new `## vX.Y.Z` section at the top (below the frontmatter) with a brief summary of changes. Move "Latest release." to the new version.

2. **Regenerate the RSS feed:**
   ```bash
   node apps/web/scripts/generate-rss.mjs
   ```

3. **Verify:**
   ```bash
   pnpm run --filter web typecheck
   ```

The changelog is the single source of truth for the `/changelog` route and the RSS feed. Keep entries concise — bullet points, not paragraphs.
