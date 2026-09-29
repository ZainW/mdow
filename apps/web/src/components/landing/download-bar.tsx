import { useScrollPast, useDownloadBarDismissed } from '~/hooks/use-scroll-past'
import { DownloadButton } from '~/components/download-button'
import { CloseIcon } from '~/components/icons'
import { Logo } from '~/components/logo'
import type { PlatformId } from '~/lib/download-links'
import type { ReleaseInfo } from '~/lib/github-releases'
import { cn } from '~/lib/utils'

interface DownloadBarProps {
  platform: PlatformId
  release: ReleaseInfo | null
  downloadUrl: string | null
}

/** A small floating download prompt that appears once the hero is out of view. */
export function DownloadBar({ platform, release, downloadUrl }: DownloadBarProps) {
  const scrolledPast = useScrollPast(900)
  const [dismissed, dismiss] = useDownloadBarDismissed()

  if (!downloadUrl || !release) return null

  const visible = scrolledPast && !dismissed

  return (
    <div
      role="region"
      aria-label="Download Mdow"
      aria-hidden={!visible}
      inert={!visible}
      className={cn(
        'fixed inset-x-0 bottom-[max(1rem,env(safe-area-inset-bottom))] z-dropdown flex justify-center px-4',
        'motion-bar transition-[transform,opacity] duration-300 ease-[var(--ease-out)]',
        visible ? 'translate-y-0 opacity-100' : 'pointer-events-none translate-y-4 opacity-0',
      )}
    >
      <div className="flex items-center gap-3 rounded-2xl border border-border bg-popover/90 py-1.5 pl-2 pr-1.5 shadow-[0_12px_32px_-12px_hsl(var(--shadow-color)/0.3)] backdrop-blur-xl">
        <Logo className="size-8" alt="" />
        <p className="hidden text-sm sm:block">
          <span className="font-medium">Mdow</span>{' '}
          <span className="font-mono text-[12px] tabular-nums text-muted-foreground">
            v{release.version}
          </span>
        </p>
        <DownloadButton href={downloadUrl} platform={platform} size="sm">
          Download
        </DownloadButton>
        <button
          type="button"
          onClick={dismiss}
          className="btn btn-ghost size-9 rounded-xl p-0"
          aria-label="Dismiss download prompt"
        >
          <CloseIcon className="size-4" />
        </button>
      </div>
    </div>
  )
}
