import { cn } from '@renderer/lib/utils'
import type { HTMLAttributes } from 'react'

export function Shimmer({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLSpanElement> & { children: string }) {
  return (
    <span
      className={cn(
        'inline-block bg-linear-to-r from-muted-foreground/40 via-foreground to-muted-foreground/40 bg-size-[200%_100%] bg-clip-text text-transparent animate-[companion-shimmer_1.6s_linear_infinite]',
        className,
      )}
      {...props}
    >
      {children}
    </span>
  )
}
