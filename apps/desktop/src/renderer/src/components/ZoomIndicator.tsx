import { useEffect, useRef, useState } from 'react'
import { useAppStore } from '../store/app-store'
import { Minus, Plus, ArrowCounterClockwise } from '@phosphor-icons/react'

export function ZoomIndicator() {
  const zoomLevel = useAppStore((s) => s.zoomLevel)
  const zoomIn = useAppStore((s) => s.zoomIn)
  const zoomOut = useAppStore((s) => s.zoomOut)
  const resetZoom = useAppStore((s) => s.resetZoom)
  const [visible, setVisible] = useState(false)
  const [mounted, setMounted] = useState(false)
  const hideTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const prevZoom = useRef(zoomLevel)

  useEffect(() => {
    if (zoomLevel === prevZoom.current) return undefined
    prevZoom.current = zoomLevel

    // Show indicator on any zoom change
    setMounted(true)
    // Defer to next frame so the mount renders before the enter animation
    requestAnimationFrame(() => setVisible(true))

    // Auto-hide after 2s (unless zoom is not 100%)
    clearTimeout(hideTimer.current)
    hideTimer.current = setTimeout(() => {
      setVisible(false)
    }, 2000)

    return () => clearTimeout(hideTimer.current)
  }, [zoomLevel])

  // Keep mounted through exit animation, then unmount
  const handleTransitionEnd = () => {
    if (!visible) setMounted(false)
  }

  // Also show persistently on hover (re-show if fading out)
  const handleMouseEnter = () => {
    clearTimeout(hideTimer.current)
    setVisible(true)
  }

  const handleMouseLeave = () => {
    hideTimer.current = setTimeout(() => {
      setVisible(false)
    }, 1000)
  }

  if (!mounted && zoomLevel === 100) return null

  // Always show if zoom is not 100% (persistent, but faded)
  const persistentlyVisible = zoomLevel !== 100

  return (
    <div
      className="absolute bottom-4 right-4 z-20 flex items-center gap-1 rounded-lg border border-border bg-popover px-1.5 py-1 text-xs text-foreground shadow-sm"
      style={{
        opacity: visible || persistentlyVisible ? 1 : 0,
        transform: visible || persistentlyVisible ? 'scale(1)' : 'scale(0.95)',
        transition:
          'opacity 200ms cubic-bezier(0.23, 1, 0.32, 1), transform 200ms cubic-bezier(0.23, 1, 0.32, 1)',
        pointerEvents: visible || persistentlyVisible ? 'auto' : 'none',
      }}
      onTransitionEnd={handleTransitionEnd}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <button
        className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors duration-100 hover:bg-muted hover:text-foreground active:scale-[0.97]"
        onClick={zoomOut}
        title="Zoom out"
      >
        <Minus size={14} weight="bold" />
      </button>
      <span className="min-w-[3ch] text-center tabular-nums">{zoomLevel}%</span>
      <button
        className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors duration-100 hover:bg-muted hover:text-foreground active:scale-[0.97]"
        onClick={zoomIn}
        title="Zoom in"
      >
        <Plus size={14} weight="bold" />
      </button>
      {zoomLevel !== 100 && (
        <button
          className="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors duration-100 hover:bg-muted hover:text-foreground active:scale-[0.97]"
          onClick={resetZoom}
          title="Reset zoom"
        >
          <ArrowCounterClockwise size={14} weight="bold" />
        </button>
      )}
    </div>
  )
}
