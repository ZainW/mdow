import { Link } from '@tanstack/react-router'
import { GradientSection } from '~/components/gradient-section'
import { DownloadButton } from '~/components/download-button'
import { downloadButtonLabel, type PlatformId } from '~/lib/download-links'
import { btnPrimaryClass, btnSecondaryClass } from '~/lib/button-styles'
import { cn } from '~/lib/utils'

const GITHUB_URL = 'https://github.com/ZainW/mdow'

interface LandingCtaProps {
  platform: PlatformId
  downloadUrl: string | null
}

export function LandingCta({ platform, downloadUrl }: LandingCtaProps) {
  return (
    <GradientSection innerClassName="text-center py-20 md:py-28">
      <div className="mx-auto max-w-2xl">
        <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl text-balance">
          Ready to read markdown beautifully?
        </h2>
        <p className="mx-auto mt-4 max-w-xl text-muted-foreground text-balance leading-relaxed">
          Download mdow and turn any folder of markdown into a calm reading experience.
        </p>
      </div>
      <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
        {downloadUrl ? (
          <DownloadButton href={downloadUrl} size="lg">
            {downloadButtonLabel(platform)}
          </DownloadButton>
        ) : (
          <Link to="/download" className={cn(btnPrimaryClass, 'h-12 px-8 text-sm font-medium')}>
            Download for free
          </Link>
        )}
        <a
          href={GITHUB_URL}
          className={cn(btnSecondaryClass, 'h-12 px-8 text-sm font-medium')}
          target="_blank"
          rel="noopener noreferrer"
        >
          View on GitHub
        </a>
      </div>
      <p className="mt-5 text-xs text-muted-foreground">
        Available for macOS, Windows, and Linux · MIT licensed
      </p>
    </GradientSection>
  )
}
