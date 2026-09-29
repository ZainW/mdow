import { Link } from '@tanstack/react-router'
import { DownloadButton } from '~/components/download-button'
import { ArrowRightIcon } from '~/components/icons'
import { IntroSection } from '~/components/landing/intro-section'
import { btnPrimary, btnSecondary } from '~/lib/button-styles'
import { downloadButtonLabel, type PlatformId } from '~/lib/download-links'
import type { ReleaseInfo } from '~/lib/github-releases'
import { AppWindow } from './app-window'

interface LandingHeroProps {
  platform: PlatformId
  release: ReleaseInfo | null
  downloadUrl: string | null
  latest: { version: string; anchor: string; headline: string } | null
}

export function LandingHero({ platform, release, downloadUrl, latest }: LandingHeroProps) {
  return (
    <section className="relative isolate overflow-hidden">
      <div
        aria-hidden
        className="bg-paper absolute inset-x-0 top-0 -z-10 h-[46rem] [mask-image:radial-gradient(ellipse_70%_80%_at_50%_0%,black_40%,transparent)]"
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
          <h1 className="font-display text-[2.75rem] leading-[1.02] sm:text-6xl md:text-7xl md:leading-[0.98]">
            A quiet place to
            <br />
            read <em className="italic">markdown</em>.
          </h1>
          <p className="mx-auto mt-7 max-w-xl text-lg leading-relaxed text-muted-foreground sm:text-xl sm:leading-relaxed">
            Mdow is a fast, focused reader for your notes, docs, and READMEs. Open a file or a whole
            folder, and ask questions about it with the AI tools you already use.
          </p>
          <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
            {downloadUrl ? (
              <DownloadButton
                href={downloadUrl}
                platform={platform}
                size="lg"
                className="w-full max-w-72 sm:w-auto"
              >
                {downloadButtonLabel(platform)}
              </DownloadButton>
            ) : (
              <Link to="/download" className={btnPrimary('lg', 'w-full max-w-72 sm:w-auto')}>
                Download for free
              </Link>
            )}
            <Link to="/docs" className={btnSecondary('lg', 'w-full max-w-72 sm:w-auto')}>
              Read the docs
            </Link>
          </div>
          <p className="mt-5 text-[13px] text-muted-foreground">
            Free and open source{release ? ` · v${release.version}` : ''} ·{' '}
            <Link to="/download" className="link-underline">
              Also for{' '}
              {platform === 'mac'
                ? 'Windows and Linux'
                : platform === 'windows'
                  ? 'macOS and Linux'
                  : 'macOS and Windows'}
            </Link>
          </p>
        </IntroSection>
        <IntroSection delay={1} className="mx-auto mt-16 max-w-5xl md:mt-20">
          <AppWindow
            name="reading"
            priority
            alt="Mdow showing a sync engine architecture document with a folder sidebar, tabs, a Mermaid flowchart, and a highlighted TypeScript code block"
          />
        </IntroSection>
      </div>
    </section>
  )
}
