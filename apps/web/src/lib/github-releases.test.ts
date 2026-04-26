import { describe, expect, it } from 'vitest'
import { parseRelease } from './github-releases'

const sample = {
  tag_name: 'v1.2.3',
  name: 'v1.2.3',
  published_at: '2026-04-26T10:00:00Z',
  html_url: 'https://github.com/ZainW/mdow/releases/tag/v1.2.3',
  assets: [
    {
      name: 'Mdow-1.2.3-arm64.dmg',
      browser_download_url: 'https://example.test/Mdow-1.2.3-arm64.dmg',
    },
    { name: 'Mdow-1.2.3-x64.dmg', browser_download_url: 'https://example.test/Mdow-1.2.3-x64.dmg' },
    {
      name: 'Mdow-1.2.3-arm64-mac.zip',
      browser_download_url: 'https://example.test/Mdow-1.2.3-arm64-mac.zip',
    },
    {
      name: 'Mdow-Setup-1.2.3.exe',
      browser_download_url: 'https://example.test/Mdow-Setup-1.2.3.exe',
    },
    {
      name: 'Mdow-1.2.3.AppImage',
      browser_download_url: 'https://example.test/Mdow-1.2.3.AppImage',
    },
    { name: 'latest.yml', browser_download_url: 'https://example.test/latest.yml' },
  ],
}

describe('parseRelease', () => {
  it('extracts version, html_url, and platform-keyed assets', () => {
    const result = parseRelease(sample)!
    expect(result.version).toBe('1.2.3')
    expect(result.htmlUrl).toBe('https://github.com/ZainW/mdow/releases/tag/v1.2.3')
    expect(result.publishedAt).toBe('2026-04-26T10:00:00Z')
    expect(result.assets.mac.dmg).toEqual([
      { arch: 'arm64', url: 'https://example.test/Mdow-1.2.3-arm64.dmg' },
      { arch: 'x64', url: 'https://example.test/Mdow-1.2.3-x64.dmg' },
    ])
    expect(result.assets.mac.zip).toHaveLength(1)
    expect(result.assets.windows.exe).toBe('https://example.test/Mdow-Setup-1.2.3.exe')
    expect(result.assets.linux.appImage).toBe('https://example.test/Mdow-1.2.3.AppImage')
  })

  it('strips the leading v from tag_name', () => {
    const result = parseRelease({ ...sample, tag_name: 'v9.9.9' })
    expect(result?.version).toBe('9.9.9')
  })

  it('returns null when no assets are present', () => {
    const result = parseRelease({ ...sample, assets: [] })
    expect(result).toBeNull()
  })

  it('ignores update-manifest yml files in asset matching', () => {
    const result = parseRelease(sample)
    const allUrls = JSON.stringify(result)
    expect(allUrls).not.toContain('latest.yml')
  })
})
