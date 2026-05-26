import { Link } from '@tanstack/react-router'

const GITHUB_URL = 'https://github.com/ZainW/mdow'

export function SiteFooter() {
  return (
    <footer className="border-t py-10 pb-[max(2.5rem,env(safe-area-inset-bottom))]">
      <div className="mx-auto flex max-w-6xl flex-col items-center gap-6 px-6 text-sm text-muted-foreground md:flex-row md:justify-between">
        <div className="text-center md:text-left">
          <p>&copy; {new Date().getFullYear()} Mdow</p>
          <p className="mt-1 text-xs">A quiet place to read markdown.</p>
        </div>
        <nav className="flex flex-wrap items-center justify-center gap-x-6 gap-y-2">
          <Link to="/docs" className="transition-colors hover:text-foreground">
            Docs
          </Link>
          <Link to="/changelog" className="transition-colors hover:text-foreground">
            Changelog
          </Link>
          <Link to="/download" className="transition-colors hover:text-foreground">
            Download
          </Link>
          <a
            href={GITHUB_URL}
            className="transition-colors hover:text-foreground"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>
        </nav>
      </div>
    </footer>
  )
}
