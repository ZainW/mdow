import { createFileRoute } from '@tanstack/react-router'
import { LandingHero } from '~/components/landing/hero'
import { LandingFeatures } from '~/components/landing/features'
import { LandingProductPreview } from '~/components/landing/product-preview'
import { LandingTrust } from '~/components/landing/trust'
import { LandingCta } from '~/components/landing/cta'

export const Route = createFileRoute('/')({
  component: HomePage,
})

function HomePage() {
  return (
    <>
      <LandingHero />
      <LandingFeatures />
      <LandingProductPreview />
      <LandingTrust />
      <LandingCta />
    </>
  )
}
