import { Link } from '@tanstack/react-router'
import { ThemeToggle } from './theme-toggle'

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-50 border-b bg-background/80 backdrop-blur-sm">
      <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-6">
        <div className="flex items-center gap-8">
          <Link to="/" className="text-lg font-semibold tracking-tight">
            Mdow
          </Link>
          <nav className="hidden items-center gap-6 text-sm md:flex">
            <Link
              to="/docs"
              className="text-muted-foreground transition-colors hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Docs
            </Link>
            <Link
              to="/changelog"
              className="text-muted-foreground transition-colors hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Changelog
            </Link>
            <Link
              to="/download"
              className="text-muted-foreground transition-colors hover:text-foreground"
              activeProps={{ className: 'text-foreground' }}
            >
              Download
            </Link>
          </nav>
        </div>
        <ThemeToggle />
      </div>
    </header>
  )
}
