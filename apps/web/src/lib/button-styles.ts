import { cn } from '~/lib/utils'

export const btnPrimaryClass = cn(
  'inline-flex items-center justify-center rounded-md bg-primary text-primary-foreground shadow-soft',
  'touch-manipulation',
  'transition-[transform,box-shadow,background-color] duration-150 ease-out',
  'active:scale-[0.98]',
  'motion-safe:hover-lift motion-safe:hover:bg-primary/90 motion-safe:hover:shadow-soft-lg',
)

export const btnSecondaryClass = cn(
  'inline-flex items-center justify-center rounded-md border border-border-subtle bg-card shadow-soft',
  'touch-manipulation',
  'transition-[transform,box-shadow,background-color,border-color] duration-150 ease-out',
  'active:scale-[0.98]',
  'motion-safe:hover:bg-muted motion-safe:hover:shadow-card',
)

export const cardLiftClass = cn(
  'transition-[transform,box-shadow] duration-200 ease-out',
  'motion-safe:hover-lift motion-safe:hover:shadow-elevated',
)
