import { cn } from '~/lib/utils'

/** Shared interactive button/link styles — touch-safe hovers, explicit transitions. */
export const btnPrimaryClass = cn(
  'inline-flex items-center justify-center rounded-lg bg-primary text-primary-foreground shadow-soft',
  'touch-manipulation',
  'transition-[transform,box-shadow,background-color] duration-150 ease-out',
  'active:scale-[0.98]',
  'motion-safe:hover-lift motion-safe:hover:bg-primary/95 motion-safe:hover:shadow-soft-lg',
)

export const btnSecondaryClass = cn(
  'inline-flex items-center justify-center rounded-lg border border-border bg-card',
  'touch-manipulation',
  'transition-[background-color,border-color] duration-150 ease',
  'active:scale-[0.98]',
  'motion-safe:hover:bg-muted',
)

export const cardLiftClass = cn(
  'transition-[transform,box-shadow] duration-150 ease-out',
  'motion-safe:hover-lift motion-safe:hover:shadow-soft-lg',
)
