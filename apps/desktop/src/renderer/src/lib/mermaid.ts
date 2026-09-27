import { memoizeAsync } from './cache-storage'
import {
  getMermaidInitConfig,
  resolveMermaidPaletteId,
  type MermaidInitConfig,
  type MermaidPaletteId,
} from './mermaid-theme'

type MermaidApi = (typeof import('mermaid'))['default']

let mermaidInitialized = false
let mermaidPromise: Promise<MermaidApi> | null = null
let mermaidOptions: MermaidInitConfig = getMermaidInitConfig('light')

function ensureMermaidInitialized(paletteId: MermaidPaletteId): void {
  if (mermaidInitialized) return
  mermaidOptions = getMermaidInitConfig(paletteId)
  mermaidInitialized = true
}

async function loadMermaid(): Promise<MermaidApi> {
  mermaidPromise ??= import('mermaid').then((mod) => mod.default)
  return mermaidPromise
}

let preloadScheduled = false

/** Loads Mermaid at the next idle moment, so the first diagram does not wait on the download. */
export function preloadMermaid(): void {
  if (preloadScheduled || mermaidPromise) return
  preloadScheduled = true
  const load = () => void loadMermaid()
  if ('requestIdleCallback' in globalThis) requestIdleCallback(load, { timeout: 2000 })
  else setTimeout(load, 1)
}

export function initMermaid(isDark?: boolean): void {
  mermaidOptions = getMermaidInitConfig(resolveMermaidPaletteId(isDark))
  mermaidInitialized = true
}

export function updateMermaidTheme(isDark?: boolean): void {
  mermaidOptions = getMermaidInitConfig(resolveMermaidPaletteId(isDark))
  if (mermaidPromise) {
    void mermaidPromise.then((mermaid) => mermaid.initialize(mermaidOptions))
  }
}

function applySvgToElement(el: HTMLElement, svg: string): void {
  el.className = 'mermaid mermaid-container'
  el.innerHTML = svg
  const svgEl = el.querySelector('svg')
  if (svgEl) {
    svgEl.style.background = 'transparent'
    svgEl.style.backgroundColor = 'transparent'
    svgEl.style.maxWidth = '100%'
    svgEl.style.height = 'auto'
    svgEl.style.width = 'auto'
    const width = svgEl.getAttribute('width')
    if (width === '100%' || width === '100') {
      svgEl.removeAttribute('width')
    }
  }
  el.setAttribute('role', 'img')
  if (!el.getAttribute('aria-label')) {
    el.setAttribute('aria-label', 'Mermaid diagram')
  }
}

async function generateMermaidSvgUncached(
  blockId: string,
  code: string,
  paletteId: MermaidPaletteId,
): Promise<string> {
  const mermaid = await loadMermaid()
  const options = getMermaidInitConfig(paletteId)
  mermaid.initialize(options)
  const { svg } = await mermaid.render(`${blockId}-svg`, code)
  return svg
}

const generateMermaidSvg = memoizeAsync(generateMermaidSvgUncached, {
  maxEntries: 200,
  getKey: (blockId, code, paletteId) => `${paletteId}\u0000${blockId}\u0000${code}`,
})

// Mermaid shares global state across render() calls and races when diagrams render
// concurrently, so renders run one at a time. The queue always picks the waiting diagram nearest
// the viewport, so the diagram the reader is looking at never waits behind ones they scrolled past.
interface RenderJob {
  el: HTMLElement
  run: () => Promise<void>
}

const renderJobs: RenderJob[] = []
let pumping = false

function viewportDistance(el: HTMLElement): number {
  if (!el.isConnected) return Number.POSITIVE_INFINITY
  const viewport =
    el.closest('[data-markdown-scroller]')?.getBoundingClientRect() ??
    new DOMRect(0, 0, window.innerWidth, window.innerHeight)
  const rect = el.getBoundingClientRect()
  if (rect.bottom < viewport.top) return viewport.top - rect.bottom
  if (rect.top > viewport.bottom) return rect.top - viewport.bottom
  return 0
}

async function pumpRenderJobs(): Promise<void> {
  if (pumping) return
  pumping = true
  try {
    while (renderJobs.length > 0) {
      let best = 0
      let bestDistance = Number.POSITIVE_INFINITY
      for (let i = 0; i < renderJobs.length; i++) {
        const distance = viewportDistance(renderJobs[i].el)
        if (distance < bestDistance) {
          best = i
          bestDistance = distance
          if (distance === 0) break
        }
      }
      const [job] = renderJobs.splice(best, 1)
      // oxlint-disable-next-line no-await-in-loop -- Mermaid must render one diagram at a time.
      await job.run()
    }
  } finally {
    pumping = false
  }
}

function enqueueRender<T>(el: HTMLElement, task: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    renderJobs.push({ el, run: () => task().then(resolve, reject) })
    void pumpRenderJobs()
  })
}

export async function renderMermaidBlock(
  block: { id: string; code: string },
  isDark?: boolean,
): Promise<void> {
  const paletteId = resolveMermaidPaletteId(isDark)
  ensureMermaidInitialized(paletteId)

  const el = document.getElementById(block.id)
  if (!el) return

  try {
    el.replaceChildren()
    const svg = await enqueueRender(el, () => generateMermaidSvg(block.id, block.code, paletteId))
    applySvgToElement(el, svg)
  } catch (e) {
    el.className = 'mermaid-error'
    el.removeAttribute('role')
    el.textContent = `Mermaid diagram error: ${e instanceof Error ? e.message : String(e)}`
    const errorSvg = document.getElementById(`d${block.id}-svg`)
    if (errorSvg) errorSvg.remove()
  }
}

export async function renderMermaidBlocks(blocks: { id: string; code: string }[]): Promise<void> {
  // Sequential: Mermaid races when multiple diagrams render concurrently.
  await blocks.reduce(
    (chain, block) => chain.then(() => renderMermaidBlock(block)),
    Promise.resolve(),
  )
}
