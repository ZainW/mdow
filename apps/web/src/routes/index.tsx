import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getRequestHeader, setResponseHeader } from '@tanstack/react-start/server'
import { LandingHero } from '~/components/landing/hero'
import { LandingFeatures } from '~/components/landing/features'
import { LandingHighlights } from '~/components/landing/highlights'
import { LandingProductPreview } from '~/components/landing/product-preview'
import { LandingTrust } from '~/components/landing/trust'
import { LandingCta } from '~/components/landing/cta'
import { DownloadBar } from '~/components/landing/download-bar'
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

  return {
    platform,
    release,
    downloadUrl: release ? primaryDownloadUrl(release, platform) : null,
  }
})

export const Route = createFileRoute('/')({
  loader: () => loadHomeData(),
  head: ({ loaderData }) => ({
    meta: seo({
      title: 'Mdow: AI Markdown Reader for Mac, Windows & Linux',
      description:
        'Read markdown and ask questions about local files with OpenCode and ACP. Free for Mac, Windows, and Linux, with Mermaid and Shiki built in.',
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
  const { platform, release, downloadUrl } = Route.useLoaderData()

  return (
    <>
      <LandingHero platform={platform} release={release} downloadUrl={downloadUrl} />
      <LandingFeatures />
      <LandingHighlights />
      <LandingProductPreview />
      <LandingTrust />
      <LandingCta platform={platform} downloadUrl={downloadUrl} />
      <DownloadBar platform={platform} release={release} downloadUrl={downloadUrl} />
    </>
  )
}
