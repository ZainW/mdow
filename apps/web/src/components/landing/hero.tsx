import { Link } from '@tanstack/react-router'
import { ArrowRightIcon } from '~/components/icons'
import { IntroSection } from '~/components/landing/intro-section'
import { btnSecondary } from '~/lib/button-styles'
import type { ReleaseInfo } from '~/lib/github-releases'
import { AppWindow } from './app-window'
import { DownloadCta, OtherPlatformLink } from './download-cta'

interface LandingHeroProps {
  release: ReleaseInfo | null
  latest: { version: string; anchor: string; headline: string } | null
}

const PROOF = ['Written in Rust', 'Drawn on the GPU', 'No browser engine', 'Free and open source']

export function LandingHero({ release, latest }: LandingHeroProps) {
  return (
    <section className="relative isolate overflow-hidden">
      <div
        aria-hidden
        className="bg-paper absolute inset-x-0 top-0 -z-10 h-[52rem] [mask-image:radial-gradient(ellipse_70%_80%_at_50%_0%,black_40%,transparent)]"
      />
      <div className="mx-auto max-w-6xl px-5 pb-20 pt-14 sm:px-6 md:pb-28 md:pt-24">
        <IntroSection className="mx-auto max-w-3xl text-center">
          {latest && (
            <Link
              to="/changelog"
              hash={latest.anchor}
              className="group mb-8 inline-flex max-w-full items-center gap-2 rounded-full border border-border bg-card/80 py-1 pl-1 pr-3 text-[13px] text-muted-foreground shadow-[0_1px_2px_hsl(var(--shadow-color)/0.05)] backdrop-blur transition-colors duration-150 ease hover:text-foreground"
            >
              <span className="shrink-0 rounded-full bg-accent/12 px-2 py-0.5 font-mono text-[11.5px] font-medium tabular-nums text-accent">
                {latest.version}
              </span>
              <span className="truncate">{latest.headline}</span>
              <ArrowRightIcon className="size-3.5 shrink-0 transition-transform duration-200 ease-out group-hover:translate-x-0.5" />
            </Link>
          )}
          <h1 className="font-display text-balance text-[2.75rem] leading-[1.02] sm:text-6xl md:text-7xl md:leading-[0.98]">
            Your markdown,
            <br />
            <em className="italic">beautifully</em> read.
          </h1>
          <p className="mx-auto mt-7 max-w-xl text-pretty text-lg leading-relaxed text-muted-foreground sm:text-xl sm:leading-relaxed">
            Mdow is a native markdown reader for macOS and Linux. Diagrams, math and code show up
            the instant you open a file. When you have a question, Claude, Codex, Cursor or OpenCode
            can answer it from your docs.
          </p>
          <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
            <DownloadCta release={release} className="w-full max-w-72 sm:w-auto" />
            <Link to="/docs" className={btnSecondary('lg', 'w-full max-w-72 sm:w-auto')}>
              Read the docs
            </Link>
          </div>
          <p className="mt-5 text-[13px] text-muted-foreground">
            {release ? `v${release.version} · ` : ''}
            <OtherPlatformLink className="link-underline" />
          </p>
        </IntroSection>

        <IntroSection delay={1} className="relative mx-auto mt-16 max-w-5xl md:mt-20">
          <div
            aria-hidden
            className="hero-glow pointer-events-none absolute -inset-x-16 -top-10 bottom-0 -z-10"
          />
          <div className="hero-tilt">
            <AppWindow
              name="reading"
              priority
              alt="Mdow showing a sync engine architecture document with a folder sidebar, tabs, a Mermaid flowchart, and a highlighted TypeScript code block"
            />
          </div>
        </IntroSection>

        <IntroSection delay={2}>
          <ul
            className="mx-auto mt-12 flex max-w-3xl flex-wrap items-center justify-center gap-x-6 gap-y-3 text-[13px] text-muted-foreground"
            aria-label="What Mdow is built on"
          >
            {PROOF.map((p) => (
              <li key={p} className="flex items-center gap-2">
                <span aria-hidden className="size-1 rounded-full bg-accent" />
                {p}
              </li>
            ))}
          </ul>
        </IntroSection>
      </div>
    </section>
  )
}
