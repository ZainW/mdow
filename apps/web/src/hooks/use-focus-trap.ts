import { useEffect, useRef, type RefObject } from 'react'

/** Trap focus inside `containerRef` while `active`, restore focus on close. */
export function useFocusTrap(active: boolean, containerRef: RefObject<HTMLElement | null>) {
  const triggerRef = useRef<HTMLElement | null>(null)

  useEffect(() => {
    if (!active) return

    triggerRef.current = document.activeElement as HTMLElement | null
    const container = containerRef.current
    if (!container) return

    const focusables = container.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])',
    )
    focusables[0]?.focus()

    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') return
      if (e.key !== 'Tab' || focusables.length === 0) return

      const first = focusables[0]
      const last = focusables[focusables.length - 1]

      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault()
        last.focus()
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault()
        first.focus()
      }
    }

    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('keydown', onKeyDown)
      triggerRef.current?.focus()
    }
  }, [active, containerRef])
}
