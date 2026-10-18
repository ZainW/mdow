import { Link } from '@tanstack/react-router'
import { AppleIcon } from './icons'
import { PlatformDownloadRow } from './platform-download-row'
import { formatReleaseDate } from '~/lib/release-format'

export interface MacAppDownload {
  url: string
  version: string | null
  publishedAt: string | null
}

/** Mdow for Mac: the Swift app, offered next to the regular downloads. */
export function MacAppDownloadSection({ download }: { download: MacAppDownload }) {
  const released =
    download.version && download.publishedAt
      ? `Version ${download.version} · ${formatReleaseDate(download.publishedAt, 'long')}`
      : null
  return (
    <section className="mt-16" aria-labelledby="mac-app-heading">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div className="max-w-xl">
          <div className="flex items-center gap-2.5">
            <h2 id="mac-app-heading" className="text-xl font-semibold tracking-tight">
              Mdow for Mac
            </h2>
            <span className="rounded-full bg-accent/12 px-2 py-0.5 text-[11px] font-medium text-accent">
              New
            </span>
          </div>
          <p className="mt-2 text-[15px] leading-relaxed text-muted-foreground">
            Mdow rebuilt in Swift on AppKit and TextKit, with no web view and a calm paper-and-ink
            look. Tabs, the outline, Quick Open, Mermaid, math and the OpenCode companion all come
            along. Runs alongside the regular Mdow app.
          </p>
        </div>
        <Link
          to="/docs/$"
          params={{ _splat: 'installation' }}
          hash="mdow-for-mac"
          className="link-underline shrink-0 text-sm text-muted-foreground"
        >
          Install notes
        </Link>
      </div>
      <div className="surface-card mt-6 overflow-hidden rounded-2xl">
        <div className="border-b border-border-subtle bg-surface px-4 pt-4 sm:px-8 sm:pt-8">
          <MacAppScreenshot />
        </div>
        <PlatformDownloadRow
          icon={<AppleIcon className="size-5" />}
          platform="macOS"
          description={['Apple Silicon · macOS 15 or newer', released].filter(Boolean).join(' · ')}
          formats={[{ label: 'Mdow for Mac', detail: '.zip', url: download.url }]}
        />
      </div>
    </section>
  )
}

function MacAppScreenshot() {
  const alt =
    'Mdow for Mac showing a document on a paper card, with tabs in the header and the outline in the sidebar'
  return (
    <>
      <MacAppImage theme="light" alt={alt} className="block dark:hidden" />
      <MacAppImage theme="dark" alt={alt} className="hidden dark:block" />
    </>
  )
}

function MacAppImage({
  theme,
  alt,
  className,
}: {
  theme: 'light' | 'dark'
  alt: string
  className: string
}) {
  return (
    <img
      src={`/screenshots/mac-app-${theme}.webp`}
      alt={alt}
      width={1600}
      height={1114}
      loading="lazy"
      decoding="async"
      className={`${className} h-auto w-full rounded-t-xl shadow-[0_8px_30px_-12px_rgb(0_0_0/0.25)]`}
    />
  )
}
