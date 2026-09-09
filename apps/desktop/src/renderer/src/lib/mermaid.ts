import { defineCachedFunction } from 'ocache'
import { rendererCacheStorage } from './cache-storage'
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

const generateMermaidSvg = defineCachedFunction(generateMermaidSvgUncached, {
  name: 'mermaidSvg',
  storage: rendererCacheStorage,
  maxAge: 3600,
})

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
    const svg = await generateMermaidSvg(block.id, block.code, paletteId)
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
