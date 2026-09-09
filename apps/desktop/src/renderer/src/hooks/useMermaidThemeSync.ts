import { useEffect, useRef } from 'react'
import type { RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlock, updateMermaidTheme } from '../lib/mermaid'

export function useMermaidThemeSync(renderResult: RenderResult | null): void {
  const mermaidBlocksRef = useRef<RenderResult['mermaidBlocks']>([])

  useEffect(() => {
    initMermaid()
  }, [])

  useEffect(() => {
    mermaidBlocksRef.current = renderResult?.mermaidBlocks ?? []
  }, [renderResult])

  useEffect(() => {
    const observer = new MutationObserver(() => {
      updateMermaidTheme()
      const blocks = mermaidBlocksRef.current
      for (const block of blocks) {
        const el = document.getElementById(block.id)
        if (el?.querySelector('svg')) {
          void renderMermaidBlock(block)
        }
      }
    })

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class', 'data-palette', 'data-theme'],
    })

    return () => observer.disconnect()
  }, [])
}
