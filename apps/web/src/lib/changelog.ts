import changelogRaw from '../../content/changelog.md?raw'
import rssRaw from '../../public/changelog/rss.xml?raw'
import { versionAnchor } from './release-format'

export interface ChangelogRelease {
  version: string
  /** Matches the anchors used by the RSS feed, e.g. `v1-10-0`. */
  anchor: string
  /** ISO date string, when the RSS feed knows it. */
  date: string | null
  /** Markdown body with editorial notes like "Latest release." removed. */
  markdown: string
  bullets: string[]
}

function publicationDates(rss: string): Map<string, string> {
  const dates = new Map<string, string>()
  const items = rss.matchAll(
    /<guid isPermaLink="false">([^<]+)<\/guid>\s*<pubDate>([^<]+)<\/pubDate>/g,
  )
  for (const [, version, pubDate] of items) {
    const parsed = new Date(pubDate)
    if (!Number.isNaN(parsed.getTime())) dates.set(version, parsed.toISOString())
  }
  return dates
}

export function parseChangelog(raw: string, rss = ''): ChangelogRelease[] {
  const body = raw.replace(/^---[\s\S]*?---\n/, '').replace(/^\s*# .*\n+/, '')
  const dates = publicationDates(rss)

  return body
    .split(/^## /m)
    .filter((section) => section.trim())
    .map((section) => {
      const [versionLine, ...rest] = section.split('\n')
      const version = versionLine.trim()
      const markdown = rest
        .filter((line) => line.trim() !== 'Latest release.')
        .join('\n')
        .trim()
      const bullets = markdown
        .split('\n')
        .filter((line) => line.startsWith('- '))
        .map((line) => line.slice(2).trim())
      return {
        version,
        anchor: versionAnchor(version),
        date: dates.get(version) ?? null,
        markdown,
        bullets,
      }
    })
}

export function getReleases(): ChangelogRelease[] {
  return parseChangelog(changelogRaw, rssRaw)
}

/** A short, plain-text teaser for a release: its first bullet up to the first clause break. */
export function releaseHeadline(release: ChangelogRelease, maxLength = 60): string {
  const first = release.bullets[0] ?? `Mdow ${release.version}`
  const plain = first.replace(/[`*]/g, '').split(/[:;]/)[0].trim()
  if (plain.length <= maxLength) return plain
  const cut = plain.slice(0, maxLength)
  return `${cut.slice(0, cut.lastIndexOf(' '))}…`
}
