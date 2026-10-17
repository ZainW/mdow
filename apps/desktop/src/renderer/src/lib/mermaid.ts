// Mermaid diagrams render with Pele: synchronous, themed entirely by the `--pele-*` CSS variables in
// markdown.css, so switching light/dark restyles diagrams without re-rendering them.
type Pele = typeof import('pele/lazy')

const DIAGRAM_FONT = "'IBM Plex Mono', 'Geist Mono', ui-monospace, monospace"
const DIAGRAM_FONT_SIZE = 13.5

let pelePromise: Promise<Pele> | null = null

// `pele/lazy` keeps each diagram type in its own chunk, loaded the first time a document uses it.
function loadPele(): Promise<Pele> {
  pelePromise ??= import('pele/lazy')
  return pelePromise
}

let preloadScheduled = false

/** Loads Pele at the next idle moment, so the first diagram does not wait on the download. */
export function preloadMermaid(): void {
  if (preloadScheduled || pelePromise) return
  preloadScheduled = true
  const load = () => void loadPele()
  if ('requestIdleCallback' in globalThis) requestIdleCallback(load, { timeout: 2000 })
  else setTimeout(load, 1)
}

function diagramFont(el: HTMLElement): { fontFamily: string; fontSize: number } {
  const style = getComputedStyle(el)
  const fontFamily = style.getPropertyValue('--md-diagram-font').trim() || DIAGRAM_FONT
  const fontSize =
    Number.parseFloat(style.getPropertyValue('--md-diagram-font-size')) || DIAGRAM_FONT_SIZE
  return { fontFamily, fontSize }
}

export async function renderMermaidBlock(block: { id: string; code: string }): Promise<void> {
  const el = document.getElementById(block.id)
  if (!el) return

  try {
    const { renderAsync } = await loadPele()
    const { fontFamily, fontSize } = diagramFont(el)
    const { svg } = await renderAsync(block.code, {
      fontFamily,
      fontFamilyMono: fontFamily,
      fontSize,
      idPrefix: `${block.id}-`,
      // The reading column is narrow; keep the author's direction instead of flipping LR to TD.
      autoDirection: false,
    })
    el.className = 'mermaid mermaid-container'
    el.innerHTML = svg
    el.setAttribute('role', 'img')
    if (!el.getAttribute('aria-label')) el.setAttribute('aria-label', 'Mermaid diagram')
  } catch (e) {
    el.className = 'mermaid-error'
    el.removeAttribute('role')
    el.textContent = `Mermaid diagram error: ${e instanceof Error ? e.message : String(e)}`
  }
}
