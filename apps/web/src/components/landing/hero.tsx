import { Link } from '@tanstack/react-router'
import { DownloadButton } from '~/components/download-button'
import { IntroSection } from '~/components/landing/intro-section'
import { downloadButtonLabel, type PlatformId } from '~/lib/download-links'
import type { ReleaseInfo } from '~/lib/github-releases'
import { btnPrimaryClass, btnSecondaryClass } from '~/lib/button-styles'
import { cn } from '~/lib/utils'
import { Screenshot } from './screenshot'

interface LandingHeroProps {
  platform: PlatformId
  release: ReleaseInfo | null
  downloadUrl: string | null
}

export function LandingHero({ platform, release, downloadUrl }: LandingHeroProps) {
  return (
    <section className="relative overflow-hidden bg-hero-gradient">
      <div className="mx-auto max-w-6xl px-6 pt-18 pb-14 md:pt-24 md:pb-20">
        <IntroSection className="mx-auto max-w-3xl text-center">
          {release && (
            <Link
              to="/changelog"
              className="mb-6 inline-flex items-center gap-2 rounded-md border border-border-subtle bg-card/85 px-2.5 py-1 text-xs font-medium text-muted-foreground shadow-soft backdrop-blur-sm transition-colors hover:text-foreground"
            >
              <span className="size-1.5 rounded-full bg-primary" />
              <span className="tabular-nums font-mono">v{release.version}</span>
              <span className="text-border">|</span>
              <span>What&apos;s new</span>
            </Link>
          )}
          <h1 className="text-balance text-4xl font-semibold tracking-tight sm:text-5xl md:text-6xl lg:text-[4rem] lg:leading-[1.08]">
            A quiet place to read markdown
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-balance text-lg leading-relaxed text-muted-foreground">
            Open folders, browse with a file tree, and read beautifully rendered markdown, with
            Shiki syntax highlighting, Mermaid diagrams, and a calm interface that stays out of your
            way.
          </p>
          <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
            {downloadUrl ? (
              <DownloadButton href={downloadUrl}>{downloadButtonLabel(platform)}</DownloadButton>
            ) : (
              <Link to="/download" className={cn(btnPrimaryClass, 'h-11 px-7 text-sm font-medium')}>
                Download for free
              </Link>
            )}
            <Link to="/docs" className={cn(btnSecondaryClass, 'h-11 px-7 text-sm font-medium')}>
              Read the docs
            </Link>
          </div>
          <p className="mt-4 text-xs text-muted-foreground">
            Free for macOS, Windows, and Linux ·{' '}
            <Link to="/download" className="underline underline-offset-2 hover:text-foreground">
              All download options
            </Link>
          </p>
        </IntroSection>
        <IntroSection delay={1} className="mx-auto mt-14 max-w-5xl">
          <div className="relative">
            <Screenshot
              name="reading-dark"
              alt="Mdow rendering a markdown document with syntax highlighting and a Mermaid diagram in dark mode"
              priority
              className="rounded-lg shadow-elevated"
            />
          </div>
        </IntroSection>
      </div>
    </section>
  )
}
