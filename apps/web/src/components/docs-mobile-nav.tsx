import { useState, useEffect } from 'react'
import type { DocMeta } from '~/lib/content'
import { DocsSidebar } from './docs-sidebar'

interface DocsMobileNavProps {
  docs: DocMeta[]
  currentSlug: string
}

export function DocsMobileNav({ docs, currentSlug }: DocsMobileNavProps) {
  const [open, setOpen] = useState(false)

  useEffect(() => {
    setOpen(false)
  }, [currentSlug])

  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden'
      return () => {
        document.body.style.overflow = ''
      }
    }
  }, [open])

  return (
    <div className="lg:hidden">
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="inline-flex h-9 items-center gap-2 rounded-md border border-border bg-card px-3 text-sm font-medium transition-colors hover:bg-muted"
        aria-label="Open documentation menu"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
        >
          <line x1="3" x2="21" y1="6" y2="6" />
          <line x1="3" x2="21" y1="12" y2="12" />
          <line x1="3" x2="21" y1="18" y2="18" />
        </svg>
        Menu
      </button>
      {open && (
        <div className="fixed inset-0 z-50">
          <div
            className="absolute inset-0 bg-background/70 backdrop-blur-sm"
            onClick={() => setOpen(false)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') setOpen(false)
            }}
            role="button"
            tabIndex={-1}
            aria-label="Close menu"
          />
          <aside className="absolute left-0 top-0 h-full w-72 max-w-[80vw] overflow-y-auto border-r border-border bg-background p-6 shadow-soft-lg">
            <div className="mb-4 flex items-center justify-between">
              <span className="text-sm font-semibold">Documentation</span>
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                aria-label="Close menu"
              >
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden
                >
                  <path d="M18 6 6 18" />
                  <path d="m6 6 12 12" />
                </svg>
              </button>
            </div>
            <DocsSidebar docs={docs} currentSlug={currentSlug} />
          </aside>
        </div>
      )}
    </div>
  )
}
