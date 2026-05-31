import { useScrollPast, useDownloadBarDismissed } from '~/hooks/use-scroll-past'
import { DownloadButton } from '~/components/download-button'
import { downloadButtonLabel, platformLabel, type PlatformId } from '~/lib/download-links'
import type { ReleaseInfo } from '~/lib/github-releases'
import { cn } from '~/lib/utils'

interface DownloadBarProps {
  platform: PlatformId
  release: ReleaseInfo | null
  downloadUrl: string | null
}

export function DownloadBar({ platform, release, downloadUrl }: DownloadBarProps) {
  const scrolledPast = useScrollPast(520)
  const [dismissed, dismiss] = useDownloadBarDismissed()

  const visible = scrolledPast && !dismissed && !!downloadUrl && !!release

  if (!downloadUrl || !release) return null

  return (
    <div
      role="region"
      aria-label="Download prompt"
      aria-hidden={!visible}
      className={cn(
        'fixed inset-x-0 bottom-0 z-modal border-t border-border-subtle bg-background/95 backdrop-blur-md',
        'pb-[env(safe-area-inset-bottom)]',
        'transition-[transform,opacity] duration-200 ease-out download-bar-motion',
        visible ? 'translate-y-0 opacity-100' : 'pointer-events-none translate-y-full opacity-0',
      )}
    >
      <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-6 py-3">
        <p className="min-w-0 text-sm text-muted-foreground">
          <span className="font-medium text-foreground">Mdow v{release.version}</span>
          <span className="hidden sm:inline"> — free for {platformLabel(platform)}</span>
        </p>
        <div className="flex shrink-0 items-center gap-2">
          <DownloadButton href={downloadUrl} className="h-9 px-4 text-sm">
            {downloadButtonLabel(platform)}
          </DownloadButton>
          <button
            type="button"
            onClick={dismiss}
            className="inline-flex min-h-11 min-w-11 items-center justify-center rounded-md text-muted-foreground transition-[background-color,color] duration-150 ease hover:bg-muted hover:text-foreground"
            aria-label="Dismiss download bar"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden
            >
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  )
}
