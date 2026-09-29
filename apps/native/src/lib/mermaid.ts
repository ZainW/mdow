import { renderMermaidSVG, type RenderOptions } from 'beautiful-mermaid'
import type { Theme } from './theme'

export interface MermaidPalette {
  bg: string
  fg: string
  line: string
  accent: string
  muted: string
  surface: string
  border: string
  font: string
}

export type MermaidResult =
  | { ok: true; src: string; width: number; height: number }
  | { ok: false; error: string }

const cache = new Map<string, MermaidResult>()
const CACHE_LIMIT = 200

const cacheKey = (source: string, palette: MermaidPalette) =>
  `${JSON.stringify(palette)}\n${source}`

function splitKey(key: string): [MermaidPalette, string] {
  const newline = key.indexOf('\n')
  return [JSON.parse(key.slice(0, newline)) as MermaidPalette, key.slice(newline + 1)]
}

/** Ink-on-paper diagrams: the reader's own colours, with the link blue for arrowheads. */
export function mermaidPalette(theme: Theme, font: string): MermaidPalette {
  const hex = (color: string) => toHex(parseColor(color) ?? [128, 128, 128, 255])
  const bg = hex(theme.surfaceWell)
  const fg = hex(theme.foreground)
  return {
    bg,
    fg,
    line: mix(fg, bg, 0.45),
    accent: hex(theme.primary),
    muted: hex(theme.mutedForeground),
    surface: hex(theme.surfaceRaised),
    border: mix(fg, bg, 0.22),
    font,
  }
}

let worker: Worker | null | undefined
let nextRequest = 0
const pending = new Map<number, { key: string; resolve: (result: MermaidResult) => void }>()
const inflight = new Map<string, Promise<MermaidResult>>()

function mermaidWorker(): Worker | null {
  if (worker !== undefined) return worker
  try {
    // A compiled binary names embedded workers by their path from the build root.
    const compiled = import.meta.url.startsWith('file:///$bunfs/')
    worker = new Worker(
      compiled
        ? './src/lib/mermaid-worker.ts'
        : new URL('./mermaid-worker.ts', import.meta.url).href,
    )
    worker.onerror = () => {
      // The worker can't run here: finish what it was given on the main thread instead.
      worker?.terminate()
      worker = null
      for (const [id, request] of pending) {
        pending.delete(id)
        const [palette, source] = splitKey(request.key)
        setTimeout(() => request.resolve(renderMermaid(source, palette)), 0)
      }
    }
    worker.onmessage = (event: MessageEvent<{ id: number; result: MermaidResult }>) => {
      const request = pending.get(event.data.id)
      if (!request) return
      pending.delete(event.data.id)
      remember(request.key, event.data.result)
      request.resolve(event.data.result)
    }
    // Let the process exit while the worker idles.
    worker.unref()
  } catch {
    worker = null
  }
  return worker
}

/**
 * Lay out a diagram without blocking the UI thread: in a worker when one can start, otherwise on
 * the main thread in a later task. Concurrent requests for the same diagram share one layout.
 */
export function renderMermaidAsync(
  source: string,
  palette: MermaidPalette,
): Promise<MermaidResult> {
  const key = cacheKey(source, palette)
  const hit = cache.get(key)
  if (hit) return Promise.resolve(hit)
  const running = inflight.get(key)
  if (running) return running
  const promise = new Promise<MermaidResult>((resolve) => {
    const target = mermaidWorker()
    if (target) {
      const id = nextRequest++
      pending.set(id, { key, resolve })
      target.postMessage({ id, source, palette })
    } else {
      setTimeout(() => resolve(renderMermaid(source, palette)), 0)
    }
  }).finally(() => inflight.delete(key))
  inflight.set(key, promise)
  return promise
}

function remember(key: string, result: MermaidResult) {
  if (cache.size >= CACHE_LIMIT) cache.delete(cache.keys().next().value!)
  cache.set(key, result)
}

export function cachedMermaid(source: string, palette: MermaidPalette): MermaidResult | null {
  return cache.get(cacheKey(source, palette)) ?? null
}

/**
 * Lay out a Mermaid diagram and return it as an SVG data URL sized in CSS pixels.
 *
 * beautiful-mermaid themes through CSS custom properties and `color-mix()`, which GPUI's SVG
 * rasterizer does not evaluate, so every colour is resolved to a literal before it leaves here.
 */
export function renderMermaid(source: string, palette: MermaidPalette): MermaidResult {
  const key = cacheKey(source, palette)
  const hit = cache.get(key)
  if (hit) return hit
  let result: MermaidResult
  try {
    const options: RenderOptions = { ...palette, transparent: true, padding: 16 }
    const svg = resolveCss(renderMermaidSVG(source.trim(), options))
    const width = Number(/<svg[^>]*\swidth="([\d.]+)"/.exec(svg)?.[1] ?? 0)
    const height = Number(/<svg[^>]*\sheight="([\d.]+)"/.exec(svg)?.[1] ?? 0)
    if (!width || !height) throw new Error('Diagram has no size')
    result = {
      ok: true,
      src: `data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`,
      width,
      height,
    }
  } catch (error) {
    result = { ok: false, error: error instanceof Error ? error.message : String(error) }
  }
  remember(key, result)
  return result
}

