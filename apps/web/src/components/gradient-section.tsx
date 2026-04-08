import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface GradientSectionProps {
  children: ReactNode
  className?: string
  variant?: 'gradient' | 'surface' | 'plain'
  innerClassName?: string
}

export function GradientSection({
  children,
  className,
  variant = 'plain',
  innerClassName,
}: GradientSectionProps) {
  return (
    <section
      className={cn(
        'relative w-full',
        variant === 'gradient' && 'bg-warm-gradient',
        variant === 'surface' && 'bg-section-warm',
        className,
      )}
    >
      <div className={cn('mx-auto max-w-6xl px-6 py-20 md:py-28', innerClassName)}>{children}</div>
    </section>
  )
}
