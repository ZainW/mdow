import { Link } from '@tanstack/react-router'
import { AppleIcon, GitHubIcon, LinuxIcon } from '~/components/icons'
import { Logo } from '~/components/logo'
import { btnSecondary } from '~/lib/button-styles'
import type { ReleaseInfo } from '~/lib/github-releases'
import { GITHUB_URL } from '~/lib/site'
import { DownloadCta } from './download-cta'

interface LandingCtaProps {
  release: ReleaseInfo | null
}

export function LandingCta({ release }: LandingCtaProps) {
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
            Free and open source under MIT. No account, no setup, just your files.
          </p>
          <div className="mt-9 flex flex-col items-center justify-center gap-3 sm:flex-row">
            <DownloadCta release={release} className="w-full max-w-72 sm:w-auto" />
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
            <Link to="/download" className="flex items-center gap-1.5 hover:text-foreground">
              <AppleIcon className="size-3.5" /> macOS
            </Link>
            <Link to="/download" className="flex items-center gap-1.5 hover:text-foreground">
              <LinuxIcon className="size-3.5" /> AppImage and Flatpak
            </Link>
          </div>
        </div>
      </div>
    </section>
  )
}
