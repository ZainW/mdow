import { useCallback, useEffect, useRef } from 'react'

export const ROVING_ITEM_SELECTOR = '[role="radio"], [role="tab"], [role="menuitem"]'

interface UseRovingFocusOptions {
  // Axis the container lays its options out on. 'horizontal' wires
  // ArrowLeft/Right; 'vertical' wires ArrowUp/Down; 'both' wires all four.
  orientation?: 'horizontal' | 'vertical' | 'both'
  // Focus the first non-disabled option as soon as the container mounts.
  // Useful for popovers/menus that should swallow keyboard focus on open.
  autoFocusFirst?: boolean
}

// ARIA roving-tabindex implementation for radiogroups, tablists, and menus.
// Returns { containerRef, onKeyDown } — spread the ref + handler on the
// container, mark the active option with tabIndex={0} and the rest with -1
// (helper `rovingTabIndex(active)` does this), and arrow keys do the rest.
export function useRovingFocus<T extends HTMLElement = HTMLDivElement>({
  orientation = 'both',
  autoFocusFirst = false,
}: UseRovingFocusOptions = {}) {
  const containerRef = useRef<T | null>(null)

  const items = useCallback((includeDisabled: boolean): HTMLElement[] => {
    const container = containerRef.current
    if (!container) return []
    const all = Array.from(container.querySelectorAll<HTMLElement>(ROVING_ITEM_SELECTOR))
    return includeDisabled
      ? all
      : all.filter(
          (el) => !el.hasAttribute('disabled') && el.getAttribute('aria-disabled') !== 'true',
        )
  }, [])

  const focusItem = useCallback(
    (delta: number, currentEl: Element | null) => {
      const enabled = items(false)
      if (enabled.length === 0) return
      const idx = currentEl instanceof HTMLElement ? enabled.indexOf(currentEl) : -1
      const next =
        idx < 0
          ? delta > 0
            ? 0
            : enabled.length - 1
          : (idx + delta + enabled.length) % enabled.length
      enabled[next]?.focus()
    },
    [items],
  )

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLElement>) => {
      const horizontal = orientation === 'horizontal' || orientation === 'both'
      const vertical = orientation === 'vertical' || orientation === 'both'
      const active = e.target instanceof HTMLElement ? e.target : null
      if (!active?.matches(ROVING_ITEM_SELECTOR)) return

      switch (e.key) {
        case 'ArrowRight':
          if (!horizontal) return
          e.preventDefault()
          focusItem(1, active)
          return
        case 'ArrowLeft':
          if (!horizontal) return
          e.preventDefault()
          focusItem(-1, active)
          return
        case 'ArrowDown':
          if (!vertical) return
          e.preventDefault()
          focusItem(1, active)
          return
        case 'ArrowUp':
          if (!vertical) return
          e.preventDefault()
          focusItem(-1, active)
          return
        case 'Home':
          e.preventDefault()
          items(false)[0]?.focus()
          return
        case 'End': {
          e.preventDefault()
          const enabled = items(false)
          enabled[enabled.length - 1]?.focus()
          return
        }
      }
    },
    [orientation, focusItem, items],
  )

  useEffect(() => {
    if (autoFocusFirst) items(false)[0]?.focus()
  }, [autoFocusFirst, items])

  return { containerRef, onKeyDown }
}

// Helper: pick the right tabindex for a roving group's item.
export function rovingTabIndex(isActive: boolean): 0 | -1 {
  return isActive ? 0 : -1
}
