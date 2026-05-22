import { useEffect, useReducer, useRef } from 'react'
import { Minus, Plus, RotateCcw } from 'lucide-react'
import { useAppStore } from '../store/app-store'
import { iconSize, iconStroke } from '../lib/icons'

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

  useEffect(() => {
    if (zoomLevel === prevZoom.current) return undefined
    prevZoom.current = zoomLevel

    dispatch({ type: 'show' })
    requestAnimationFrame(() => dispatch({ type: 'reveal' }))

    clearTimeout(hideTimer.current)
    hideTimer.current = setTimeout(() => {
      dispatch({ type: 'hide' })
    }, 2000)

    return () => clearTimeout(hideTimer.current)
  }, [zoomLevel])

  const handleTransitionEnd = () => {
    if (!visible) dispatch({ type: 'unmount' })
  }

  const handleMouseEnter = () => {
    clearTimeout(hideTimer.current)
    dispatch({ type: 'reveal' })
  }

  const handleMouseLeave = () => {
    hideTimer.current = setTimeout(() => {
      dispatch({ type: 'hide' })
    }, 1000)
  }

  if (!mounted && zoomLevel === 100) return null

  const persistentlyVisible = zoomLevel !== 100

  return (
    <div
      className="zoom-indicator absolute bottom-4 right-4 z-20 flex items-center gap-1 rounded-lg border border-border bg-popover px-1.5 py-1 text-xs text-foreground shadow-sm"
      style={{
        opacity: visible || persistentlyVisible ? 1 : 0,
        transform: visible || persistentlyVisible ? 'scale(1)' : 'scale(0.95)',
        pointerEvents: visible || persistentlyVisible ? 'auto' : 'none',
      }}
      onTransitionEnd={handleTransitionEnd}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <button
        className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors duration-100 hover:bg-muted hover:text-foreground active:scale-[0.97]"
        onClick={zoomOut}
        aria-label="Zoom out"
        title="Zoom out"
      >
        <Minus size={iconSize.md} strokeWidth={iconStroke.emphasis} aria-hidden />
      </button>
      <span className="min-w-[3ch] text-center tabular-nums" aria-live="polite">
        {zoomLevel}%
      </span>
      <button
        className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors duration-100 hover:bg-muted hover:text-foreground active:scale-[0.97]"
        onClick={zoomIn}
        aria-label="Zoom in"
        title="Zoom in"
      >
        <Plus size={iconSize.md} strokeWidth={iconStroke.emphasis} aria-hidden />
      </button>
      <button
        className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-[opacity,background-color,color,transform] duration-150 hover:bg-muted hover:text-foreground active:scale-[0.97] disabled:opacity-0"
        onClick={resetZoom}
        aria-label="Reset zoom"
        title="Reset zoom"
        disabled={zoomLevel === 100}
      >
        <RotateCcw size={iconSize.md} strokeWidth={iconStroke.emphasis} aria-hidden />
      </button>
    </div>
  )
}
