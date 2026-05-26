import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface BrowserFrameProps {
  children: ReactNode
  title?: string
  className?: string
  /** Accessible label for decorative code/diagram illustrations */
  ariaLabel?: string
}

export function BrowserFrame({ children, title, className, ariaLabel }: BrowserFrameProps) {
  return (
    <div
      role={ariaLabel ? 'img' : undefined}
      aria-label={ariaLabel}
      className={cn(
        'illustration overflow-hidden rounded-xl border border-border-subtle bg-card shadow-soft-lg',
        className,
      )}
    >
      <div className="flex items-center gap-2 border-b border-border-subtle bg-surface px-4 py-2.5">
        <div className="flex gap-1.5" aria-hidden>
          <span className="h-3 w-3 rounded-full bg-[oklch(0.78_0.16_25)]" />
          <span className="h-3 w-3 rounded-full bg-[oklch(0.85_0.16_85)]" />
          <span className="h-3 w-3 rounded-full bg-[oklch(0.78_0.16_150)]" />
        </div>
        {title && <span className="ml-3 text-xs text-muted-foreground select-none">{title}</span>}
      </div>
      <div className="bg-card">{children}</div>
    </div>
  )
}
