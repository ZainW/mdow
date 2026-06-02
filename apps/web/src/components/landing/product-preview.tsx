import { GradientSection } from '~/components/gradient-section'
import { Screenshot } from './screenshot'

export function LandingProductPreview() {
  return (
    <GradientSection variant="surface" innerClassName="text-center">
      <h2 className="mx-auto max-w-2xl text-balance text-3xl font-semibold tracking-tight sm:text-4xl">
        Built for long reading sessions
      </h2>
      <p className="mx-auto mt-4 max-w-xl text-balance text-muted-foreground">
        Whether you're skimming notes or diving into a long document, mdow keeps things comfortable
        at any time of day, with tabs, outline navigation, and typography tuned for reading.
      </p>
      <div className="relative mx-auto mt-12 max-w-5xl">
        <div className="grid gap-5 md:grid-cols-2">
          <div className="overflow-hidden rounded-lg shadow-elevated">
            <Screenshot
              name="reading-light"
              alt="Mdow in light mode with clean white background"
              className="h-auto w-full"
            />
          </div>
          <div className="overflow-hidden rounded-lg shadow-elevated md:mt-8">
            <Screenshot
              name="sidebar-dark"
              alt="Mdow in dark mode with folder sidebar and tabbed documents"
              className="h-auto w-full"
            />
          </div>
        </div>
      </div>
    </GradientSection>
  )
}
