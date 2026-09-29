import { describe, expect, it } from 'vitest'
import { parseChangelog, releaseHeadline } from './changelog'
import { versionAnchor } from './release-format'

const RAW = `---
title: Changelog
---

# Changelog

## v1.10.0

Latest release.

- Huge documents open about 5× faster: a 3 MB file renders fully in under a second
- Syntax highlighting renders as you scroll

## v1.9.3

- Fixed Mac Electron \`CSC_NAME\`; builds publish again
`

const RSS = `
    <item>
      <guid isPermaLink="false">v1.10.0</guid>
      <pubDate>Sun, 27 Sep 2026 22:45:55 GMT</pubDate>
    </item>`

describe('parseChangelog', () => {
  it('splits releases, strips the "Latest release." note, and collects bullets', () => {
    const [latest, previous] = parseChangelog(RAW, RSS)

    expect(latest.version).toBe('v1.10.0')
    expect(latest.markdown).not.toContain('Latest release.')
    expect(latest.bullets).toHaveLength(2)
    expect(previous.version).toBe('v1.9.3')
  })

  it('uses the same anchors as the RSS feed', () => {
    const [latest] = parseChangelog(RAW, RSS)
    expect(latest.anchor).toBe('v1-10-0')
    expect(versionAnchor('v1.9.3')).toBe('v1-9-3')
  })

  it('reads publication dates from the RSS feed when known', () => {
    const [latest, previous] = parseChangelog(RAW, RSS)
    expect(latest.date).toBe('2026-09-27T22:45:55.000Z')
    expect(previous.date).toBeNull()
  })
})

describe('releaseHeadline', () => {
  it('uses the first bullet up to its first clause break', () => {
    const [latest, previous] = parseChangelog(RAW, RSS)
    expect(releaseHeadline(latest)).toBe('Huge documents open about 5× faster')
    expect(releaseHeadline(previous)).toBe('Fixed Mac Electron CSC_NAME')
  })

  it('truncates long headlines at a word boundary', () => {
    const [latest] = parseChangelog(RAW, RSS)
    expect(releaseHeadline(latest, 20)).toBe('Huge documents open…')
  })
})
