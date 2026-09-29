import { useEffect, useRef, useState } from 'react'
import { reduceMotion } from '../lib/motion'
import { useApp } from '../store'
import { useUi } from './context'
import { Label } from './primitives'

const IN_MS = 120
const HOLD_MS = 800
const OUT_MS = 220
const FRAME_MS = 16

/** A small pill that confirms the zoom level after ⌘+ / ⌘−, then fades away. */
export function ZoomHud() {
  const { theme, scale } = useUi()
  const zoom = useApp((state) => state.prefs.zoomLevel)
  const first = useRef(true)
  const [opacity, setOpacity] = useState(0)
  useEffect(() => {
    if (first.current) {
      first.current = false
      return
    }
    // Repeated presses restart the hold without fading back in from nothing.
    const started = performance.now() - (reduceMotion() ? IN_MS : 0)
    let timer: ReturnType<typeof setTimeout>
    const step = () => {
      const t = performance.now() - started
      const next =
        t < IN_MS ? t / IN_MS : t < IN_MS + HOLD_MS ? 1 : 1 - (t - IN_MS - HOLD_MS) / OUT_MS
      setOpacity((current) => (t < IN_MS ? Math.max(current, next) : Math.max(0, next)))
      if (next > 0) timer = setTimeout(step, reduceMotion() ? HOLD_MS : FRAME_MS)
    }
    step()
    return () => clearTimeout(timer)
  }, [zoom])
  if (opacity <= 0) return null
  return (
    <div
      style={{
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: 28,
        display: 'flex',
        justifyContent: 'center',
        pointerEvents: 'none',
        opacity,
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          height: scale.buttonHeight + 8,
          paddingLeft: 16,
          paddingRight: 16,
          borderRadius: 999,
          borderWidth: 1,
          borderColor: theme.border,
          backgroundColor: theme.surfaceRaised,
          boxShadow: {
            offsetX: 0,
            offsetY: 6,
            blurRadius: 18,
            spreadRadius: 0,
            color: '#00000030',
          },
        }}
      >
        <Label weight={500} size={scale.controlFont + 2} mono>{`${zoom}%`}</Label>
      </div>
    </div>
  )
}
