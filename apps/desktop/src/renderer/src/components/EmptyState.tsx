import type { LucideIcon } from 'lucide-react'
import { cn } from '../lib/utils'
import { iconStroke } from '../lib/icons'

interface EmptyStateProps {
  icon: LucideIcon
  title: string
  hint?: string
  action?: React.ReactNode
  size?: 'sm' | 'md'
  className?: string
}

export function EmptyState({
  icon: IconComponent,
  title,
  hint,
  action,
  size = 'md',
  className,
}: EmptyStateProps) {
  if (size === 'sm') {
    return (
      <div
        className={cn(
          'flex flex-col items-center gap-1.5 px-3 py-6 text-center text-xs',
          className,
        )}
      >
        <IconComponent
          className="size-5 text-muted-foreground/40"
          strokeWidth={iconStroke.default}
          aria-hidden
        />
        <div className="text-sidebar-foreground/80">{title}</div>
        {hint && (
          <p className="text-[11px] leading-snug text-muted-foreground/70 max-w-[22ch]">{hint}</p>
        )}
        {action && <div className="mt-1">{action}</div>}
      </div>
    )
  }

  return (
    <div className={cn('flex flex-1 items-center justify-center', className)}>
      <div className="flex max-w-sm flex-col items-center gap-4 text-center">
        <div className="flex size-12 items-center justify-center rounded-full bg-muted" aria-hidden>
          <IconComponent
            className="size-6 text-muted-foreground"
            strokeWidth={iconStroke.default}
          />
        </div>
        <div className="space-y-1.5">
          <h2 className="text-base font-medium">{title}</h2>
          {hint && <p className="text-sm leading-relaxed text-muted-foreground">{hint}</p>}
        </div>
        {action && <div className="flex gap-2">{action}</div>}
      </div>
    </div>
  )
}
