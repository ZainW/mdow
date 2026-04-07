import { BrowserFrame } from '~/components/browser-frame'
import { GradientSection } from '~/components/gradient-section'
import { MockMarkdown } from './mock-markdown'

export function LandingProductPreview() {
  return (
    <GradientSection variant="surface" innerClassName="text-center">
      <h2 className="mx-auto max-w-2xl text-3xl font-bold tracking-tight sm:text-4xl text-balance">
        Built for long reading sessions
      </h2>
      <p className="mx-auto mt-4 max-w-xl text-muted-foreground text-balance">
        Whether you're skimming notes or diving into a long-form document, mdow keeps things
        comfortable, no matter the time of day.
      </p>
      <div className="relative mx-auto mt-14 max-w-5xl">
        <div className="grid gap-6 md:grid-cols-2 md:gap-4">
          <div className="md:translate-y-4">
            <BrowserFrame title="readme.md — Light">
              <div className="bg-[oklch(0.98_0.005_70)] text-[oklch(0.13_0.02_50)]">
                <MockMarkdown />
              </div>
            </BrowserFrame>
          </div>
          <div className="md:-translate-y-4">
            <BrowserFrame title="readme.md — Dark">
              <div className="bg-[oklch(0.14_0_0)] text-[oklch(0.92_0_0)]">
                <MockMarkdown />
              </div>
            </BrowserFrame>
          </div>
        </div>
      </div>
    </GradientSection>
  )
}
