import { useEffect, useReducer } from 'react'
import { Download, RefreshCw, X } from 'lucide-react'
import { cn } from '@renderer/lib/utils'
import { iconStroke } from '../lib/icons'

type UpdateState =
  | { status: 'idle' }
  | { status: 'available'; version: string }
  | { status: 'downloading'; percent: number }
  | { status: 'ready' }
  | { status: 'up-to-date' }
  | { status: 'check-failed' }

interface BannerUi {
  update: UpdateState
  dismissed: boolean
}

type BannerAction =
  | { type: 'set-update'; update: UpdateState; resetDismissed?: boolean }
  | { type: 'dismiss' }

function bannerReducer(state: BannerUi, action: BannerAction): BannerUi {
  switch (action.type) {
    case 'set-update':
      return {
        update: action.update,
        dismissed: action.resetDismissed ? false : state.dismissed,
      }
    case 'dismiss':
      return { ...state, dismissed: true }
    default:
      return state
  }
}

export function UpdateBanner() {
  const [{ update: state, dismissed }, dispatch] = useReducer(bannerReducer, {
    update: { status: 'idle' },
    dismissed: false,
  })

  useEffect(() => {
    const unsubs = [
      window.api.onUpdateAvailable((info) => {
        dispatch({
          type: 'set-update',
          update: { status: 'available', version: info.version },
          resetDismissed: true,
        })
      }),
      window.api.onUpdateDownloadProgress((progress) => {
        dispatch({
          type: 'set-update',
          update: { status: 'downloading', percent: progress.percent },
        })
      }),
      window.api.onUpdateDownloaded(() => {
        dispatch({
          type: 'set-update',
          update: { status: 'ready' },
          resetDismissed: true,
        })
      }),
      window.api.onUpdateUpToDate((info) => {
        if (info.wasManual) {
          dispatch({
            type: 'set-update',
            update: { status: 'up-to-date' },
            resetDismissed: true,
          })
        }
      }),
      window.api.onUpdateError(() => {
        dispatch({
          type: 'set-update',
          update: { status: 'check-failed' },
          resetDismissed: true,
        })
      }),
      window.api.onMenuCheckForUpdates(() => {
        void window.api.checkForUpdates({ manual: true })
      }),
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [])

  if (state.status === 'idle' || dismissed) return null

  return (
    <output
      className={cn(
        'flex items-center gap-2 border-t border-border bg-muted/50 px-3 py-1.5 text-xs text-muted-foreground',
        'motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-200',
      )}
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
            <Download strokeWidth={iconStroke.emphasis} className="size-3" aria-hidden />
            Download
          </button>
        </>
      )}

      {state.status === 'downloading' && (
        <>
          <RefreshCw className="size-3 motion-safe:animate-spin" aria-hidden />
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
          <span>Update ready. Restart to apply.</span>
          <button
            type="button"
            onClick={() => void window.api.installUpdate()}
            className="ml-1 inline-flex min-h-[28px] items-center gap-1 rounded-md bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground transition-colors duration-150 ease-out hover:bg-primary/90"
          >
            <RefreshCw strokeWidth={iconStroke.emphasis} className="size-3" aria-hidden />
            Restart
          </button>
        </>
      )}

      {state.status === 'up-to-date' && <span>You're on the latest version</span>}

      {state.status === 'check-failed' && (
        <span>Couldn&apos;t check for updates. Try again later.</span>
      )}

      <button
        type="button"
        onClick={() => dispatch({ type: 'dismiss' })}
        className="ml-auto rounded p-1 transition-colors duration-150 ease-out hover:bg-muted"
        aria-label="Dismiss update notification"
      >
        <X className="size-3" strokeWidth={iconStroke.emphasis} aria-hidden />
      </button>
    </output>
  )
}
