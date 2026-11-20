import { useEffect, useState } from 'react'
import { detectPlatform, type PlatformId } from '~/lib/download-links'

/** The visitor's system, or null until mounted (SSR renders the Mac button). */
export function usePlatform(): PlatformId | null {
  const [platform, setPlatform] = useState<PlatformId | null>(null)

  useEffect(() => {
    setPlatform(detectPlatform(navigator.userAgent))
  }, [])

  return platform
}
