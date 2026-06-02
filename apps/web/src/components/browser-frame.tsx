import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface BrowserFrameProps {
  children: ReactNode
  title?: string
  className?: string
  ariaLabel?: string
}

export function BrowserFrame({ children, title, className, ariaLabel }: BrowserFrameProps) {
  return (
    <div
      role={ariaLabel ? 'img' : undefined}
      aria-label={ariaLabel}
      className={cn('illustration overflow-hidden rounded-lg bg-card shadow-elevated', className)}
    >
      <div className="flex items-center gap-2 border-b border-border-subtle bg-surface px-3.5 py-2">
        <div className="flex gap-1.5" aria-hidden>
          <span className="size-2.5 rounded-full bg-[oklch(0.78_0.16_25)]" />
          <span className="size-2.5 rounded-full bg-[oklch(0.85_0.16_85)]" />
          <span className="size-2.5 rounded-full bg-[oklch(0.78_0.16_150)]" />
        </div>
        {title && (
          <span className="ml-3 select-none font-mono text-xs text-muted-foreground">{title}</span>
        )}
      </div>
      <div className="bg-card">{children}</div>
    </div>
  )
}
