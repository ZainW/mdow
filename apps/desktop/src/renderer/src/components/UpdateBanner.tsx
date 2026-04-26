import { useEffect, useState } from 'react'
import { ArrowsClockwise, DownloadSimple, X } from '@phosphor-icons/react'
import { cn } from '@renderer/lib/utils'

type UpdateState =
  | { status: 'idle' }
  | { status: 'available'; version: string }
  | { status: 'downloading'; percent: number }
  | { status: 'ready' }
  | { status: 'up-to-date' }
  | { status: 'check-failed' }

export function UpdateBanner() {
  const [state, setState] = useState<UpdateState>({ status: 'idle' })
  const [dismissed, setDismissed] = useState(false)

  useEffect(() => {
    const unsubs = [
      window.api.onUpdateAvailable((info) => {
        setState({ status: 'available', version: info.version })
        setDismissed(false)
      }),
      window.api.onUpdateDownloadProgress((progress) => {
        setState({ status: 'downloading', percent: progress.percent })
      }),
      window.api.onUpdateDownloaded(() => {
        setState({ status: 'ready' })
        setDismissed(false)
      }),
      window.api.onUpdateUpToDate((info) => {
        if (info.wasManual) {
          setState({ status: 'up-to-date' })
          setDismissed(false)
        }
      }),
      window.api.onUpdateError(() => {
        setState({ status: 'check-failed' })
        setDismissed(false)
      }),
      window.api.onMenuCheckForUpdates(() => {
        void window.api.checkForUpdates({ manual: true })
      }),
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [])

  if (state.status === 'idle' || dismissed) return null

  return (
    <div
      className={cn(
        'flex items-center gap-2 border-t border-border bg-muted/50 px-3 py-1.5 text-xs text-muted-foreground',
        'motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-200',
      )}
      role="status"
      aria-live="polite"
    >
      {state.status === 'available' && (
        <>
          <span>
            Mdow <strong>{state.version}</strong> is available
          </span>
          <button
            type="button"
            onClick={() => void window.api.downloadUpdate()}
            className="ml-1 inline-flex min-h-[28px] items-center gap-1 rounded-md bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground transition-colors duration-150 ease-out hover:bg-primary/90"
          >
            <DownloadSimple weight="bold" className="size-3" aria-hidden />
            Download
          </button>
        </>
      )}

      {state.status === 'downloading' && (
        <>
          <ArrowsClockwise className="size-3 motion-safe:animate-spin" aria-hidden />
          <span>
            Downloading update… <span className="tabular-nums">{state.percent}</span>%
          </span>
          <div className="h-1 w-24 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary transition-[width] duration-150 ease-out"
              style={{ width: `${state.percent}%` }}
            />
          </div>
        </>
      )}

      {state.status === 'ready' && (
        <>
          <span>Update ready — restart to apply</span>
          <button
            type="button"
            onClick={() => void window.api.installUpdate()}
            className="ml-1 inline-flex min-h-[28px] items-center gap-1 rounded-md bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground transition-colors duration-150 ease-out hover:bg-primary/90"
          >
            <ArrowsClockwise weight="bold" className="size-3" aria-hidden />
            Restart
          </button>
        </>
      )}

      {state.status === 'up-to-date' && <span>You're on the latest version</span>}

      {state.status === 'check-failed' && <span>Couldn't check for updates — try again later</span>}

      <button
        type="button"
        onClick={() => setDismissed(true)}
        className="ml-auto rounded p-1 transition-colors duration-150 ease-out hover:bg-muted"
        aria-label="Dismiss update notification"
      >
        <X className="size-3" aria-hidden />
      </button>
    </div>
  )
}
