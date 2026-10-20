import { describe, expect, it } from 'vitest'
import {
  detectPlatform,
  downloadButtonLabel,
  gpuiLinuxBetaDownloadUrl,
  platformLabel,
  primaryDownloadUrl,
} from './download-links'
import { parseRelease } from './github-releases'

const stableRelease = parseRelease({
  tag_name: 'v1.0.5',
  published_at: '2026-05-26T00:00:00Z',
  html_url: 'https://github.com/ZainW/mdow/releases/tag/v1.0.5',
  assets: [
    { name: 'Mdow-1.0.5-arm64.dmg', browser_download_url: 'https://example.com/app.dmg' },
    { name: 'Mdow-1.0.5.exe', browser_download_url: 'https://example.com/app.exe' },
    { name: 'Mdow-1.0.5.AppImage', browser_download_url: 'https://example.com/app.AppImage' },
  ],
})!

describe('detectPlatform', () => {
  it('detects macOS', () => {
    expect(detectPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X)')).toBe('mac')
  })

  it('detects Windows', () => {
    expect(detectPlatform('Mozilla/5.0 (Windows NT 10.0)')).toBe('windows')
  })

  it('detects Linux', () => {
    expect(detectPlatform('Mozilla/5.0 (X11; Linux x86_64)')).toBe('linux')
  })
})

describe('primaryDownloadUrl', () => {
  it('returns dmg for mac', () => {
    expect(primaryDownloadUrl(stableRelease, 'mac')).toBe('https://example.com/app.dmg')
  })

  it('prefers the Swift app zip for mac', () => {
    const release = parseRelease({
      tag_name: 'v1.0.5',
      published_at: '2026-05-26T00:00:00Z',
      html_url: 'https://github.com/ZainW/mdow/releases/tag/v1.0.5',
      assets: [
        { name: 'Mdow-1.0.5.zip', browser_download_url: 'https://example.com/Mdow-1.0.5.zip' },
        { name: 'Mdow-1.0.5.exe', browser_download_url: 'https://example.com/app.exe' },
      ],
    })!
    expect(primaryDownloadUrl(release, 'mac')).toBe('https://example.com/Mdow-1.0.5.zip')
  })

  it('returns exe for windows', () => {
    expect(primaryDownloadUrl(stableRelease, 'windows')).toBe('https://example.com/app.exe')
  })

  it('returns appimage for linux', () => {
    expect(primaryDownloadUrl(stableRelease, 'linux')).toBe('https://example.com/app.AppImage')
  })
})

describe('gpuiLinuxBetaDownloadUrl', () => {
  it('prefers the parsed Linux GPUI alias', () => {
    const release = parseRelease({
      tag_name: 'v1.0.5',
      published_at: '2026-05-26T00:00:00Z',
      html_url: 'https://github.com/ZainW/mdow/releases/tag/v1.0.5',
      assets: [
        {
          name: 'mdow-native_1.0.5_x86_64.AppImage',
          browser_download_url: 'https://example.com/mdow-native_1.0.5_x86_64.AppImage',
        },
        {
          name: 'MdowNative-linux-beta.AppImage',
          browser_download_url: 'https://example.com/MdowNative-linux-beta.AppImage',
        },
      ],
    })!

    expect(gpuiLinuxBetaDownloadUrl(release)).toBe(
      'https://example.com/MdowNative-linux-beta.AppImage',
    )
  })

  it('falls back to the durable Linux alias when the API has no beta asset', () => {
    expect(gpuiLinuxBetaDownloadUrl(stableRelease)).toBe(
      'https://github.com/ZainW/mdow/releases/latest/download/MdowNative-linux-beta.AppImage',
    )
  })
})

describe('labels', () => {
  it('formats platform labels', () => {
    expect(platformLabel('mac')).toBe('macOS')
    expect(downloadButtonLabel('windows')).toBe('Download for Windows')
  })
})
