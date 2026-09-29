import { Link } from '@tanstack/react-router'
import { formatReleaseDate } from '~/lib/release-format'
import { ArrowRightIcon, RssIcon } from '../icons'

export interface LatestReleaseData {
  version: string
  anchor: string
  date: string | null
  /** Pre-rendered HTML for a short list of the release's highlights. */
  highlightsHtml: string
  total: number
}

export function LandingLatestRelease({ release }: { release: LatestReleaseData }) {
  return (
    <section className="py-20 md:py-28">
      <div className="mx-auto grid max-w-6xl gap-12 px-5 sm:px-6 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:gap-16">
        <div>
          <p className="eyebrow">Changelog</p>
          <h2 className="font-display mt-3 text-4xl leading-[1.08] sm:text-5xl">
            Quietly getting better.
          </h2>
          <p className="mt-5 text-lg leading-relaxed text-muted-foreground">
            Mdow ships small, careful releases and updates itself in the background. Every change is
            written down.
          </p>
          <div className="mt-8 flex flex-wrap gap-2">
            <Link to="/changelog" className="btn btn-secondary btn-md">
              Read the changelog
            </Link>
            <a href="/changelog/rss.xml" className="btn btn-ghost btn-md">
              <RssIcon className="size-4 text-accent" />
              RSS
            </a>
          </div>
        </div>

        <article className="surface-card rounded-2xl p-6 sm:p-8">
          <header className="flex flex-wrap items-baseline justify-between gap-2 border-b border-border-subtle pb-5">
            <div className="flex items-center gap-2.5">
              <span className="font-mono text-lg font-medium tabular-nums">{release.version}</span>
              <span className="rounded-full bg-accent/12 px-2 py-0.5 text-[11px] font-medium text-accent">
                Latest
              </span>
            </div>
            {release.date && (
              <time dateTime={release.date} className="text-sm tabular-nums text-muted-foreground">
                {formatReleaseDate(release.date, 'long')}
              </time>
            )}
          </header>
          <div
            className="prose mt-5 max-w-none text-[15px] prose-p:my-0 prose-ul:my-0 prose-li:my-2"
            // Trusted: rendered server-side from our own changelog.md by md4x.
            dangerouslySetInnerHTML={{ __html: release.highlightsHtml }}
          />
          {release.total > 0 && (
            <Link
              to="/changelog"
              hash={release.anchor}
              className="group mt-6 inline-flex items-center gap-1.5 text-sm font-medium"
            >
              See all {release.total} changes in {release.version}
              <ArrowRightIcon className="size-3.5 transition-transform duration-200 ease-out group-hover:translate-x-0.5" />
            </Link>
          )}
        </article>
      </div>
    </section>
  )
}
