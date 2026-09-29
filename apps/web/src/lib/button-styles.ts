import { cn } from '~/lib/utils'

type ButtonSize = 'sm' | 'md' | 'lg'

export function btnPrimary(size: ButtonSize = 'md', className?: string) {
  return cn('btn btn-primary', `btn-${size}`, className)
}

export function btnSecondary(size: ButtonSize = 'md', className?: string) {
  return cn('btn btn-secondary', `btn-${size}`, className)
}

export function btnGhost(size: ButtonSize = 'md', className?: string) {
  return cn('btn btn-ghost', `btn-${size}`, className)
}
