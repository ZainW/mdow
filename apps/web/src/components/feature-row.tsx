import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface FeatureRowProps {
  title: string
  description: string
  align: 'left' | 'right'
  children: ReactNode
}

export function FeatureRow({ title, description, align, children }: FeatureRowProps) {
  return (
    <div
      className={cn(
        'flex flex-col gap-10 md:flex-row md:items-center md:gap-16',
        align === 'right' && 'md:flex-row-reverse',
      )}
    >
      <div className="flex-1">
        <h3 className="text-2xl font-bold tracking-tight sm:text-3xl text-balance">{title}</h3>
        <p className="mt-3 text-muted-foreground text-balance">{description}</p>
      </div>
      <div className="flex-1">{children}</div>
    </div>
  )
}
