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
      name: 'Mdow-for-Mac-1.2.3.zip',
      browser_download_url: 'https://example.test/Mdow-for-Mac-1.2.3.zip',
    },
    { name: 'Mdow-for-Mac.zip', browser_download_url: 'https://example.test/Mdow-for-Mac.zip' },
    {
      name: 'MdowNative-linux-beta.AppImage',
      browser_download_url: 'https://example.test/MdowNative-linux-beta.AppImage',
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

function asset(name: string, url: string) {
  return { name, browser_download_url: url }
}

function releaseWithAssets(assets: (typeof sample.assets)[number][]) {
  return { ...sample, assets }
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
    expect(result.assets.mac.app).toEqual({ url: 'https://example.test/Mdow-for-Mac-1.2.3.zip' })
    expect(result.assets.windows.exe).toBe('https://example.test/Mdow-Setup-1.2.3.exe')
    expect(result.assets.linux.appImage).toBe('https://example.test/Mdow-1.2.3.AppImage')
    expect(result.assets.linux.gpuiBeta).toEqual({
      url: 'https://example.test/MdowNative-linux-beta.AppImage',
    })
  })

  it('keeps Mdow for Mac out of the Electron archives, whatever the asset order', () => {
    const parsed = parseRelease(
      releaseWithAssets([
        asset('Mdow-for-Mac.zip', 'alias'),
        asset('Mdow-2.0.0-arm64-mac.zip', 'electron'),
        asset('Mdow-for-Mac-2.0.0.zip', 'versioned'),
      ]),
    )!

    expect(parsed.assets.mac.zip).toEqual([{ arch: 'arm64', url: 'electron' }])
    expect(parsed.assets.mac.app).toEqual({ url: 'versioned' })
  })

  it('falls back to the fixed-name Mdow for Mac zip', () => {
    const parsed = parseRelease(releaseWithAssets([asset('Mdow-for-Mac.zip', 'alias')]))!
    expect(parsed.assets.mac.app).toEqual({ url: 'alias' })
  })

  it('ignores the retired Mdow Native Mac beta in older releases', () => {
    const parsed = parseRelease(
      releaseWithAssets([
        asset('MdowNative-mac-beta.zip', 'gpui'),
        asset('MdowNative-1.2.3-arm64-mac-beta.zip', 'gpui-versioned'),
        asset('Mdow-1.2.3-arm64-mac.zip', 'electron'),
      ]),
    )!

    expect(parsed.assets.mac.zip).toEqual([{ arch: 'arm64', url: 'electron' }])
    expect(parsed.assets.mac.app).toBeNull()
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
