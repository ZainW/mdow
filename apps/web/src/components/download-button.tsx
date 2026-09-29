import type { ReactNode } from 'react'
import { btnPrimary, btnSecondary } from '~/lib/button-styles'
import type { PlatformId } from '~/lib/download-links'
import { AppleIcon, LinuxIcon, WindowsIcon } from './icons'

interface DownloadButtonProps {
  href: string
  children: ReactNode
  platform?: PlatformId
  className?: string
  variant?: 'primary' | 'secondary'
  size?: 'sm' | 'md' | 'lg'
}

export function PlatformIcon({
  platform,
  className,
}: {
  platform: PlatformId
  className?: string
}) {
  if (platform === 'mac') return <AppleIcon className={className} />
  if (platform === 'windows') return <WindowsIcon className={className} />
  return <LinuxIcon className={className} />
}

export function DownloadButton({
  href,
  children,
  platform,
  className,
  variant = 'primary',
  size = 'md',
}: DownloadButtonProps) {
  const cls = variant === 'primary' ? btnPrimary(size, className) : btnSecondary(size, className)
  return (
    <a href={href} className={cls}>
      {platform && <PlatformIcon platform={platform} className="-ml-0.5 size-[17px]" />}
      {children}
    </a>
  )
}
