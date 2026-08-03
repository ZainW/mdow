import { describe, expect, it } from 'vitest'
import { generateRss } from './rss-generator.mjs'

const changelog = `---
title: Changelog
---

# Changelog

## v2.0.0

New release.

## v1.0.0

Original release.
`

const existingRss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <item>
      <guid isPermaLink="false">v1.0.0</guid>
      <pubDate>Mon, 01 Jan 2024 00:00:00 GMT</pubDate>
    </item>
  </channel>
</rss>`

describe('generateRss', () => {
  it('preserves existing version dates and dates only new versions', () => {
    const rss = generateRss(changelog, existingRss, new Date('2026-08-03T15:00:00Z'))

    expect(rss).toContain(
      '<guid isPermaLink="false">v1.0.0</guid>\n      <pubDate>Mon, 01 Jan 2024 00:00:00 GMT</pubDate>',
    )
    expect(rss).toContain(
      '<guid isPermaLink="false">v2.0.0</guid>\n      <pubDate>Mon, 03 Aug 2026 15:00:00 GMT</pubDate>',
    )
  })

  it('is byte-identical when regenerating unchanged content', () => {
    const first = generateRss(changelog, existingRss, new Date('2026-08-03T15:00:00Z'))
    const second = generateRss(changelog, first, new Date('2026-08-04T15:00:00Z'))

    expect(second).toBe(first)
  })

  it('truncates long descriptions at a word boundary', () => {
    const completeWords = 'word '.repeat(99)
    const longChangelog = `# Changelog

## v3.0.0

${completeWords}boundaryword after
`

    const rss = generateRss(longChangelog, '', new Date('2026-08-03T15:00:00Z'))
    const description = rss.match(/<description>([\s\S]*?)<\/description>/g)?.at(-1)

    expect(description).toBe(`<description>${completeWords.trimEnd()}</description>`)
    expect(description).not.toContain('bound')
  })
})
