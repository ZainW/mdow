import { useEffect, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { cn } from '~/lib/utils'
import { btnPrimaryClass } from '~/lib/button-styles'
import { useFocusTrap } from '~/hooks/use-focus-trap'

const links = [
  { to: '/docs' as const, label: 'Docs' },
  { to: '/changelog' as const, label: 'Changelog' },
  { to: '/download' as const, label: 'Download' },
]

export function SiteMobileNav() {
  const [open, setOpen] = useState(false)
  const navRef = useRef<HTMLElement>(null)

  useFocusTrap(open, navRef)

  useEffect(() => {
    const main = document.querySelector('main')
    const footer = document.querySelector('footer')
    if (open) {
      main?.setAttribute('inert', '')
      footer?.setAttribute('inert', '')
      document.body.style.overflow = 'hidden'
    } else {
      main?.removeAttribute('inert')
      footer?.removeAttribute('inert')
      document.body.style.overflow = ''
    }
    return () => {
      main?.removeAttribute('inert')
      footer?.removeAttribute('inert')
      document.body.style.overflow = ''
    }
  }, [open])

  useEffect(() => {
    if (!open) return
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [open])

  return (
    <div className="md:hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="inline-flex min-h-11 min-w-11 items-center justify-center rounded-md border border-border bg-card text-muted-foreground transition-[background-color,color] duration-150 ease hover:bg-muted hover:text-foreground"
        aria-expanded={open}
        aria-controls="mobile-nav"
        aria-label={open ? 'Close menu' : 'Open menu'}
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          aria-hidden
        >
          {open ? (
            <>
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </>
          ) : (
            <>
              <path d="M4 6h16" />
              <path d="M4 12h16" />
              <path d="M4 18h16" />
            </>
          )}
        </svg>
      </button>
      {open && (
        <>
          <div
            className="fixed inset-0 z-overlay bg-foreground/20"
            aria-hidden
            onClick={() => setOpen(false)}
          />
          <nav
            id="mobile-nav"
            ref={navRef}
            className={cn(
              'absolute left-0 right-0 top-14 z-modal border-b bg-background/95 px-6 py-4 backdrop-blur-sm',
              'flex flex-col gap-1',
              'pb-[max(1rem,env(safe-area-inset-bottom))]',
            )}
          >
            {links.map((link) => (
              <Link
                key={link.to}
                to={link.to}
                onClick={() => setOpen(false)}
                className="rounded-md px-3 py-2.5 text-sm text-muted-foreground transition-[background-color,color] duration-150 ease hover:bg-muted hover:text-foreground"
                activeProps={{ className: 'bg-muted text-foreground' }}
              >
                {link.label}
              </Link>
            ))}
            <Link
              to="/download"
              onClick={() => setOpen(false)}
              className={cn(btnPrimaryClass, 'mt-2 h-11 px-4 text-sm font-medium')}
            >
              Download for free
            </Link>
          </nav>
        </>
      )}
    </div>
  )
}
