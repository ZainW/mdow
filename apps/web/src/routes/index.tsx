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
