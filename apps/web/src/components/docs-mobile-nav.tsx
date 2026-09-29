import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { DocMeta } from '~/lib/content'
import { useFocusTrap } from '~/hooks/use-focus-trap'
import { DocsSidebar } from './docs-sidebar'
import { CloseIcon, MenuIcon } from './icons'

interface DocsMobileNavProps {
  docs: DocMeta[]
  currentSlug: string
}

export function DocsMobileNav({ docs, currentSlug }: DocsMobileNavProps) {
  const [open, setOpen] = useState(false)
  const panelRef = useRef<HTMLDivElement>(null)
  const current = docs.find((d) => d.slug === currentSlug)

  useFocusTrap(open, panelRef)

  useEffect(() => {
    setOpen(false)
  }, [currentSlug])

  useEffect(() => {
    if (!open) return
    document.body.style.overflow = 'hidden'
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.body.style.overflow = ''
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open])

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="inline-flex h-9 min-w-0 max-w-[55%] shrink-0 items-center gap-2 rounded-lg border border-border bg-card px-3 text-sm font-medium"
        aria-expanded={open}
        aria-label={`Documentation menu${current ? `, current page ${current.title}` : ''}`}
      >
        <MenuIcon className="size-4 shrink-0 text-muted-foreground" />
        <span className="truncate">{current?.title ?? 'Menu'}</span>
      </button>
      {/* Portaled: the sticky docs bar uses backdrop-filter, which would
          otherwise become the containing block for this fixed drawer. */}
      {open &&
        createPortal(
          <div className="fixed inset-0 z-modal">
            <div
              className="animate-overlay-in absolute inset-0 bg-foreground/15 backdrop-blur-[2px] dark:bg-black/50"
              onClick={() => setOpen(false)}
              aria-hidden
            />
            <div
              ref={panelRef}
              role="dialog"
              aria-modal="true"
              aria-label="Documentation"
              className="animate-slide-in-left absolute inset-y-0 left-0 w-80 max-w-[85vw] overflow-y-auto border-r border-border bg-background px-3 pb-[max(1.5rem,env(safe-area-inset-bottom))] pt-[max(1rem,env(safe-area-inset-top))] shadow-[16px_0_48px_-16px_hsl(var(--shadow-color)/0.3)]"
            >
              <div className="mb-4 flex items-center justify-between pl-3">
                <span className="text-sm font-semibold">Documentation</span>
                <button
                  type="button"
                  onClick={() => setOpen(false)}
                  className="btn btn-ghost size-11 rounded-lg p-0"
                  aria-label="Close menu"
                >
                  <CloseIcon className="size-5" />
                </button>
              </div>
              <DocsSidebar docs={docs} currentSlug={currentSlug} />
            </div>
          </div>,
          document.body,
        )}
    </>
  )
}
