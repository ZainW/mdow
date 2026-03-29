import mermaid from 'mermaid'

let mermaidInitialized = false

export function initMermaid(isDark: boolean): void {
  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'loose',
  })
  mermaidInitialized = true
}

export function updateMermaidTheme(isDark: boolean): void {
  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'loose',
  })
}

export async function renderMermaidBlocks(blocks: { id: string; code: string }[]): Promise<void> {
  if (!mermaidInitialized) return

  for (const block of blocks) {
    const el = document.getElementById(block.id)
    if (!el) continue

    try {
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
