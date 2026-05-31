import { Link } from '@tanstack/react-router'
import { Logo } from './logo'

const GITHUB_URL = 'https://github.com/ZainW/mdow'

export function SiteFooter() {
  return (
    <footer className="border-t py-12 pb-[max(3rem,env(safe-area-inset-bottom))]">
      <div className="mx-auto max-w-6xl px-6">
        <div className="flex flex-col items-center gap-8 md:flex-row md:justify-between">
          <div className="flex items-center gap-3">
            <Logo className="h-5 w-5 opacity-60" />
            <div className="text-sm">
              <p className="font-medium text-foreground">Mdow</p>
              <p className="text-xs text-muted-foreground">A quiet place to read markdown.</p>
            </div>
          </div>
          <nav className="flex flex-wrap items-center justify-center gap-x-8 gap-y-2 text-sm text-muted-foreground">
            <Link to="/docs" className="transition-colors duration-150 ease hover:text-foreground">
              Docs
            </Link>
            <Link
              to="/changelog"
              className="transition-colors duration-150 ease hover:text-foreground"
            >
              Changelog
            </Link>
            <Link
              to="/download"
              className="transition-colors duration-150 ease hover:text-foreground"
            >
              Download
            </Link>
            <a
              href={GITHUB_URL}
              className="transition-colors duration-150 ease hover:text-foreground"
              target="_blank"
              rel="noopener noreferrer"
            >
              GitHub
            </a>
          </nav>
        </div>
        <div className="mt-8 border-t border-border-subtle pt-6 text-center text-xs text-muted-foreground">
          &copy; {new Date().getFullYear()} Mdow · MIT License
        </div>
      </div>
    </footer>
  )
}
