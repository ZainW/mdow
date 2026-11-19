import { Link } from '@tanstack/react-router'
import { DownloadButton } from '~/components/download-button'
import { usePlatform } from '~/hooks/use-platform'
import { btnPrimary } from '~/lib/button-styles'
import type { PlatformId } from '~/lib/download-links'
import type { ReleaseInfo } from '~/lib/github-releases'

const LABEL: Record<PlatformId, string> = { mac: 'macOS', linux: 'Linux' }

/** The release asset for a system: the Mac zip, or the AppImage on Linux. */
export function assetFor(release: ReleaseInfo | null, platform: PlatformId): string | null {
  if (!release) return null
  return platform === 'mac' ? release.assets.mac : release.assets.appImage
}

/**
 * The primary download for the visitor's system, falling back to the download
 * page when the release has no build for it.
 */
export function DownloadCta({
  release,
  size = 'lg',
  className,
  compact = false,
}: {
  release: ReleaseInfo | null
  size?: 'sm' | 'md' | 'lg'
  className?: string
  compact?: boolean
}) {
  const platform = usePlatform() ?? 'mac'
  const href = assetFor(release, platform)

  if (!href) {
    return (
      <Link to="/download" className={btnPrimary(size, className)}>
        {compact ? 'Download' : 'Download for free'}
      </Link>
    )
  }
  return (
    <DownloadButton href={href} platform={platform} size={size} className={className}>
      {compact ? 'Download' : `Download for ${LABEL[platform]}`}
    </DownloadButton>
  )
}

/** "Also on Linux" on a Mac, and the other way round. */
export function OtherPlatformLink({ className }: { className?: string }) {
  const platform = usePlatform() ?? 'mac'
  return (
    <Link to="/download" className={className}>
      Also on {platform === 'mac' ? 'Linux' : 'macOS'}
    </Link>
  )
}
