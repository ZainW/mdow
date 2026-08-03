const SITE_URL = 'https://mdow.app'

function escapeXml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function existingPublicationDates(rss) {
  const dates = new Map()
  const items = rss.matchAll(
    /<guid isPermaLink="false">([^<]+)<\/guid>\s*<pubDate>([^<]+)<\/pubDate>/g,
  )

  for (const [, version, pubDate] of items) {
    dates.set(version, pubDate)
  }

  return dates
}

function truncateAtWordBoundary(content, maxLength) {
  if (content.length <= maxLength) return content

  const candidate = content.slice(0, maxLength)
  const previousBoundary = candidate.search(/\s+\S*$/)
  if (previousBoundary >= 0) return candidate.slice(0, previousBoundary).trimEnd()

  const nextBoundary = content.slice(maxLength).search(/\s/)
  return nextBoundary >= 0 ? content.slice(0, maxLength + nextBoundary) : content
}

export function generateRss(rawChangelog, existingRss = '', now = new Date()) {
  const body = rawChangelog.replace(/^---[\s\S]*?---\n/, '').replace(/^\s*# .*\n+/, '')
  const sections = body.split(/^## /m).filter(Boolean)
  const previousDates = existingPublicationDates(existingRss)
  const newVersionDate = now.toUTCString()

  const items = sections
    .map((section) => {
      const [versionLine, ...rest] = section.split('\n')
      const version = versionLine.trim()
      const content = rest.join('\n').trim()
      const pubDate = previousDates.get(version) ?? newVersionDate
      const anchor = version.replace(/\./g, '-')
      return `
    <item>
      <title>${escapeXml(version)}</title>
      <link>${SITE_URL}/changelog#${escapeXml(anchor)}</link>
      <guid isPermaLink="false">${escapeXml(version)}</guid>
      <pubDate>${pubDate}</pubDate>
      <description>${escapeXml(truncateAtWordBoundary(content, 500))}</description>
    </item>`
    })
    .join('')

  return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Mdow Changelog</title>
    <link>${SITE_URL}/changelog</link>
    <description>What's new in Mdow</description>
    <language>en-us</language>${items}
  </channel>
</rss>`
}
