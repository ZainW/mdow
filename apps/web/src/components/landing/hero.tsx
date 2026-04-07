import { Link } from '@tanstack/react-router'
import { BrowserFrame } from '~/components/browser-frame'
import { MockMarkdown } from './mock-markdown'

export function LandingHero() {
  return (
    <section className="relative overflow-hidden bg-warm-gradient">
      <div className="mx-auto max-w-6xl px-6 pt-20 pb-16 md:pt-28 md:pb-24">
        <div className="mx-auto max-w-3xl text-center">
          <h1 className="text-4xl font-bold tracking-tight text-balance sm:text-5xl md:text-6xl">
            A quiet place to read markdown
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground text-balance">
            Beautiful rendering, syntax highlighting, and a reading experience that gets out of the
            way. Free for Mac, Windows, and Linux.
          </p>
          <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
            <Link
              to="/download"
              className="inline-flex h-11 items-center rounded-lg bg-primary px-7 text-sm font-medium text-primary-foreground shadow-soft transition-all hover:-translate-y-0.5 hover:bg-primary/95 hover:shadow-soft-lg"
            >
              Download for free
            </Link>
            <Link
              to="/docs"
              className="inline-flex h-11 items-center rounded-lg border border-border bg-card px-7 text-sm font-medium transition-colors hover:bg-muted"
            >
              Read the docs
            </Link>
          </div>
        </div>
        <div className="mx-auto mt-16 max-w-4xl">
          <BrowserFrame title="readme.md — Mdow">
            <MockMarkdown />
          </BrowserFrame>
        </div>
      </div>
    </section>
  )
}
