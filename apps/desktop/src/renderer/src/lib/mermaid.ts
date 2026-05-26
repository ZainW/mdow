type MermaidApi = (typeof import('mermaid'))['default']
interface MermaidOptions {
  startOnLoad: false
  theme: 'default' | 'dark'
  securityLevel: 'loose'
}

let mermaidInitialized = false
let mermaidPromise: Promise<MermaidApi> | null = null
let mermaidOptions: MermaidOptions = {
  startOnLoad: false,
  theme: 'default',
  securityLevel: 'loose',
}

const svgCache = new Map<string, string>()

function getMermaidOptions(isDark: boolean): MermaidOptions {
  return {
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'loose',
  }
}

function cacheKey(block: { id: string; code: string }, isDark: boolean): string {
  return `${block.id}:${isDark ? 'dark' : 'light'}:${block.code}`
}

async function loadMermaid(): Promise<MermaidApi> {
  mermaidPromise ??= import('mermaid').then((mod) => mod.default)
  return mermaidPromise
}

export function initMermaid(isDark: boolean): void {
  mermaidOptions = getMermaidOptions(isDark)
  mermaidInitialized = true
}

export function updateMermaidTheme(isDark: boolean): void {
  mermaidOptions = getMermaidOptions(isDark)
  if (mermaidPromise) {
    void mermaidPromise.then((mermaid) => mermaid.initialize(mermaidOptions))
  }
}

export function clearMermaidSvgCache(): void {
  svgCache.clear()
}

export function getMermaidSvgCacheSize(): number {
  return svgCache.size
}

function applySvgToElement(el: HTMLElement, svg: string): void {
  el.className = 'mermaid mermaid-container'
  el.innerHTML = svg
  el.setAttribute('role', 'img')
  if (!el.getAttribute('aria-label')) {
    el.setAttribute('aria-label', 'Mermaid diagram')
  }
}

export async function renderMermaidBlock(
  block: { id: string; code: string },
  isDark = document.documentElement.classList.contains('dark'),
): Promise<void> {
  if (!mermaidInitialized) return

  const el = document.getElementById(block.id)
  if (!el) return

  const key = cacheKey(block, isDark)
  const cached = svgCache.get(key)
  if (cached) {
    applySvgToElement(el, cached)
    return
  }

  const mermaid = await loadMermaid()
  mermaid.initialize(mermaidOptions)

  try {
    el.replaceChildren()
    const { svg } = await mermaid.render(`${block.id}-svg`, block.code)
    svgCache.set(key, svg)
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
  if (!mermaidInitialized) return

  // Sequential: Mermaid races when multiple diagrams render concurrently.
  await blocks.reduce(
    (chain, block) => chain.then(() => renderMermaidBlock(block)),
    Promise.resolve(),
  )
}
