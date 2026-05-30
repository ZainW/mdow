import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getRequestHeader, setResponseHeader } from '@tanstack/react-start/server'
import { DownloadCard } from '~/components/download-card'
import { DownloadButton } from '~/components/download-button'
import { detectPlatform } from '~/lib/download-links'
import { fetchLatestRelease, type ReleaseInfo } from '~/lib/github-releases'
import { seo } from '~/lib/seo'

const REPO_RELEASES_URL = 'https://github.com/ZainW/mdow/releases'

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
      title: 'Download Mdow',
      description:
        'Download Mdow for Mac, Windows, or Linux. Free markdown viewer with syntax highlighting and Mermaid support.',
      image: '/og-image.png',
    }),
  }),
  component: DownloadPage,
})

function DownloadPage() {
  const { os, release } = Route.useLoaderData() as {
    os: 'mac' | 'windows' | 'linux'
    release: ReleaseInfo | null
  }

  if (!release) {
    return (
      <div className="mx-auto max-w-4xl px-6 py-16">
        <div className="mb-12 text-center">
          <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Download Mdow</h1>
          <p className="mt-3 text-muted-foreground">
            Downloads are temporarily unavailable.{' '}
            <a className="underline hover:text-foreground" href={REPO_RELEASES_URL}>
              Browse all releases on GitHub
            </a>
            .
          </p>
        </div>
      </div>
    )
  }

  const platforms = buildPlatforms(release)
  const sorted = [...platforms].sort((a, b) => (a.id === os ? -1 : b.id === os ? 1 : 0))
  const recommended = sorted.find((p) => p.id === os)
  const primaryUrl = recommended?.formats[0]?.url

  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      <div className="mb-10 text-center">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Download Mdow</h1>
        <p className="mt-3 text-muted-foreground">
          Version <span className="tabular-nums">{release.version}</span> · released{' '}
          {new Date(release.publishedAt).toLocaleDateString(undefined, {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
          })}
        </p>
        {primaryUrl && (
          <DownloadButton href={primaryUrl} size="lg" className="mt-6">
            Download for {recommended?.platform ?? 'your platform'}
          </DownloadButton>
        )}
      </div>
      <div className="grid gap-6 sm:grid-cols-3">
        {sorted.map((p) => (
          <DownloadCard
            key={p.id}
            platform={p.platform}
            icon={p.icon}
            formats={p.formats}
            recommended={p.id === os}
            note={p.note}
          />
        ))}
      </div>
      <p className="mt-8 text-center text-sm text-muted-foreground">
        <a className="underline hover:text-foreground" href={REPO_RELEASES_URL}>
          View all releases on GitHub
        </a>
        {' · '}
        <a
          className="underline hover:text-foreground"
          href="https://github.com/ZainW/mdow"
          target="_blank"
          rel="noopener noreferrer"
        >
          Source code
        </a>
      </p>
    </div>
  )
}

interface PlatformBlock {
  id: 'mac' | 'windows' | 'linux'
  platform: string
  icon: string
  formats: { label: string; url: string }[]
  note?: string
}

function buildPlatforms(release: ReleaseInfo): PlatformBlock[] {
  const macFormats = [
    ...release.assets.mac.dmg.map((a) => ({
      label: a.arch ? `Download .dmg (${a.arch})` : 'Download .dmg',
      url: a.url,
    })),
    ...release.assets.mac.zip.map((a) => ({
      label: a.arch ? `Download .zip (${a.arch})` : 'Download .zip',
      url: a.url,
    })),
  ]

  return [
    {
      id: 'mac',
      platform: 'macOS',
      icon: '\u{1F4BB}',
      formats: macFormats,
    },
    {
      id: 'windows',
      platform: 'Windows',
      icon: '\u{1FAA9}',
      formats: release.assets.windows.exe
        ? [{ label: 'Download installer (.exe)', url: release.assets.windows.exe }]
        : [],
    },
    {
      id: 'linux',
      platform: 'Linux',
      icon: '\u{1F427}',
      formats: release.assets.linux.appImage
        ? [{ label: 'Download .AppImage', url: release.assets.linux.appImage }]
        : [],
    },
  ]
}
