import { useEffect, useRef, useState } from 'react'
import { initMarkdown, renderMarkdown, type RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlocks, updateMermaidTheme } from '../lib/mermaid'
import { useAppStore } from '../store/app-store'
import { Button } from './ui/button'
import { ArrowLeftRightIcon } from 'lucide-react'

interface MarkdownViewProps {
  content: string
}

export function MarkdownView({ content }: MarkdownViewProps) {
  const [html, setHtml] = useState('')
  const [ready, setReady] = useState(false)
  const mermaidBlocksRef = useRef<RenderResult['mermaidBlocks']>([])
  const wideMode = useAppStore((s) => s.wideMode)
  const toggleWideMode = useAppStore((s) => s.toggleWideMode)

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

  // Content is from local markdown files rendered by md4x — trusted source
  return (
    <div className="group/content relative flex-1 overflow-y-auto">
      <Button
        variant="ghost"
        size="icon-xs"
        className="absolute top-2 right-3 z-10 text-muted-foreground opacity-0 transition-opacity hover:opacity-100 group-hover/content:opacity-60"
        onClick={toggleWideMode}
        title={wideMode ? 'Constrained width' : 'Full width'}
      >
        <ArrowLeftRightIcon />
      </Button>
      <div
        className="mx-auto px-12 py-8 text-foreground markdown-body transition-[max-width] duration-300 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={{ maxWidth: wideMode ? '100%' : '65ch' }}
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </div>
  )
}
