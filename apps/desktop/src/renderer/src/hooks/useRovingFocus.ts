import { useCallback, useEffect, useRef } from 'react'

interface UseRovingFocusOptions {
  // Axis the container lays its options out on. 'horizontal' wires
  // ArrowLeft/Right; 'vertical' wires ArrowUp/Down; 'both' wires all four.
  orientation?: 'horizontal' | 'vertical' | 'both'
  // CSS selector for the focusable option elements inside the container.
  // Defaults to elements with a role attribute matching radio/tab/menuitem.
  itemSelector?: string
  // When the orientation wraps (typical for radiogroup/menu), Arrow at the
  // boundary cycles to the other end.
  loop?: boolean
}

// ARIA roving-tabindex implementation for radiogroups, tablists, and menus.
//
// Returns:
// - containerRef → attach to the wrapping role=radiogroup / tablist / menu
// - onKeyDown    → spread onto the container, handles Arrow/Home/End
// - tabIndexFor(isActive) → returns 0 for the active option, -1 for the rest
//   so only one option is in the tab order; arrow keys do the rest.
//
// The hook keeps the container's first paint's "active" option focusable via
// Tab and rotates focus on Arrow key, so screen-reader users get the canonical
// radiogroup keyboard contract.
export function useRovingFocus<T extends HTMLElement = HTMLDivElement>({
  orientation = 'both',
  itemSelector = '[role="radio"], [role="tab"], [role="menuitem"]',
  loop = true,
}: UseRovingFocusOptions = {}) {
  const containerRef = useRef<T | null>(null)

  const focusItem = useCallback(
    (delta: number, currentEl: Element | null) => {
      const container = containerRef.current
      if (!container) return
      const items = Array.from(container.querySelectorAll<HTMLElement>(itemSelector)).filter(
        (el) => !el.hasAttribute('disabled') && el.getAttribute('aria-disabled') !== 'true',
      )
      if (items.length === 0) return
      const idx = currentEl instanceof HTMLElement ? items.indexOf(currentEl) : -1
      let next: number
      if (idx < 0) {
        next = delta > 0 ? 0 : items.length - 1
      } else {
        next = idx + delta
        if (loop) {
          next = (next + items.length) % items.length
        } else {
          next = Math.max(0, Math.min(items.length - 1, next))
        }
      }
      items[next]?.focus()
    },
    [itemSelector, loop],
  )

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLElement>) => {
      const horizontal = orientation === 'horizontal' || orientation === 'both'
      const vertical = orientation === 'vertical' || orientation === 'both'
      const active = e.target instanceof HTMLElement ? e.target : null
      const isItem = active?.matches(itemSelector)
      if (!isItem) return

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
        case 'Home': {
          e.preventDefault()
          const container = containerRef.current
          if (!container) return
          const items = Array.from(container.querySelectorAll<HTMLElement>(itemSelector))
          items[0]?.focus()
          return
        }
        case 'End': {
          e.preventDefault()
          const container = containerRef.current
          if (!container) return
          const items = Array.from(container.querySelectorAll<HTMLElement>(itemSelector))
          items[items.length - 1]?.focus()
          return
        }
        default:
          return
      }
    },
    [orientation, itemSelector, focusItem],
  )

  // Whenever the container mounts, make sure the initially-active item (the
  // one with tabindex=0) is the only one in the tab order — defensively, in
  // case the consumer forgot to apply tabIndexFor everywhere.
  useEffect(() => {
    const container = containerRef.current
    if (!container) return
    const items = Array.from(container.querySelectorAll<HTMLElement>(itemSelector))
    let hasZero = false
    for (const item of items) {
      if (item.getAttribute('tabindex') === '0') {
        hasZero = true
        break
      }
    }
    if (!hasZero && items[0]) {
      items[0].setAttribute('tabindex', '0')
    }
  }, [itemSelector])

  return { containerRef, onKeyDown }
}

// Helper: pick the right tabindex for a roving group's item.
export function rovingTabIndex(isActive: boolean): 0 | -1 {
  return isActive ? 0 : -1
}
