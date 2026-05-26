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
    <section className="relative overflow-hidden bg-warm-gradient">
      <div className="mx-auto max-w-6xl px-6 pt-20 pb-16 md:pt-28 md:pb-20">
        <IntroSection className="mx-auto max-w-3xl text-center">
          {release && (
            <p className="mb-4 text-sm font-medium text-muted-foreground">
              Latest release{' '}
              <span className="tabular-nums rounded-full border border-border bg-card px-2.5 py-0.5 font-mono text-xs text-foreground">
                v{release.version}
              </span>
            </p>
          )}
          <h1 className="text-balance text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl">
            A quiet place to read markdown
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-balance text-lg text-muted-foreground">
            Open folders, browse with a file tree, and read beautifully rendered markdown — with
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
        <IntroSection delay={1} className="mx-auto mt-16 max-w-5xl">
          <Screenshot
            name="reading-dark"
            alt="Mdow rendering a markdown document with syntax highlighting and a Mermaid diagram in dark mode"
            priority
            className="rounded-xl shadow-soft-lg ring-1 ring-border/50"
          />
        </IntroSection>
      </div>
    </section>
  )
}
