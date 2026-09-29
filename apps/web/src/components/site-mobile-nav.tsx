import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { Link, useRouterState } from '@tanstack/react-router'
import { useFocusTrap } from '~/hooks/use-focus-trap'
import { btnPrimary } from '~/lib/button-styles'
import { GITHUB_URL, NAV_LINKS } from '~/lib/site'
import { cn } from '~/lib/utils'
import { CloseIcon, ExternalIcon, MenuIcon } from './icons'

export function SiteMobileNav() {
  const [open, setOpen] = useState(false)
  const navRef = useRef<HTMLDivElement>(null)
  const pathname = useRouterState({ select: (s) => s.location.pathname })

  useFocusTrap(open, navRef)

  useEffect(() => {
    setOpen(false)
  }, [pathname])

  useEffect(() => {
    const main = document.querySelector('main')
    const footer = document.querySelector('footer')
    if (open) {
      main?.setAttribute('inert', '')
      footer?.setAttribute('inert', '')
      document.body.style.overflow = 'hidden'
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
        className="btn btn-ghost ml-1 size-11 rounded-lg p-0"
        aria-expanded={open}
        aria-controls="mobile-nav"
        aria-label={open ? 'Close menu' : 'Open menu'}
      >
        {open ? <CloseIcon className="size-5" /> : <MenuIcon className="size-5" />}
      </button>
      {open && (
        <>
          {/* Portaled: the header's backdrop-filter would otherwise contain this
              fixed overlay to the header's own box. */}
          {createPortal(
            <div
              className="animate-overlay-in fixed inset-0 top-[calc(4rem+env(safe-area-inset-top))] z-[99] bg-background/60 backdrop-blur-sm"
              aria-hidden
              onClick={() => setOpen(false)}
            />,
            document.body,
          )}
          <div
            id="mobile-nav"
            ref={navRef}
            className={cn(
              'animate-drop-in absolute inset-x-0 top-16 z-modal border-b border-border-subtle bg-background px-5 pt-2',
              'pb-[max(1.25rem,env(safe-area-inset-bottom))]',
            )}
          >
            <nav aria-label="Mobile" className="flex flex-col">
              {NAV_LINKS.map((link) => (
                <Link
                  key={link.to}
                  to={link.to}
                  onClick={() => setOpen(false)}
                  className="flex min-h-12 items-center border-b border-border-subtle text-base text-muted-foreground"
                  activeProps={{ className: 'text-foreground font-medium' }}
                >
                  {link.label}
                </Link>
              ))}
              <a
                href={GITHUB_URL}
                target="_blank"
                rel="noopener noreferrer"
                className="flex min-h-12 items-center justify-between border-b border-border-subtle text-base text-muted-foreground"
              >
                GitHub
                <ExternalIcon className="size-4" />
              </a>
            </nav>
            <Link
              to="/download"
              onClick={() => setOpen(false)}
              className={btnPrimary('lg', 'mt-5 w-full')}
            >
              Download Mdow
            </Link>
          </div>
        </>
      )}
    </div>
  )
}
