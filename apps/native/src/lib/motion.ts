import { useEffect, useState } from 'react'
import { IS_MAC } from './platform'

/**
 * macOS "Reduce motion". Read once at launch and again with the appearance poll; motion in the
 * app collapses to instant changes while it is on.
 */
let reduced = false

export function reduceMotion() {
  return reduced
}

export async function refreshReduceMotion() {
  if (!IS_MAC) return
  try {
    const proc = Bun.spawn(['defaults', 'read', 'com.apple.universalaccess', 'reduceMotion'], {
      stdout: 'pipe',
      stderr: 'ignore',
    })
    reduced = (await new Response(proc.stdout).text()).trim() === '1'
  } catch {
    reduced = false
  }
}

/** Ease-out cubic: fast start, gentle landing. */
export function easeOut(t: number) {
  return 1 - (1 - t) ** 3
}

/** Short enters for surfaces opened now and then, never for hot paths like the palette. */
export const ENTER_MS = 180
const FRAME_MS = 16

/**
 * Eased 0→1 progress for an element's first moments on screen.
 *
 * Driven from JS on purpose: gpuix's native `motion` only advances while something else makes
 * the window redraw, so a lone fade could stall half-way. These are short and rare, so the few
 * React commits they cost don't matter.
 */
export function useEnter(enabled = true, ms = ENTER_MS): number {
  const [progress, setProgress] = useState(() => (enabled && !reduced ? 0 : 1))
  useEffect(() => {
    if (progress >= 1) return
    const started = performance.now()
    let timer: ReturnType<typeof setTimeout>
    const step = () => {
      const t = Math.min(1, (performance.now() - started) / ms)
      setProgress(t)
      if (t < 1) timer = setTimeout(step, FRAME_MS)
    }
    timer = setTimeout(step, 0)
    return () => clearTimeout(timer)
    // Runs once per mount: an enter never replays.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
  return easeOut(progress)
}
