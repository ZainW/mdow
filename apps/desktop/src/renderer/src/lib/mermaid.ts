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

function getMermaidOptions(isDark: boolean): MermaidOptions {
  return {
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'loose',
  }
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

export async function renderMermaidBlocks(blocks: { id: string; code: string }[]): Promise<void> {
  if (!mermaidInitialized) return
  const mermaid = await loadMermaid()
  mermaid.initialize(mermaidOptions)

  for (const block of blocks) {
    const el = document.getElementById(block.id)
    if (!el) continue

    try {
      el.className = 'mermaid mermaid-container'
      el.replaceChildren()
      // oxlint-disable-next-line no-await-in-loop -- intentional sequential rendering to avoid Mermaid race conditions
      const { svg } = await mermaid.render(`${block.id}-svg`, block.code)
      el.innerHTML = svg
    } catch (e) {
      el.className = 'mermaid-error'
      el.textContent = `Mermaid diagram error: ${e instanceof Error ? e.message : String(e)}`
      const errorSvg = document.getElementById(`d${block.id}-svg`)
      if (errorSvg) errorSvg.remove()
    }
  }
}
