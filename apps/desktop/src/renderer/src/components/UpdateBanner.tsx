import { useEffect, useState } from 'react'
import { ArrowsClockwise, DownloadSimple, X } from '@phosphor-icons/react'

type UpdateState =
  | { status: 'idle' }
  | { status: 'available'; version: string }
  | { status: 'downloading'; percent: number }
  | { status: 'ready' }
  | { status: 'error'; message: string }

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
      }),
      window.api.onUpdateError((message) => {
        setState({ status: 'error', message })
      }),
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [])

  if (state.status === 'idle' || dismissed) return null

  return (
    <div className="flex items-center gap-2 border-t border-border bg-muted/50 px-3 py-1.5 text-xs text-muted-foreground">
      {state.status === 'available' && (
        <>
          <span>
            Mdow <strong>{state.version}</strong> is available
          </span>
          <button
            type="button"
            onClick={() => void window.api.downloadUpdate()}
            className="ml-1 inline-flex items-center gap-1 rounded-md bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground hover:bg-primary/90"
          >
            <DownloadSimple weight="bold" className="size-3" />
            Download
          </button>
        </>
      )}

      {state.status === 'downloading' && (
        <>
          <ArrowsClockwise className="size-3 animate-spin" />
          <span>Downloading update… {state.percent}%</span>
          <div className="h-1 w-24 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary transition-all"
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
            className="ml-1 inline-flex items-center gap-1 rounded-md bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground hover:bg-primary/90"
          >
            <ArrowsClockwise weight="bold" className="size-3" />
            Restart
          </button>
        </>
      )}

      {state.status === 'error' && <span>Update failed: {state.message}</span>}

      <button
        type="button"
        onClick={() => setDismissed(true)}
        className="ml-auto rounded p-0.5 hover:bg-muted"
        aria-label="Dismiss"
      >
        <X className="size-3" />
      </button>
    </div>
  )
}
