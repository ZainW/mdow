import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'
import { btnPrimaryClass, btnSecondaryClass } from '~/lib/button-styles'

interface DownloadButtonProps {
  href: string
  children: ReactNode
  className?: string
  variant?: 'primary' | 'secondary'
  size?: 'default' | 'lg'
}

export function DownloadButton({
  href,
  children,
  className,
  variant = 'primary',
  size = 'default',
}: DownloadButtonProps) {
  return (
    <a
      href={href}
      className={cn(
        variant === 'primary' ? btnPrimaryClass : btnSecondaryClass,
        size === 'default' && 'h-11 px-7 text-sm font-medium',
        size === 'lg' && 'h-12 px-8 text-sm font-medium',
        className,
      )}
    >
      {children}
    </a>
  )
}
