import { useEffect, useReducer, useRef } from 'react'
import { Minus, Plus } from 'lucide-react'
import { prefersReducedMotion } from '../lib/motion'
import { useAppStore } from '../store/app-store'
import { ZOOM_MAX, ZOOM_MIN } from '../store/slices/settings-slice'
import { cn } from '../lib/utils'

// How long the pill stays up after the last zoom change, and after the pointer leaves it.
export const ZOOM_HUD_VISIBLE_MS = 1500
const ZOOM_HUD_LINGER_MS = 1000

interface IndicatorState {
  mounted: boolean
  visible: boolean
}

type IndicatorAction =
  | { type: 'show' }
  | { type: 'hide' }
  | { type: 'unmount' }
  | { type: 'reveal' }

function indicatorReducer(state: IndicatorState, action: IndicatorAction): IndicatorState {
  switch (action.type) {
    case 'show':
      return { mounted: true, visible: false }
    case 'hide':
      return { ...state, visible: false }
    case 'unmount':
      return { mounted: false, visible: false }
    case 'reveal':
      return { ...state, visible: true }
    default:
      return state
  }
}

/**
 * Transient zoom pill (bottom-right): appears on ⌘+ / ⌘− / ⌘0, fades out after
 * ~1.5s, and stays while hovered. Under reduced motion it appears and disappears
 * without a transition.
 */
export function ZoomIndicator() {
  const zoomLevel = useAppStore((s) => s.zoomLevel)
  const zoomIn = useAppStore((s) => s.zoomIn)
  const zoomOut = useAppStore((s) => s.zoomOut)
  const resetZoom = useAppStore((s) => s.resetZoom)
  const [{ mounted, visible }, dispatch] = useReducer(indicatorReducer, {
    mounted: false,
    visible: false,
  })
  const hideTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const prevZoom = useRef(zoomLevel)
  const hovered = useRef(false)

  const scheduleHide = (delay: number) => {
    clearTimeout(hideTimer.current)
    hideTimer.current = setTimeout(() => {
      if (hovered.current) return
      dispatch({ type: prefersReducedMotion() ? 'unmount' : 'hide' })
    }, delay)
  }

  useEffect(() => {
    if (zoomLevel === prevZoom.current) return undefined
    prevZoom.current = zoomLevel

    if (prefersReducedMotion()) {
      dispatch({ type: 'show' })
      dispatch({ type: 'reveal' })
    } else if (!mounted) {
      dispatch({ type: 'show' })
      requestAnimationFrame(() => dispatch({ type: 'reveal' }))
    } else {
      dispatch({ type: 'reveal' })
    }
    scheduleHide(ZOOM_HUD_VISIBLE_MS)

    return () => clearTimeout(hideTimer.current)
    // oxlint-disable-next-line react-hooks/exhaustive-deps -- only a zoom change should re-show the pill
  }, [zoomLevel])

  if (!mounted) return null

  const reduceMotion = prefersReducedMotion()

  return (
    <div
      data-testid="zoom-indicator"
      data-visible={visible}
      className="zoom-indicator floating-surface absolute right-4 bottom-4 z-(--z-sticky) flex items-center rounded-lg p-[3px] text-xs text-foreground"
      style={{
        opacity: visible ? 1 : 0,
        transform: reduceMotion ? undefined : visible ? 'scale(1)' : 'scale(0.96)',
        pointerEvents: visible ? 'auto' : 'none',
      }}
      onTransitionEnd={(e) => {
        if (e.target === e.currentTarget && !visible) dispatch({ type: 'unmount' })
      }}
      onMouseEnter={() => {
        hovered.current = true
        clearTimeout(hideTimer.current)
        dispatch({ type: 'reveal' })
      }}
      onMouseLeave={() => {
        hovered.current = false
        scheduleHide(ZOOM_HUD_LINGER_MS)
      }}
    >
      <HudButton label="Zoom out" disabled={zoomLevel <= ZOOM_MIN} onClick={zoomOut}>
        <Minus className="size-[13px]" aria-hidden />
      </HudButton>
      <output aria-live="polite" className="w-[46px] text-center font-medium tabular-nums">
        {zoomLevel}%
      </output>
      <HudButton label="Zoom in" disabled={zoomLevel >= ZOOM_MAX} onClick={zoomIn}>
        <Plus className="size-[13px]" aria-hidden />
      </HudButton>
      <span aria-hidden className="mx-[3px] h-4 w-px bg-border-subtle" />
      <button
        type="button"
        onClick={resetZoom}
        disabled={zoomLevel === 100}
        className="h-[26px] rounded-md px-2 text-[11.5px] text-muted-foreground outline-none hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50"
      >
        Reset
      </button>
    </div>
  )
}

function HudButton({
  label,
  disabled,
  onClick,
  children,
}: {
  label: string
  disabled?: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        'flex size-[26px] items-center justify-center rounded-md text-muted-foreground outline-none',
        'hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-40',
      )}
    >
      {children}
    </button>
  )
}
