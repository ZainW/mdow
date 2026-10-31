---
name: release
description: Cut a full Mdow release. Bumps the app version, updates the website changelog and RSS feed, commits on main, tags vX.Y.Z, and follows GitHub Release CI plus the Cloudflare site deploy. Use when the user asks to release, ship, cut a version, bump the version, or publish.
---

# Release

Do this on a clean `main` that matches `origin/main`. Do not skip changelog/RSS. v1.6.1 shipped without them and needed a follow-up commit.

## 1. Version

Bump `version` in the root `package.json`. The Mac app (`apps/macos/scripts/bundle.sh`) reads it, and the release workflow fails if it does not match the tag. Leave `apps/web/package.json` alone.

Patch for fixes and small additions. Minor for user-facing features. Match recent history (`1.5.1` command palette, `1.4.1` native beta).

## 2. Changelog and static site files

Edit `apps/web/content/changelog.md`:

- Add `## vX.Y.Z` at the top, below the frontmatter and `# Changelog`
- Put `Latest release.` on the new section
- Remove `Latest release.` from the previous version
- Concise bullets, not paragraphs

Then regenerate RSS, synced public docs, and the sitemap:

```bash
pnpm run --filter web generate:static
```

Commit the updated `apps/web/public/changelog/rss.xml` (and any synced `apps/web/public/docs/*.md`). Existing RSS `pubDate` values stay; only the new version gets today's date.

## 3. Verify

```bash
pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test
```

Fix failures before tagging.

## 4. Commit, push, tag

Commit on `main` (this is how prior releases were cut):

```text
chore: bump to vX.Y.Z
```

Push `main`, then create a lightweight tag on that commit and push it. The tag name must be `v` plus the root version (`v1.6.2`).

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

Do not force-push. Do not skip hooks.

## 5. GitHub Release notes

Pushing the tag starts `.github/workflows/release.yml`. It creates a draft named after the tag, uploads the Mac app, then publishes with `--latest` once its signed update feed is attached.

As soon as the draft exists, set notes from the changelog bullets (no `Latest release.` line):

```bash
gh release edit vX.Y.Z --title vX.Y.Z --notes "$(cat <<'EOF'
- bullet
EOF
)"
```

If the draft is not there yet, wait and retry. Do not `gh release create` a published release before CI finishes; the workflow publishes the draft only once the Mac app is on it.

## 6. Website

Cloudflare Workers Builds deploys `mdow` from `main` automatically. Do not `wrangler deploy` unless that build fails.

Confirm a new build for the bump commit succeeds, then check:

- https://mdow.wania.app/changelog
- https://mdow.wania.app/changelog/rss.xml

The download page reads GitHub `releases/latest`, so it stays on the previous version until CI publishes.

## 7. Wait until it is actually out

Poll `gh run list --workflow=release.yml --branch vX.Y.Z` until the run succeeds. Then confirm:

```bash
gh release view vX.Y.Z
```

The release must be published (not draft), marked latest, and include the Mac app's `Mdow-X.Y.Z.zip`, `Mdow.zip` and `mdow-appcast.xml`. Mdow for Linux is released separately by `.github/workflows/linux.yml`.

There is no Homebrew tap updater in this repo. Skip it.

## 8. If something is already tagged

Never retag. Fix forward with a new patch version.
