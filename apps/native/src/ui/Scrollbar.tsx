import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { useGpuixRequired, type PublicInstance } from '@gpuix/react'
import { withAlpha } from '../lib/theme'
import { useUi } from './context'
import { setTopRow } from './reader-bus'
import { rowAt } from './scroll-model'

const HIT_WIDTH = 12
const THUMB_WIDTH = 6
const THUMB_WIDTH_ACTIVE = 8
/** gpuix never draws a native thumb shorter than this. */
const THUMB_MIN = 24
/** How often the current section is re-read while something scrolls. Reading commits nothing. */
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

type Listener = () => void
const activity = new Set<Listener>()
let lastScrollAt = Number.NEGATIVE_INFINITY

/** Tell the scrollbar something scrolled, so it wakes and follows. */
export function noteScroll() {
  lastScrollAt = performance.now()
  for (const listener of activity) listener()
}

/**
 * Whether the reader scrolled within the last moment. Rows sliding under a still pointer fire
 * mouse-enter as they pass, and each hover commit costs the window a re-measure of the visible
 * rows, so hover effects wait until the scroll settles (as browsers do).
 */
export function isScrolling(): boolean {
  return performance.now() - lastScrollAt < 150
}

interface Geometry {
  top: number
  size: number
  track: number
  total: number
  viewport: number
}

/**
 * The reader's overlay scrollbar. The list paints the thumb itself in every frame it scrolls, so
 * scrolling commits nothing; this hook only commits when the thumb fades in or out, when the
 * pointer hovers it (it widens), and while it is dragged. `listStyle` goes on the
 * `<virtual-list>`, and `overlay` is the invisible strip that takes presses and drags.
 */
export function useOverlayScrollbar(source: ScrollSource): {
  listStyle: { scrollbarColor: string; scrollbarWidth: number }
  overlay: ReactNode
} {
  const { theme } = useUi()
  const renderer = useGpuixRequired()
  const stripRef = useRef<PublicInstance | null>(null)
  const [fade, setFade] = useState(0)
  const [scrollable, setScrollable] = useState(false)
  const [hover, setHover] = useState(false)
  const [drag, setDrag] = useState<{ y: number; top: number } | null>(null)
  const sourceRef = useRef(source)
  sourceRef.current = source

  // Thumb geometry on estimated row heights plus GPUI's exact offset inside the current row; the
  // list draws its thumb from measured heights, so the two agree once rows near it are measured.
  const geometry = (): Geometry | null => {
    const { anchor, sums } = sourceRef.current
    const at = anchor()
    const total = sums.at(-1) ?? 0
    if (!at || total <= 0) return null
    const [row, offset, viewport] = at
    const y = row >= sums.length - 1 ? Math.max(0, total - viewport) : (sums[row] ?? 0) + offset
    // A heading counts as the current section once it reaches the top fifth of the view.
    setTopRow(Math.max(row, rowAt(sums, y + viewport * 0.2)[0]))
    if (total <= viewport + 1) return { top: 0, size: 0, track: viewport, total, viewport }
    const size = Math.min(viewport, Math.max(THUMB_MIN, (viewport * viewport) / total))
    const top = Math.max(0, Math.min(viewport - size, (y / (total - viewport)) * (viewport - size)))
    return { top, size, track: viewport, total, viewport }
  }

  useEffect(() => {
    let idleAt = 0
    let timer: ReturnType<typeof setTimeout> | null = null
    const tick = () => {
      const next = geometry()
      if (next) setScrollable(next.size > 0)
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
    // `geometry` reads the latest source through a ref.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const active = hover || drag !== null
  const opacity = active ? 1 : fade
  const color = withAlpha(active ? theme.mutedForeground : theme.border, opacity)
  const width = active ? THUMB_WIDTH_ACTIVE : THUMB_WIDTH
  const listStyle = useMemo(
    () => ({ scrollbarColor: color, scrollbarWidth: width }),
    [color, width],
  )

  const seekTo = (thumbTop: number, at: Geometry) => {
    const { sums, seek } = sourceRef.current
    const room = Math.max(1, at.track - at.size)
    const fraction = Math.max(0, Math.min(1, thumbTop / room))
    const [row, offset] = rowAt(sums, fraction * Math.max(0, at.total - at.viewport))
    seek(row, offset)
    noteScroll()
  }

  const overlay = scrollable ? (
    <>
      <div
        ref={stripRef}
        // Pointer-only: the keyboard already scrolls the reader, so this adds no a11y node.
        role="presentation"
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        onMouseDown={(event) => {
          const at = geometry()
          if (!at || at.size === 0) return
          // A press on the track jumps there; a press on the thumb starts a drag.
          const id = stripRef.current?.id
          const bounds = id === undefined ? null : renderer.getElementBounds?.(id)
          const local = (event.y ?? 0) - (bounds?.y ?? 0)
          let top = at.top
          if (local < at.top || local > at.top + at.size) {
            top = local - at.size / 2
            seekTo(top, at)
          }
          setDrag({ y: event.y ?? 0, top })
        }}
        style={{
          position: 'absolute',
          top: 0,
          right: 0,
          bottom: 0,
          width: HIT_WIDTH,
          pointerEvents: 'auto',
        }}
      />
      {drag ? (
        // Catch moves anywhere over the reader while dragging, so the thumb keeps following.
        <div
          role="presentation"
          onMouseMove={(event) => {
            // The button came up somewhere we didn't see (the sidebar, outside the window):
            // end the drag instead of scrolling with no button held.
            if (event.pressedButton !== 0) {
              setDrag(null)
              return
            }
            const at = geometry()
            if (at) seekTo(drag.top + ((event.y ?? 0) - drag.y), at)
          }}
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
  ) : null

  return { listStyle, overlay }
}
