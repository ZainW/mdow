export type PlatformId = 'mac' | 'linux'

/** Mdow for Linux is published as a rolling pre-release, separate from the Mac releases. */
export const LINUX_PREVIEW_URL = 'https://github.com/ZainW/mdow/releases/tag/linux-latest'

export function detectPlatform(userAgent: string): PlatformId {
  if (userAgent.includes('Mac')) return 'mac'
  if (userAgent.includes('Linux')) return 'linux'
  return 'mac'
}
