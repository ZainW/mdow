import { createFileRoute, Link } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { RssIcon } from '~/components/icons'
import { getReleases } from '~/lib/changelog'
import { formatReleaseDate } from '~/lib/release-format'
import { RELEASES_URL } from '~/lib/site'
import { canonical, seo } from '~/lib/seo'
import { cn } from '~/lib/utils'

const fetchChangelog = createServerFn({ method: 'GET' }).handler(async () => {
  const { renderToHtml, init } = await import('md4x')
  await init()
  return getReleases().map((release) => ({
    version: release.version,
    anchor: release.anchor,
    date: release.date,
    html: renderToHtml(release.markdown),
  }))
})

export const Route = createFileRoute('/changelog')({
  loader: () => fetchChangelog(),
  head: () => ({
    meta: seo({
      title: 'Changelog — Mdow',
      description: 'Every Mdow release, newest first: new features, fixes, and improvements.',
      path: '/changelog',
    }),
    links: [canonical('/changelog')],
  }),
  component: ChangelogPage,
})

function ChangelogPage() {
  const releases = Route.useLoaderData()

  return (
    <div className="mx-auto max-w-5xl px-5 pb-24 pt-14 sm:px-6 md:pt-20">
      <header className="max-w-2xl">
        <p className="eyebrow">Changelog</p>
        <h1 className="font-display mt-3 text-4xl sm:text-5xl">What&rsquo;s new in Mdow</h1>
        <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
          Every release, newest first. Mdow updates itself in the background, or you can{' '}
          <Link to="/download" className="link-underline text-foreground">
            grab the latest build
          </Link>
          .
        </p>
        <div className="mt-6 flex flex-wrap items-center gap-2">
          <a href="/changelog/rss.xml" className="btn btn-secondary btn-sm">
            <RssIcon className="size-4 text-accent" />
            Subscribe via RSS
          </a>
          <a
            href={RELEASES_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="btn btn-ghost btn-sm"
          >
            GitHub releases
          </a>
        </div>
      </header>

      <ol className="mt-16 md:mt-20">
        {releases.map((release, index) => (
          <li
            key={release.version}
            id={release.anchor}
            className={cn(
              'grid scroll-mt-24 gap-3 border-t border-border-subtle py-10 md:grid-cols-[12rem_1fr] md:gap-10',
              index === 0 && 'border-border',
            )}
          >
            <div className="md:sticky md:top-24 md:self-start">
              <div className="flex items-center gap-2.5">
                <a
                  href={`#${release.anchor}`}
                  className="font-mono text-[15px] font-medium tabular-nums text-foreground hover:underline hover:underline-offset-4"
                >
                  {release.version}
                </a>
                {index === 0 && (
                  <span className="rounded-full bg-accent/12 px-2 py-0.5 text-[11px] font-medium text-accent">
                    Latest
                  </span>
                )}
              </div>
              {release.date && (
                <time
                  dateTime={release.date}
                  className="mt-1 block text-sm tabular-nums text-muted-foreground"
                >
                  {formatReleaseDate(release.date, 'long')}
                </time>
              )}
            </div>
            <div
              className={cn(
                'prose max-w-[68ch] text-[15px]',
                'prose-ul:my-0 prose-li:my-1.5 prose-p:my-0',
              )}
              // Trusted: rendered server-side from our own changelog.md by md4x.
              dangerouslySetInnerHTML={{ __html: release.html }}
            />
          </li>
        ))}
      </ol>
    </div>
  )
}
