import { useEffect, useRef, useState } from 'react'
import { initMarkdown, renderMarkdown, type RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlocks, updateMermaidTheme } from '../lib/mermaid'

interface MarkdownViewProps {
  content: string
}

export function MarkdownView({ content }: MarkdownViewProps) {
  const [html, setHtml] = useState('')
  const [ready, setReady] = useState(false)
  const mermaidBlocksRef = useRef<RenderResult['mermaidBlocks']>([])

  useEffect(() => {
    const isDark = document.documentElement.classList.contains('dark')
    void initMarkdown().then(() => {
      initMermaid(isDark)
      setReady(true)
    })
  }, [])

  useEffect(() => {
    if (!ready || !content) return

    const result = renderMarkdown(content)
    setHtml(result.html)
    mermaidBlocksRef.current = result.mermaidBlocks
  }, [content, ready])

  useEffect(() => {
    if (mermaidBlocksRef.current.length > 0) {
      void renderMermaidBlocks(mermaidBlocksRef.current)
    }
  }, [html])

  useEffect(() => {
    const observer = new MutationObserver(() => {
      const isDark = document.documentElement.classList.contains('dark')
      updateMermaidTheme(isDark)
      if (ready && content) {
        const result = renderMarkdown(content)
        setHtml(result.html)
        mermaidBlocksRef.current = result.mermaidBlocks
      }
    })

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    })

    return () => observer.disconnect()
  }, [ready, content])

  if (!ready) {
    return <div className="flex-1 overflow-y-auto px-12 py-8 opacity-50">Loading...</div>
  }

  return (
    <div
      className="flex-1 overflow-y-auto px-12 py-8 text-foreground markdown-body"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}
