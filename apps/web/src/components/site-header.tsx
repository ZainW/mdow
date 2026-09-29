import { Link } from '@tanstack/react-router'
import { useScrollPast } from '~/hooks/use-scroll-past'
import { btnPrimary } from '~/lib/button-styles'
import { GITHUB_URL, NAV_LINKS } from '~/lib/site'
import { cn } from '~/lib/utils'
import { GitHubIcon } from './icons'
import { Logo } from './logo'
import { SiteMobileNav } from './site-mobile-nav'
import { ThemeToggle } from './theme-toggle'

export function SiteHeader() {
  const scrolled = useScrollPast(4)

  return (
    <header
      className={cn(
        'sticky top-0 z-header pt-[env(safe-area-inset-top)]',
        'border-b bg-background/85 backdrop-blur-lg backdrop-saturate-150',
        'transition-[border-color] duration-200 ease',
        scrolled ? 'border-border-subtle' : 'border-transparent',
      )}
    >
      <div className="relative mx-auto flex h-16 max-w-6xl items-center justify-between gap-4 px-5 sm:px-6">
        <div className="flex items-center gap-8">
          <Link
            to="/"
            className="-m-1 flex items-center gap-2.5 rounded-lg p-1 text-[17px] font-semibold tracking-tight"
          >
            <Logo className="size-7" alt="" />
            Mdow
          </Link>
          <nav className="hidden items-center gap-1 text-sm md:flex" aria-label="Main">
            {NAV_LINKS.map((link) => (
              <Link
                key={link.to}
                to={link.to}
                className="rounded-md px-3 py-1.5 text-muted-foreground transition-colors duration-150 ease hover:text-foreground"
                activeProps={{ className: 'text-foreground' }}
              >
                {link.label}
              </Link>
            ))}
          </nav>
        </div>
        <div className="flex items-center gap-1">
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="btn btn-ghost hidden size-9 rounded-lg p-0 sm:inline-flex"
            aria-label="Mdow on GitHub"
          >
            <GitHubIcon className="size-[18px]" />
          </a>
          <ThemeToggle />
          <Link to="/download" className={btnPrimary('sm', 'ml-2 hidden sm:inline-flex')}>
            Download
          </Link>
          <SiteMobileNav />
        </div>
      </div>
    </header>
  )
}
