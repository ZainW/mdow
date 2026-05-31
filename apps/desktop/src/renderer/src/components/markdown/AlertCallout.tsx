import type { HTMLAttributes } from 'react'
import { cn } from '../../lib/utils'

export const ALERT_TYPES = ['tip', 'note', 'important', 'warning', 'caution'] as const

export function AlertCallout({
  type,
  children,
  className,
  ...props
}: HTMLAttributes<HTMLDivElement> & { type: string }) {
  return (
    <div
      className={cn('markdown-alert', `markdown-alert-${type}`, className)}
      role="note"
      {...props}
    >
      {children}
    </div>
  )
}
