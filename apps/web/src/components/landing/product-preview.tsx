import { GradientSection } from '~/components/gradient-section'
import { Screenshot } from './screenshot'

export function LandingProductPreview() {
  return (
    <GradientSection variant="surface" innerClassName="text-center">
      <h2 className="mx-auto max-w-2xl text-balance text-3xl font-bold tracking-tight sm:text-4xl">
        Built for long reading sessions
      </h2>
      <p className="mx-auto mt-4 max-w-xl text-balance text-muted-foreground">
        Whether you're skimming notes or diving into a long document, mdow keeps things comfortable,
        no matter the time of day.
      </p>
      <div className="relative mx-auto mt-14 max-w-5xl">
        <div className="grid gap-6 md:grid-cols-2 md:gap-4">
          <div className="md:translate-y-4">
            <Screenshot
              name="reading-light"
              alt="Mdow in light mode"
              className="rounded-xl shadow-soft-lg"
            />
          </div>
          <div className="md:-translate-y-4">
            <Screenshot
              name="reading-dark"
              alt="Mdow in dark mode"
              className="rounded-xl shadow-soft-lg"
            />
          </div>
        </div>
      </div>
    </GradientSection>
  )
}
