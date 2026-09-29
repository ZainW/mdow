import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getRequestHeader, setResponseHeader } from '@tanstack/react-start/server'
import { LandingHero } from '~/components/landing/hero'
import { LandingFeatures } from '~/components/landing/features'
import { LandingReaderSection } from '~/components/landing/reader-section'
import { LandingLatestRelease } from '~/components/landing/latest-release'
import { LandingCta } from '~/components/landing/cta'
import { DownloadBar } from '~/components/landing/download-bar'
import { getReleases, releaseHeadline } from '~/lib/changelog'
import { detectPlatform, primaryDownloadUrl } from '~/lib/download-links'
import { fetchLatestRelease } from '~/lib/github-releases'
import { absoluteUrl, canonical, jsonLd, seo, SITE_URL } from '~/lib/seo'

const loadHomeData = createServerFn({ method: 'GET' }).handler(async () => {
  const ua = getRequestHeader('user-agent') || ''
  const platform = detectPlatform(ua)
  const release = await fetchLatestRelease()

  setResponseHeader(
    'Cache-Control',
    release ? 'public, max-age=600, s-maxage=600' : 'public, max-age=30, s-maxage=30',
  )

  const [latest] = getReleases()
  let latestRelease = null
  if (latest) {
    const { renderToHtml, init } = await import('md4x')
    await init()
    const highlights = latest.bullets.slice(0, 4)
    latestRelease = {
      version: latest.version,
      anchor: latest.anchor,
      date: latest.date,
      headline: releaseHeadline(latest),
      highlightsHtml: renderToHtml(highlights.map((b) => `- ${b}`).join('\n')),
      total: latest.bullets.length,
    }
  }

  return {
    platform,
    release,
    downloadUrl: release ? primaryDownloadUrl(release, platform) : null,
    latestRelease,
  }
})

export const Route = createFileRoute('/')({
  loader: () => loadHomeData(),
  head: ({ loaderData }) => ({
    meta: seo({
      title: 'Mdow: A Quiet Markdown Reader for Mac, Windows & Linux',
      description:
        'A fast, focused markdown reader for your notes, docs, and READMEs. Browse folders, render Mermaid and Shiki, and ask questions with OpenCode or Codex. Free and open source.',
    }),
    links: [canonical('/')],
    scripts: [
      {
        type: 'application/ld+json',
        children: jsonLd({
          '@context': 'https://schema.org',
          '@type': 'SoftwareApplication',
          name: 'Mdow',
          url: SITE_URL,
          image: absoluteUrl('/og-image.png'),
          description:
            'A cross-platform markdown reader with local AI chat through OpenCode and ACP.',
          applicationCategory: 'UtilitiesApplication',
          operatingSystem: 'macOS, Windows, Linux',
          softwareVersion: loaderData?.release?.version,
          downloadUrl: absoluteUrl('/download'),
          isAccessibleForFree: true,
          license: 'https://opensource.org/license/mit',
          offers: {
            '@type': 'Offer',
            price: '0',
            priceCurrency: 'USD',
          },
          featureList: [
            'Local AI chat through OpenCode and ACP',
            'Clickable source citations',
            'Markdown and local HTML reading',
            'Shiki syntax highlighting',
            'Mermaid diagrams',
            'Folder browsing and document outlines',
          ],
          sameAs: ['https://github.com/ZainW/mdow'],
          author: {
            '@type': 'Person',
            name: 'Zain Wania',
            url: 'https://zainwania.dev',
          },
        }),
      },
    ],
  }),
  component: HomePage,
})

function HomePage() {
  const { platform, release, downloadUrl, latestRelease } = Route.useLoaderData()

  return (
    <>
      <LandingHero
        platform={platform}
        release={release}
        downloadUrl={downloadUrl}
        latest={latestRelease}
      />
      <LandingFeatures />
      <LandingReaderSection />
      {latestRelease && <LandingLatestRelease release={latestRelease} />}
      <LandingCta platform={platform} downloadUrl={downloadUrl} />
      <DownloadBar platform={platform} release={release} downloadUrl={downloadUrl} />
    </>
  )
}
