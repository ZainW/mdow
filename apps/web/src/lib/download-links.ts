import type { ReleaseInfo } from './github-releases'

export type PlatformId = 'mac' | 'windows' | 'linux'

export function detectPlatform(userAgent: string): PlatformId {
  if (userAgent.includes('Mac')) return 'mac'
  if (userAgent.includes('Windows')) return 'windows'
  if (userAgent.includes('Linux')) return 'linux'
  return 'mac'
}

export function primaryDownloadUrl(release: ReleaseInfo, platform: PlatformId): string | null {
  if (platform === 'mac') {
    return release.assets.mac.dmg[0]?.url ?? release.assets.mac.zip[0]?.url ?? null
  }
  if (platform === 'windows') return release.assets.windows.exe
  return release.assets.linux.appImage
}

export function platformLabel(platform: PlatformId): string {
  if (platform === 'mac') return 'macOS'
  if (platform === 'windows') return 'Windows'
  return 'Linux'
}

export function downloadButtonLabel(platform: PlatformId): string {
  return `Download for ${platformLabel(platform)}`
}
