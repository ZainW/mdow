import { useEffect, useRef, useState } from 'react'
import { useGpuixRequired, type PublicInstance } from '@gpuix/react'
import { useUi } from './context'
import { setTopRow } from './reader-bus'
import { rowAt } from './scroll-model'

const TRACK_WIDTH = 12
const TRACK_INSET = 4
const THUMB_MIN = 28
/**
 * Every React commit makes the window re-measure the list's visible rows, which is expensive for
 * heavy rows like big tables, so the thumb is sampled at 25Hz rather than every frame. (Native
 * `motion` would smooth the steps, but live windows don't apply retargeted motion reliably.)
 */
const POLL_MS = 40
/** The thumb fades after this long without scrolling, like macOS overlay scrollbars. */
const IDLE_MS = 1000
const FADE_MS = 300

export interface ScrollSource {
  /** `[row, offsetInRow, viewportHeight]`, or null before the list lays out. */
  anchor: () => [number, number, number] | null
  /** Estimated top of each row, plus the total height as the last entry. */
  sums: number[]
  seek: (row: number, offset: number) => void
}

interface Thumb {
  top: number
  size: number
  track: number
}

type Listener = () => void
const activity = new Set<Listener>()

/** Tell the scrollbar something scrolled, so it wakes and follows. */
export function noteScroll() {
  for (const listener of activity) listener()
}

/**
 * An overlay scrollbar for the reader. The list only measures rows near the viewport, so the
 * thumb is placed on estimated row heights plus GPUI's exact offset inside the current row.
 * It polls the list only while something is scrolling and sleeps otherwise.
 */
export function Scrollbar({ source }: { source: ScrollSource }) {
  const { theme } = useUi()
  const renderer = useGpuixRequired()
  const trackRef = useRef<PublicInstance | null>(null)
  const [thumb, setThumb] = useState<Thumb | null>(null)
  const [fade, setFade] = useState(0)
  const [hover, setHover] = useState(false)
  const [drag, setDrag] = useState<{ y: number; top: number } | null>(null)
  const sourceRef = useRef(source)
  sourceRef.current = source

  const measure = (): Thumb | null => {
    const { anchor, sums } = sourceRef.current
    const at = anchor()
    const total = sums.at(-1) ?? 0
    if (!at || total <= 0) return null
    const [row, offset, viewport] = at
    const track = Math.max(0, viewport - TRACK_INSET * 2)
    const y = row >= sums.length - 1 ? Math.max(0, total - viewport) : (sums[row] ?? 0) + offset
    // A heading counts as the current section once it reaches the top fifth of the view.
    setTopRow(Math.max(row, rowAt(sums, y + viewport * 0.2)[0]))
    if (total <= viewport + 1) return { top: 0, size: 0, track }
    // Whole pixels, so a slow thumb on a long document commits only when it visibly moves:
    // every commit costs the window a layout pass.
    const size = Math.round(Math.max(THUMB_MIN, (viewport / total) * track))
    const top = Math.round(
      Math.max(0, Math.min(track - size, (y / (total - viewport)) * (track - size))),
    )
    return { top, size, track }
  }

  useEffect(() => {
    let idleAt = 0
    let timer: ReturnType<typeof setTimeout> | null = null
    const tick = () => {
      const next = measure()
      if (next) {
        setThumb((current) =>
          current &&
          current.top === next.top &&
          current.size === next.size &&
          current.track === next.track
            ? current
            : next,
        )
      }
      const idle = performance.now() - idleAt
      // Quantized so the fade is a handful of commits, not one per poll.
      setFade(idle <= 0 ? 1 : Math.max(0, Math.round((1 - idle / FADE_MS) * 6) / 6))
      timer = idle < FADE_MS ? setTimeout(tick, POLL_MS) : null
    }
    const wake = () => {
      idleAt = performance.now() + IDLE_MS
      if (!timer) tick()
    }
    activity.add(wake)
    wake()
    return () => {
      activity.delete(wake)
      if (timer) clearTimeout(timer)
    }
    // `measure` reads the latest source through a ref.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  if (!thumb || thumb.size === 0) return null

  const seekTo = (thumbTop: number) => {
    const { sums, anchor, seek } = sourceRef.current
    const viewport = anchor()?.[2] ?? 0
    const total = sums.at(-1) ?? 0
    const room = Math.max(1, thumb.track - thumb.size)
    const fraction = Math.max(0, Math.min(1, thumbTop / room))
    setThumb({ ...thumb, top: Math.round(fraction * room) })
    const [row, offset] = rowAt(sums, fraction * Math.max(0, total - viewport))
    seek(row, offset)
    noteScroll()
  }

  const active = hover || drag !== null
  return (
    <>
      <div
        ref={trackRef}
        // Pointer-only: the keyboard already scrolls the reader, so this adds no a11y node.
        role="presentation"
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        onMouseDown={(event) => {
          // A press on the track jumps there; a press on the thumb starts a drag.
          const id = trackRef.current?.id
          const bounds = id === undefined ? null : renderer.getElementBounds?.(id)
          const local = (event.y ?? 0) - (bounds?.y ?? 0)
          let top = thumb.top
          if (local < thumb.top || local > thumb.top + thumb.size) {
            top = local - thumb.size / 2
            seekTo(top)
          }
          setDrag({ y: event.y ?? 0, top })
        }}
        style={{
          position: 'absolute',
          top: TRACK_INSET,
          right: 2,
          width: TRACK_WIDTH,
          height: thumb.track,
          pointerEvents: 'auto',
          opacity: active ? 1 : fade,
        }}
      >
        <div
          style={{
            position: 'absolute',
            top: thumb.top,
            right: 2,
            width: active ? 8 : 6,
            height: thumb.size,
            borderRadius: 4,
            backgroundColor: active ? theme.mutedForeground : theme.border,
          }}
        />
      </div>
      {drag ? (
        // Catch moves anywhere over the reader while dragging, so the thumb keeps following.
        <div
          role="presentation"
          onMouseMove={(event) => seekTo(drag.top + ((event.y ?? 0) - drag.y))}
          onMouseUp={() => setDrag(null)}
          onMouseDownOutside={() => setDrag(null)}
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            pointerEvents: 'auto',
          }}
        />
      ) : null}
    </>
  )
}
