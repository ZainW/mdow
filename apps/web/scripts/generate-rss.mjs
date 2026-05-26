// Generates public/changelog/rss.xml from content/changelog.md
// Run: node apps/web/scripts/generate-rss.mjs

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')
const changelogPath = join(root, 'content/changelog.md')
const outDir = join(root, 'public/changelog')
const outPath = join(outDir, 'rss.xml')

const SITE_URL = 'https://mdow.app'

function escapeXml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

const raw = readFileSync(changelogPath, 'utf8')
const body = raw.replace(/^---[\s\S]*?---\n/, '')
const sections = body.split(/^## /m).filter(Boolean)

const items = sections
  .map((section) => {
    const [versionLine, ...rest] = section.split('\n')
    const version = versionLine.trim()
    const content = rest.join('\n').trim()
    const pubDate =
      version === 'v1.0.5' ? 'Mon, 26 May 2026 00:00:00 GMT' : 'Mon, 01 Jan 2026 00:00:00 GMT'
    const anchor = version.replace(/\./g, '-')
    return `
    <item>
      <title>${escapeXml(version)}</title>
      <link>${SITE_URL}/changelog#${escapeXml(anchor)}</link>
      <guid isPermaLink="false">${escapeXml(version)}</guid>
      <pubDate>${pubDate}</pubDate>
      <description>${escapeXml(content.slice(0, 500))}</description>
    </item>`
  })
  .join('')

const rss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Mdow Changelog</title>
    <link>${SITE_URL}/changelog</link>
    <description>What's new in Mdow</description>
    <language>en-us</language>${items}
  </channel>
</rss>`

mkdirSync(outDir, { recursive: true })
writeFileSync(outPath, rss)
console.log(`RSS feed saved to ${outPath}`)
