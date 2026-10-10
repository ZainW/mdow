/**
 * Open real documents in a real window and measure what a reader feels.
 * Usage: bun scripts/bench.tsx [--json out.json] [--shots dir] <file…>
 * Synthetic documents: `bun scripts/bench-fixtures.ts` writes them to tmp/bench.
 *
 * - open: time from `openDocument` to the first frame that shows it (parse + first paint)
 * - scroll: a trackpad-like fling of wheel events, one per frame, through the whole document.
 *   Reports the gap between frames (what jank looks like) and the native frame cost.
 * - jump: `End` then `Home`, the worst case for a virtualized reader.
 */
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, join, resolve } from 'node:path'
import { createRenderer, flushSync, render } from '@gpuix/react'
import { App } from '../src/App'
import { EMPTY_SESSION } from '../src/lib/persist'
import { DEFAULT_PREFS } from '../src/lib/prefs'
import { closeTab, createAppStore, openDocumentAsync, setAppStore } from '../src/store'
import { sendReader } from '../src/ui/reader-bus'

const argv = process.argv.slice(2)
const flag = (name: string) => {
  const index = argv.indexOf(name)
  if (index === -1) return null
  const [value] = argv.splice(index, 2).slice(1)
  return value ?? null
}
const jsonOut = flag('--json')
const shotsDir = flag('--shots')
const files = argv.map((path) => resolve(path))

process.env.MDOW_STATE_PATH = join(mkdtempSync(join(tmpdir(), 'mdow-bench-')), 'state.json')
setAppStore(createAppStore({ prefs: { ...DEFAULT_PREFS }, session: { ...EMPTY_SESSION } }, 'dark'))

const WIDTH = 1200
const HEIGHT = 820
const renderer = createRenderer()
renderer.init({ title: 'Mdow bench', width: WIDTH, height: HEIGHT, focus: true })
renderer.setDebugFrameOverlay(process.env.BENCH_OVERLAY ?? 'hidden')

// Drive frames ourselves so each tick can be timed. JS work (React commits, parsing) runs
// between ticks, so a long gap between tick starts is exactly a dropped frame.
let ticks: number[] = []
let tickCost: number[] = []
let running = true
const pump = () => {
  if (!running) return
  const start = performance.now()
  renderer.tick()
  const end = performance.now()
  ticks.push(start)
  tickCost.push(end - start)
  setTimeout(pump, Math.max(0, 8 - (end - start)))
}
render(<App saveNow={() => {}} />, { renderer })
pump()

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms))
const nextFrames = async (count: number) => {
  const target = ticks.length + count
  while (ticks.length < target) await sleep(1)
}

function stats(values: number[]) {
  if (values.length === 0) return { p50: 0, p95: 0, p99: 0, max: 0 }
  const sorted = [...values].sort((a, b) => a - b)
  const at = (q: number) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))]!
  return { p50: at(0.5), p95: at(0.95), p99: at(0.99), max: sorted[sorted.length - 1]! }
}

function gaps(from: number) {
  const out: number[] = []
  for (let i = from + 1; i < ticks.length; i++) out.push(ticks[i]! - ticks[i - 1]!)
  return out
}

interface Result {
  file: string
  openMs: number
  openFrames: ReturnType<typeof stats> & { frames: number; over33: number }
  scroll: ReturnType<typeof stats> & { frames: number; over33: number; seconds: number }
  draw: { fps: number; p90: number; p99: number; max: number }
  tick: ReturnType<typeof stats>
  jumpEndMs: number
  jumpHomeMs: number
  rssMb: number
}

const results: Result[] = []
await nextFrames(20)

for (const file of files) {
  // Open: parse + commit + first paint.
  const openTickStart = ticks.length
  const openStart = performance.now()
  await openDocumentAsync(file)
  await nextFrames(2)
  const openMs = performance.now() - openStart
  const openFrameGaps = gaps(openTickStart - 1)
  const openFrames = {
    ...stats(openFrameGaps),
    frames: openFrameGaps.length,
    over33: openFrameGaps.filter((gap) => gap > 33.4).length,
  }
  await sleep(300)
  if (shotsDir) renderer.captureScreenshot?.(join(shotsDir, `${basename(file)}-open.png`))

  // Scroll: 90px per frame, like a steady trackpad fling, for up to 6 seconds.
  const x = WIDTH / 2 + 120
  const y = HEIGHT / 2
  const first = ticks.length
  const scrollStart = performance.now()
  const costStart = tickCost.length
  renderer.resetDebugFrameOverlayStats()
  const framesBefore = renderer.getDebugFrameOverlayStats().frames
  let seen = ticks.length
  while (performance.now() - scrollStart < 6000) {
    renderer.simulateScrollWheel(x, y, 0, -90)
    while (ticks.length === seen) await sleep(0)
    seen = ticks.length
  }
  const seconds = (performance.now() - scrollStart) / 1000
  const frameGaps = gaps(first)
  const scroll = {
    ...stats(frameGaps),
    frames: frameGaps.length,
    over33: frameGaps.filter((gap) => gap > 33.4).length,
    seconds,
  }
  const tick = stats(tickCost.slice(costStart))
  const draw = renderer.getDebugFrameOverlayStats()
  const drawn = draw.frames - framesBefore
  if (shotsDir) renderer.captureScreenshot?.(join(shotsDir, `${basename(file)}-scrolled.png`))

  // Jumps.
  let start = performance.now()
  flushSync(() => sendReader({ type: 'scroll', to: 'bottom' }))
  await nextFrames(2)
  const jumpEndMs = performance.now() - start
  await sleep(200)
  start = performance.now()
  flushSync(() => sendReader({ type: 'scroll', to: 'top' }))
  await nextFrames(2)
  const jumpHomeMs = performance.now() - start

  const rssMb = process.memoryUsage().rss / 1024 / 1024
  const result = {
    file: basename(file),
    openMs,
    openFrames,
    scroll,
    draw: {
      fps: drawn / seconds,
      p90: draw.p90Ms ?? 0,
      p99: draw.p99Ms ?? 0,
      max: draw.maxMs ?? 0,
    },
    tick,
    jumpEndMs,
    jumpHomeMs,
    rssMb,
  }
  results.push(result)
  console.log(
    `${result.file.padEnd(28)} open ${openMs.toFixed(0).padStart(5)}ms ` +
      `ui gap p95 ${openFrames.p95.toFixed(1)} max ${openFrames.max.toFixed(0)}ms ` +
      `>33ms ${openFrames.over33} | ` +
      `scroll fps ${(scroll.frames / seconds).toFixed(0).padStart(3)} ` +
      `gap p50 ${scroll.p50.toFixed(1)} p95 ${scroll.p95.toFixed(1)} p99 ${scroll.p99.toFixed(1)} ` +
      `max ${scroll.max.toFixed(0)}ms, >33ms ${scroll.over33} | ` +
      `draw ${result.draw.fps.toFixed(0)}fps p90 ${result.draw.p90.toFixed(1)} p99 ${result.draw.p99.toFixed(1)} max ${result.draw.max.toFixed(0)}ms | ` +
      `end ${jumpEndMs.toFixed(0)}ms home ${jumpHomeMs.toFixed(0)}ms | rss ${rssMb.toFixed(0)}MB`,
  )
  flushSync(() => closeTab())
  await nextFrames(5)
}

if (jsonOut) writeFileSync(jsonOut, JSON.stringify(results, null, 2))
running = false
process.exit(0)
