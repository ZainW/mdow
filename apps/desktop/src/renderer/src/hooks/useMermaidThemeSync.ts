import { useEffect, useRef } from 'react'
import type { RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlock, updateMermaidTheme } from '../lib/mermaid'

export function useMermaidThemeSync(renderResult: RenderResult | null): void {
  const mermaidBlocksRef = useRef<RenderResult['mermaidBlocks']>([])

  useEffect(() => {
    const isDark = document.documentElement.classList.contains('dark')
    initMermaid(isDark)
  }, [])

  useEffect(() => {
    mermaidBlocksRef.current = renderResult?.mermaidBlocks ?? []
  }, [renderResult])

  useEffect(() => {
    const observer = new MutationObserver(() => {
      const isDark = document.documentElement.classList.contains('dark')
      updateMermaidTheme(isDark)
      const blocks = mermaidBlocksRef.current
      for (const block of blocks) {
        const el = document.getElementById(block.id)
        if (el?.querySelector('svg')) {
          void renderMermaidBlock(block, isDark)
        }
      }
    })

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    })

    return () => observer.disconnect()
  }, [])
}