/** Inline every `var(--x)` and `color-mix()` so the SVG is plain SVG 1.1 paint. */
export function resolveCss(svg: string): string {
  const vars = new Map<string, string>()
  const declare = (block: string) => {
    for (const match of block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);?/g)) {
      vars.set(match[1]!, match[2]!.trim())
    }
  }
  const rootStyle = /<svg[^>]*\sstyle="([^"]*)"/.exec(svg)?.[1]
  if (rootStyle) declare(rootStyle)
  for (const style of svg.matchAll(/<style>([\s\S]*?)<\/style>/g)) {
    for (const rule of style[1]!.matchAll(/\{([^}]*)\}/g)) declare(rule[1]!)
  }

  const resolve = (value: string, depth = 0): string => {
    if (depth > 8) return value
    let out = value
    // Innermost var() first: its fallback may itself contain parentheses.
    for (let guard = 0; guard < 32 && out.includes('var('); guard++) {
      out = replaceCall(out, 'var', (inner) => {
        const comma = topLevelComma(inner)
        const name = (comma === -1 ? inner : inner.slice(0, comma)).trim()
        const fallback = comma === -1 ? '' : inner.slice(comma + 1).trim()
        const own = vars.get(name)
        return resolve(own ?? fallback, depth + 1)
      })
    }
    return replaceCall(out, 'color-mix', (inner) => colorMix(inner) ?? inner)
  }

  return svg
    .replace(/@import[^\n]*\n/g, '')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/<style>([\s\S]*?)<\/style>/g, (_, css: string) => {
      // Drop the custom-property rules and keep ordinary styling rules, resolved.
      const kept = css
        .replace(/--[\w-]+\s*:[^;]+;/g, '')
        .replace(/[^{}]*\{\s*\}/g, '')
        .trim()
      return kept ? `<style>${resolve(kept)}</style>` : ''
    })
    .replace(/(<svg[^>]*\s)style="[^"]*"/, '$1')
    .replace(/="([^"]*(?:var|color-mix)\([^"]*)"/g, (_, value: string) => `="${resolve(value)}"`)
    .replace(/(style="[^"]*")/g, (attr) => resolve(attr))
}

function replaceCall(input: string, fn: string, map: (inner: string) => string): string {
  const open = `${fn}(`
  let out = ''
  let index = 0
  while (index < input.length) {
    const start = input.indexOf(open, index)
    if (start === -1) break
    let depth = 0
    let end = start + open.length
    for (; end < input.length; end++) {
      const char = input[end]
      if (char === '(') depth++
      else if (char === ')') {
        if (depth === 0) break
        depth--
      }
    }
    const inner = input.slice(start + open.length, end)
    // Resolve nested calls of the same kind before this one.
    out += input.slice(index, start) + map(replaceCall(inner, fn, map))
    index = end + 1
  }
  return out + input.slice(index)
}

function topLevelComma(value: string) {
  let depth = 0
  for (let i = 0; i < value.length; i++) {
    if (value[i] === '(') depth++
    else if (value[i] === ')') depth--
    else if (value[i] === ',' && depth === 0) return i
  }
  return -1
}

/** `in srgb, #aaa 40%, #bbb` → hex. */
function colorMix(inner: string): string | null {
  const parts = inner.split(',').map((part) => part.trim())
  if (parts.length !== 3) return null
  const a = /^(\S+)(?:\s+([\d.]+)%)?$/.exec(parts[1]!)
  const b = /^(\S+)(?:\s+([\d.]+)%)?$/.exec(parts[2]!)
  if (!a || !b) return null
  const ca = parseColor(a[1]!)
  const cb = parseColor(b[1]!)
  if (!ca || !cb) return null
  const pa = a[2] ? Number(a[2]) / 100 : b[2] ? 1 - Number(b[2]) / 100 : 0.5
  const mixed = ca.map((channel, i) => channel * pa + cb[i]! * (1 - pa))
  return toHex(mixed)
}

function mix(a: string, b: string, share: number) {
  const ca = parseColor(a)!
  const cb = parseColor(b)!
  return toHex(ca.map((channel, i) => channel * share + cb[i]! * (1 - share)))
}

/** `#rgb`, `#rrggbb(aa)` or the theme's `hsl(h s% l% / a)`, as 0–255 RGBA. */
export function parseColor(color: string): number[] | null {
  const hsl =
    /^hsla?\(\s*([\d.]+)(?:deg)?[\s,]+([\d.]+)%[\s,]+([\d.]+)%(?:\s*[/,]\s*([\d.]+))?\s*\)$/i.exec(
      color.trim(),
    )
  if (hsl) {
    const h = Number(hsl[1]) / 360
    const s = Number(hsl[2]) / 100
    const l = Number(hsl[3]) / 100
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s
    const p = 2 * l - q
    const channel = (t: number) => {
      const k = (t + 1) % 1
      if (k < 1 / 6) return p + (q - p) * 6 * k
      if (k < 1 / 2) return q
      if (k < 2 / 3) return p + (q - p) * (2 / 3 - k) * 6
      return p
    }
    const alpha = hsl[4] === undefined ? 1 : Number(hsl[4])
    return [channel(h + 1 / 3), channel(h), channel(h - 1 / 3)]
      .map((value) => value * 255)
      .concat(alpha * 255)
  }
  let hex = color.trim().replace(/^#/, '')
  if (!/^[0-9a-f]+$/i.test(hex)) return null
  if (hex.length === 3 || hex.length === 4) {
    hex = hex
      .split('')
      .map((char) => char + char)
      .join('')
  }
  if (hex.length !== 6 && hex.length !== 8) return null
  const channels = [0, 2, 4, 6]
    .slice(0, hex.length / 2)
    .map((i) => parseInt(hex.slice(i, i + 2), 16))
  if (channels.length === 3) channels.push(255)
  return channels
}

function toHex(channels: number[]) {
  const [r, g, b, a = 255] = channels.map((channel) =>
    Math.round(Math.max(0, Math.min(255, channel))),
  )
  const hex = [r, g, b].map((channel) => channel!.toString(16).padStart(2, '0')).join('')
  return a === 255 ? `#${hex}` : `#${hex}${a.toString(16).padStart(2, '0')}`
}
