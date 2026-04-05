import { Link } from '@tanstack/react-router'

export function SiteFooter() {
  return (
    <footer className="border-t py-8">
      <div className="mx-auto flex max-w-6xl flex-col items-center gap-4 px-6 text-sm text-muted-foreground md:flex-row md:justify-between">
        <p>&copy; {new Date().getFullYear()} Mdow</p>
        <nav className="flex gap-6">
          <Link to="/docs" className="transition-colors hover:text-foreground">
            Docs
          </Link>
          <Link to="/changelog" className="transition-colors hover:text-foreground">
            Changelog
          </Link>
          <Link to="/download" className="transition-colors hover:text-foreground">
            Download
          </Link>
        </nav>
      </div>
    </footer>
  )
}
