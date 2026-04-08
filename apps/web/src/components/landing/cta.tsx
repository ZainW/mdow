import { Link } from '@tanstack/react-router'
import { GradientSection } from '~/components/gradient-section'

export function LandingCta() {
  return (
    <GradientSection variant="surface" innerClassName="text-center">
      <h2 className="mx-auto max-w-2xl text-3xl font-bold tracking-tight sm:text-4xl text-balance">
        Ready to read markdown beautifully?
      </h2>
      <p className="mx-auto mt-4 max-w-xl text-muted-foreground text-balance">
        Download mdow and turn any folder of markdown into a calm reading experience.
      </p>
      <div className="mt-10 flex justify-center">
        <Link
          to="/download"
          className="inline-flex h-12 items-center rounded-lg bg-primary px-8 text-sm font-medium text-primary-foreground shadow-soft transition-all hover:-translate-y-0.5 hover:bg-primary/95 hover:shadow-soft-lg"
        >
          Download for free
        </Link>
      </div>
      <p className="mt-4 text-xs text-muted-foreground">Available for macOS, Windows, and Linux</p>
    </GradientSection>
  )
}
