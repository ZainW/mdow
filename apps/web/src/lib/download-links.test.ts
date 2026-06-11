import { describe, expect, it } from 'vitest'
import {
  detectPlatform,
  downloadButtonLabel,
  nativeMacBetaDownloadUrl,
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

  it('returns exe for windows', () => {
    expect(primaryDownloadUrl(stableRelease, 'windows')).toBe('https://example.com/app.exe')
  })

  it('returns appimage for linux', () => {
    expect(primaryDownloadUrl(stableRelease, 'linux')).toBe('https://example.com/app.AppImage')
  })
})

describe('nativeMacBetaDownloadUrl', () => {
  it('uses the release asset when the native beta is published', () => {
    const release = parseRelease({
      tag_name: 'v1.0.5',
      published_at: '2026-05-26T00:00:00Z',
      html_url: 'https://github.com/ZainW/mdow/releases/tag/v1.0.5',
      assets: [
        {
          name: 'MdowNative-mac-beta.zip',
          browser_download_url: 'https://example.com/MdowNative-mac-beta.zip',
        },
      ],
    })!

    expect(nativeMacBetaDownloadUrl(release)).toBe('https://example.com/MdowNative-mac-beta.zip')
  })

  it('falls back to the latest-release beta alias when the API has no beta asset', () => {
    expect(nativeMacBetaDownloadUrl(stableRelease)).toBe(
      'https://github.com/ZainW/mdow/releases/latest/download/MdowNative-mac-beta.zip',
    )
  })
})

describe('labels', () => {
  it('formats platform labels', () => {
    expect(platformLabel('mac')).toBe('macOS')
    expect(downloadButtonLabel('windows')).toBe('Download for Windows')
  })
})
