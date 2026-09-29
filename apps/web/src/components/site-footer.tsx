import type { ReactNode } from 'react'
import { Link } from '@tanstack/react-router'
import { AUTHOR, GITHUB_URL, LICENSE_URL, RELEASES_URL } from '~/lib/site'
import { Logo } from './logo'

const linkClass = 'text-muted-foreground transition-colors duration-150 ease hover:text-foreground'

export function SiteFooter() {
  return (
    <footer className="border-t border-border-subtle pb-[max(2.5rem,env(safe-area-inset-bottom))] pt-14">
      <div className="mx-auto max-w-6xl px-5 sm:px-6">
        <div className="grid gap-10 sm:grid-cols-2 lg:grid-cols-[1.6fr_1fr_1fr_1fr]">
          <div className="max-w-xs">
            <Link to="/" className="inline-flex items-center gap-2.5 font-semibold tracking-tight">
              <Logo className="size-6" alt="" />
              Mdow
            </Link>
            <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
              A quiet place to read markdown. Free and open source for macOS, Windows, and Linux.
            </p>
          </div>
          <FooterColumn title="Product">
            <Link to="/download" className={linkClass}>
              Download
            </Link>
            <Link to="/changelog" className={linkClass}>
              Changelog
            </Link>
            <a href="/changelog/rss.xml" className={linkClass}>
              RSS feed
            </a>
          </FooterColumn>
          <FooterColumn title="Docs">
            <Link to="/docs/$" params={{ _splat: 'getting-started' }} className={linkClass}>
              Getting started
            </Link>
            <Link to="/docs/$" params={{ _splat: 'ai-companion' }} className={linkClass}>
              AI companion
            </Link>
            <Link to="/docs/$" params={{ _splat: 'shortcuts' }} className={linkClass}>
              Keyboard shortcuts
            </Link>
          </FooterColumn>
          <FooterColumn title="Project">
            <a href={GITHUB_URL} className={linkClass} target="_blank" rel="noopener noreferrer">
              GitHub
            </a>
            <a href={RELEASES_URL} className={linkClass} target="_blank" rel="noopener noreferrer">
              Releases
            </a>
            <a href={LICENSE_URL} className={linkClass} target="_blank" rel="noopener noreferrer">
              MIT License
            </a>
          </FooterColumn>
        </div>
        <div className="mt-14 flex flex-col gap-2 border-t border-border-subtle pt-6 text-[13px] text-faint sm:flex-row sm:items-center sm:justify-between">
          <p>&copy; {new Date().getFullYear()} Mdow</p>
          <p>
            Made by{' '}
            <a
              href={AUTHOR.url}
              className="link-underline text-muted-foreground"
              target="_blank"
              rel="noopener noreferrer"
            >
              {AUTHOR.name}
            </a>
          </p>
        </div>
      </div>
    </footer>
  )
}

function FooterColumn({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div>
      <h2 className="text-[13px] font-medium text-foreground">{title}</h2>
      <div className="mt-4 flex flex-col gap-3 text-sm">{children}</div>
    </div>
  )
}
