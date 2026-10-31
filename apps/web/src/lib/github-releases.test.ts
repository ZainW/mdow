import { describe, expect, it } from 'vitest'
import { parseRelease } from './github-releases'

const sample = {
  tag_name: 'v1.2.3',
  name: 'v1.2.3',
  published_at: '2026-04-26T10:00:00Z',
  html_url: 'https://github.com/ZainW/mdow/releases/tag/v1.2.3',
  assets: [
    { name: 'Mdow-1.2.3.zip', browser_download_url: 'https://example.test/Mdow-1.2.3.zip' },
    { name: 'Mdow.zip', browser_download_url: 'https://example.test/Mdow.zip' },
    { name: 'mdow-appcast.xml', browser_download_url: 'https://example.test/mdow-appcast.xml' },
  ],
}

function asset(name: string, url: string) {
  return { name, browser_download_url: url }
}

function releaseWithAssets(assets: (typeof sample.assets)[number][]) {
  return { ...sample, assets }
}

describe('parseRelease', () => {
  it('extracts version, html_url, and the Mac app', () => {
    const result = parseRelease(sample)!
    expect(result.version).toBe('1.2.3')
    expect(result.htmlUrl).toBe('https://github.com/ZainW/mdow/releases/tag/v1.2.3')
    expect(result.publishedAt).toBe('2026-04-26T10:00:00Z')
    expect(result.assets.mac).toBe('https://example.test/Mdow-1.2.3.zip')
  })

  it('prefers the versioned Mac app zip, whatever the asset order', () => {
    const parsed = parseRelease(
      releaseWithAssets([asset('Mdow.zip', 'alias'), asset('Mdow-2.0.0.zip', 'versioned')]),
    )!
    expect(parsed.assets.mac).toBe('versioned')
  })

  it('falls back to the fixed-name Mac app zip', () => {
    const parsed = parseRelease(releaseWithAssets([asset('Mdow.zip', 'alias')]))!
    expect(parsed.assets.mac).toBe('alias')
  })

  it('ignores the retired Electron and Mdow Native archives', () => {
    const parsed = parseRelease(
      releaseWithAssets([
        asset('MdowNative-mac-beta.zip', 'gpui'),
        asset('Mdow-1.2.3-arm64-mac.zip', 'electron'),
        asset('Mdow-Setup-1.2.3.exe', 'exe'),
      ]),
    )!
    expect(parsed.assets.mac).toBeNull()
  })

  it('strips the leading v from tag_name', () => {
    const result = parseRelease({ ...sample, tag_name: 'v9.9.9' })
    expect(result?.version).toBe('9.9.9')
  })

  it('returns null when no assets are present', () => {
    const result = parseRelease({ ...sample, assets: [] })
    expect(result).toBeNull()
  })
})
