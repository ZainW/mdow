export type PlatformId = 'mac' | 'linux'

export function detectPlatform(userAgent: string): PlatformId {
  if (userAgent.includes('Mac')) return 'mac'
  if (userAgent.includes('Linux')) return 'linux'
  return 'mac'
}
