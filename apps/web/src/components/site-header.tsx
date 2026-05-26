import { Link } from '@tanstack/react-router'
import { Logo } from './logo'
import { SiteMobileNav } from './site-mobile-nav'
import { ThemeToggle } from './theme-toggle'
import { btnPrimaryClass } from '~/lib/button-styles'
import { cn } from '~/lib/utils'

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-header border-b bg-background/80 backdrop-blur-sm pt-[env(safe-area-inset-top)]">
      <div className="relative mx-auto flex h-14 max-w-6xl items-center justify-between px-6">
        <div className="flex items-center gap-4 md:gap-8">
          <SiteMobileNav />
          <Link to="/" className="flex items-center gap-2 text-lg font-semibold tracking-tight">
            <Logo className="h-6 w-6" />
            Mdow
          </Link>
          <nav className="hidden items-center gap-6 text-sm md:flex" aria-label="Main">
            <Link
              to="/docs"
              className="text-muted-foreground transition-[color] duration-150 ease hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Docs
            </Link>
            <Link
              to="/changelog"
              className="text-muted-foreground transition-[color] duration-150 ease hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Changelog
            </Link>
            <Link
              to="/download"
              className="text-muted-foreground transition-[color] duration-150 ease hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Download
            </Link>
          </nav>
        </div>
        <div className="flex items-center gap-2">
          <Link
            to="/download"
            className={cn(btnPrimaryClass, 'hidden h-9 px-4 text-sm sm:inline-flex')}
          >
            Download
          </Link>
          <ThemeToggle />
        </div>
      </div>
    </header>
  )
}
