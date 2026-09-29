import { Link } from '@tanstack/react-router'
import { DownloadButton } from '~/components/download-button'
import { AppleIcon, GitHubIcon, LinuxIcon, WindowsIcon } from '~/components/icons'
import { Logo } from '~/components/logo'
import { btnPrimary, btnSecondary } from '~/lib/button-styles'
import { downloadButtonLabel, type PlatformId } from '~/lib/download-links'
import { GITHUB_URL } from '~/lib/site'

interface LandingCtaProps {
  platform: PlatformId
  downloadUrl: string | null
}

export function LandingCta({ platform, downloadUrl }: LandingCtaProps) {
  return (
    <section className="px-5 pb-20 sm:px-6 md:pb-28">
      <div className="surface-card relative mx-auto max-w-6xl overflow-hidden rounded-3xl px-6 py-16 text-center sm:py-20">
        <div aria-hidden className="bg-paper absolute inset-0 -z-0 opacity-70" />
        <div className="relative">
          <Logo className="mx-auto size-14" alt="" />
          <h2 className="font-display mx-auto mt-8 max-w-2xl text-4xl leading-[1.08] sm:text-5xl">
            Open something worth reading.
          </h2>
          <p className="mx-auto mt-4 max-w-md text-lg leading-relaxed text-muted-foreground">
            Free and open source. No account, no setup, just your files.
          </p>
          <div className="mt-9 flex flex-col items-center justify-center gap-3 sm:flex-row">
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
            <a
              href={GITHUB_URL}
              className={btnSecondary('lg', 'w-full max-w-72 sm:w-auto')}
              target="_blank"
              rel="noopener noreferrer"
            >
              <GitHubIcon className="size-[17px]" />
              Star on GitHub
            </a>
          </div>
          <div className="mt-8 flex items-center justify-center gap-5 text-[13px] text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <AppleIcon className="size-3.5" /> macOS
            </span>
            <span className="flex items-center gap-1.5">
              <WindowsIcon className="size-3.5" /> Windows
            </span>
            <span className="flex items-center gap-1.5">
              <LinuxIcon className="size-3.5" /> Linux
            </span>
          </div>
        </div>
      </div>
    </section>
  )
}
