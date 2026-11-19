import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { setResponseHeader } from '@tanstack/react-start/server'
import { LandingHero } from '~/components/landing/hero'
import { LandingFeatures } from '~/components/landing/features'
import { LandingEngine } from '~/components/landing/engine'
import { LandingThemeShowcase } from '~/components/landing/theme-showcase'
import { LandingReaderSection } from '~/components/landing/reader-section'
import { LandingLatestRelease } from '~/components/landing/latest-release'
import { LandingCta } from '~/components/landing/cta'
import { DownloadBar } from '~/components/landing/download-bar'
import { getReleases, releaseHeadline } from '~/lib/changelog'
import { fetchLatestRelease } from '~/lib/github-releases'
import { absoluteUrl, canonical, jsonLd, seo, SITE_URL } from '~/lib/seo'

const loadHomeData = createServerFn({ method: 'GET' }).handler(async () => {
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

  return { release, latestRelease }
})

export const Route = createFileRoute('/')({
  loader: () => loadHomeData(),
  head: ({ loaderData }) => ({
    meta: seo({
      title: 'Mdow: A Native Markdown Reader for macOS and Linux',
      description:
        'A fast, GPU-rendered markdown reader for your notes, docs and READMEs, written in Rust. Native Mermaid, math and code, your Omarchy theme, and answers from Claude, Codex, Cursor or OpenCode. Free and open source.',
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
            'A native, GPU-rendered markdown reader for macOS and Linux, with a companion that runs Claude Code, Codex, Cursor or OpenCode.',
          applicationCategory: 'UtilitiesApplication',
          operatingSystem: 'macOS, Linux',
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
            'Companion chat with Claude Code, Codex, Cursor and OpenCode',
            'Clickable source citations and reviewable edits',
            'GPU-rendered markdown and local HTML reading',
            'Native Mermaid diagrams and LaTeX math',
            'Syntax highlighting',
            'Omarchy theme support',
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
  const { release, latestRelease } = Route.useLoaderData()

  return (
    <>
      <LandingHero release={release} latest={latestRelease} />
      <LandingEngine />
      <LandingFeatures />
      <LandingThemeShowcase />
      <LandingReaderSection />
      {latestRelease && <LandingLatestRelease release={latestRelease} />}
      <LandingCta release={release} />
      <DownloadBar release={release} />
    </>
  )
}
