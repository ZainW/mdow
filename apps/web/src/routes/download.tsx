import { createFileRoute, Link } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getRequestHeader, setResponseHeader } from '@tanstack/react-start/server'
import { DownloadButton, PlatformIcon } from '~/components/download-button'
import { ArrowRightIcon } from '~/components/icons'
import { PlatformDownloadRow, type DownloadFormat } from '~/components/platform-download-row'
import { formatReleaseDate, versionAnchor } from '~/lib/release-format'
import { detectPlatform, LINUX_PREVIEW_URL, type PlatformId } from '~/lib/download-links'
import { fetchLatestRelease, type ReleaseInfo } from '~/lib/github-releases'
import { GITHUB_URL, RELEASES_URL } from '~/lib/site'
import { canonical, seo } from '~/lib/seo'

const loadDownloadData = createServerFn({ method: 'GET' }).handler(async () => {
  const ua = getRequestHeader('user-agent') || ''
  const os = detectPlatform(ua)
  const release = await fetchLatestRelease()

  setResponseHeader(
    'Cache-Control',
    release ? 'public, max-age=600, s-maxage=600' : 'public, max-age=30, s-maxage=30',
  )

  return { os, release }
})

export const Route = createFileRoute('/download')({
  loader: () => loadDownloadData(),
  head: () => ({
    meta: seo({
      title: 'Download Mdow for Mac',
      description:
        'Download Mdow, the free and open source markdown reader: a native Mac app built in Swift, with a preview of Mdow for Linux.',
      path: '/download',
    }),
    links: [canonical('/download')],
  }),
  component: DownloadPage,
})

interface PlatformBlock {
  id: PlatformId
  platform: string
  description: string
  formats: DownloadFormat[]
}

function buildPlatforms(release: ReleaseInfo): PlatformBlock[] {
  return [
    {
      id: 'mac',
      platform: 'macOS',
      description: 'Apple Silicon · macOS 15 or newer · unzip and move to Applications',
      formats: release.assets.mac
        ? [{ label: 'Archive', detail: '.zip', url: release.assets.mac }]
        : [],
    },
    {
      id: 'linux',
      platform: 'Linux',
      description: 'Preview · x64 · AppImage or Flatpak from the latest pre-release',
      formats: [{ label: 'Pre-release', detail: 'GitHub', url: LINUX_PREVIEW_URL }],
    },
  ]
}

function DownloadPage() {
  const { os, release } = Route.useLoaderData() as {
    os: PlatformId
    release: ReleaseInfo | null
  }

  const platforms = release ? buildPlatforms(release) : []
  const primary = release?.assets.mac

  return (
    <div className="mx-auto max-w-4xl px-5 pb-24 pt-14 sm:px-6 md:pt-20">
      <header className="text-center">
        <p className="eyebrow">Download</p>
        <h1 className="font-display mt-3 text-5xl sm:text-6xl">Get Mdow</h1>
        {release ? (
          <p className="mt-4 text-muted-foreground">
            Version{' '}
            <span className="font-mono tabular-nums text-foreground">{release.version}</span> ·
            Released {formatReleaseDate(release.publishedAt, 'long')} ·{' '}
            <Link
              to="/changelog"
              hash={versionAnchor(`v${release.version}`)}
              className="link-underline text-foreground"
            >
              What&rsquo;s new
            </Link>
          </p>
        ) : (
          <p className="mx-auto mt-4 max-w-md text-muted-foreground">
            We couldn&rsquo;t reach GitHub just now.{' '}
            <a href={RELEASES_URL} className="link-underline text-foreground">
              Download from GitHub Releases
            </a>{' '}
            instead.
          </p>
        )}
        {primary && (
          <div className="mt-9 flex flex-col items-center gap-3">
            <DownloadButton href={primary} platform="mac" size="lg">
              Download for macOS
            </DownloadButton>
            <p className="text-[13px] text-muted-foreground">
              Apple Silicon · Free and open source
            </p>
          </div>
        )}
      </header>

      {release && (
        <section className="mt-16" aria-labelledby="all-platforms">
          <h2 id="all-platforms" className="text-xl font-semibold tracking-tight">
            All platforms
          </h2>
          <div className="surface-card mt-6 divide-y divide-border-subtle overflow-hidden rounded-2xl">
            {platforms.map((p) => (
              <PlatformDownloadRow
                key={p.id}
                icon={<PlatformIcon platform={p.id} className="size-5" />}
                platform={p.platform}
                description={p.description}
                formats={p.formats}
                highlighted={p.id === os}
              />
            ))}
          </div>
          <p className="mt-4 text-sm text-muted-foreground">
            On Mac, Mdow checks for updates in the background.{' '}
            <Link
              to="/docs/$"
              params={{ _splat: 'installation' }}
              className="link-underline text-foreground"
            >
              Installation guide
            </Link>
          </p>
        </section>
      )}

      <div className="mt-16 grid gap-4 sm:grid-cols-2">
        <a
          href={RELEASES_URL}
          target="_blank"
          rel="noopener noreferrer"
          className="group surface-card flex items-center justify-between rounded-2xl px-6 py-5"
        >
          <span>
            <span className="block font-medium">Previous releases</span>
            <span className="mt-0.5 block text-sm text-muted-foreground">
              Every build on GitHub Releases
            </span>
          </span>
          <ArrowRightIcon className="size-4 text-muted-foreground transition-transform duration-200 ease-out group-hover:translate-x-0.5" />
        </a>
        <a
          href={GITHUB_URL}
          target="_blank"
          rel="noopener noreferrer"
          className="group surface-card flex items-center justify-between rounded-2xl px-6 py-5"
        >
          <span>
            <span className="block font-medium">Build from source</span>
            <span className="mt-0.5 block text-sm text-muted-foreground">
              MIT licensed on GitHub
            </span>
          </span>
          <ArrowRightIcon className="size-4 text-muted-foreground transition-transform duration-200 ease-out group-hover:translate-x-0.5" />
        </a>
      </div>
    </div>
  )
}
