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
      name: 'MdowNative-mac-beta.zip',
      browser_download_url: 'https://example.test/MdowNative-mac-beta.zip',
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
    expect(result.assets.mac.gpuiBeta).toEqual({
      url: 'https://example.test/MdowNative-mac-beta.zip',
    })
    expect(result.assets.windows.exe).toBe('https://example.test/Mdow-Setup-1.2.3.exe')
    expect(result.assets.linux.appImage).toBe('https://example.test/Mdow-1.2.3.AppImage')
  })

  it('recognizes the versioned GPUI mac beta asset as a fallback', () => {
    const result = parseRelease({
      ...sample,
      assets: [
        {
          name: 'MdowNative-1.2.3-arm64-mac-beta.zip',
          browser_download_url: 'https://example.test/MdowNative-1.2.3-arm64-mac-beta.zip',
        },
      ],
    })

    expect(result?.assets.mac.gpuiBeta).toEqual({
      arch: 'arm64',
      url: 'https://example.test/MdowNative-1.2.3-arm64-mac-beta.zip',
    })
  })

  it('classifies the GPUI alias separately from Electron mac archives', () => {
    const parsed = parseRelease(
      releaseWithAssets([
        asset('Mdow-2.0.0-arm64-mac.zip', 'electron'),
        asset('MdowNative-mac-beta.zip', 'gpui'),
      ]),
    )!

    expect(parsed.assets.mac.zip).toEqual([{ arch: 'arm64', url: 'electron' }])
    expect(parsed.assets.mac.gpuiBeta).toEqual({ url: 'gpui' })
  })

  it('prefers the stable GPUI alias when both beta names are present', () => {
    const parsed = parseRelease(
      releaseWithAssets([
        asset('MdowNative-2.0.0-arm64-mac-beta.zip', 'versioned'),
        asset('MdowNative-mac-beta.zip', 'alias'),
      ]),
    )!

    expect(parsed.assets.mac.gpuiBeta?.url).toBe('alias')
  })

  it('prefers the stable GPUI alias regardless of GitHub asset order', () => {
    const parsed = parseRelease(
      releaseWithAssets([
        asset('MdowNative-mac-beta.zip', 'alias'),
        asset('MdowNative-2.0.0-arm64-mac-beta.zip', 'versioned'),
      ]),
    )!

    expect(parsed.assets.mac.gpuiBeta?.url).toBe('alias')
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
