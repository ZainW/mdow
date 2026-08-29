export function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

/** Snap for high-frequency jumps (find next, tab into view). Travel for user-picked anchors. */
export function scrollBehavior(kind: 'snap' | 'travel' = 'travel'): ScrollBehavior {
  if (kind === 'snap' || prefersReducedMotion()) return 'auto'
  return 'smooth'
}

/** Theme flips must not replay hover/color transitions on every control. */
export function withoutTransitions(apply: () => void): void {
  const root = document.documentElement
  root.classList.add('theme-switching')
  apply()
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      root.classList.remove('theme-switching')
    })
  })
}
