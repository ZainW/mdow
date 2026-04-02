import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { initMarkdown, renderMarkdown, type RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlocks, updateMermaidTheme } from '../lib/mermaid'
import { useDocumentSearch } from '../hooks/useDocumentSearch'
import { useAppStore, type Tab } from '../store/app-store'
import { getContentFontFamily, getCodeFontFamily } from './SettingsDialog'
import { SearchBar } from './SearchBar'
import { Button } from './ui/button'
import { ArrowsLeftRight } from '@phosphor-icons/react'
import { ZoomIndicator } from './ZoomIndicator'

interface MarkdownViewProps {
  tab: Tab
}

export function MarkdownView({ tab }: MarkdownViewProps) {
  const [ready, setReady] = useState(false)
  const [themeKey, setThemeKey] = useState(0)
  const mermaidBlocksRef = useRef<RenderResult['mermaidBlocks']>([])
  const contentRef = useRef<HTMLDivElement>(null)
  const scrollRef = useRef<HTMLDivElement>(null)
  const prevTabIdRef = useRef(tab.id)
  const [searchQuery, setSearchQuery] = useState('')
  const wideMode = useAppStore((s) => s.wideMode)
  const toggleWideMode = useAppStore((s) => s.toggleWideMode)
  const zoomLevel = useAppStore((s) => s.zoomLevel)
  const updateTabScroll = useAppStore((s) => s.updateTabScroll)
  const searchOpen = useAppStore((s) => s.searchOpen)
  const setSearchOpen = useAppStore((s) => s.setSearchOpen)
  const contentFont = useAppStore((s) => s.contentFont)
  const codeFont = useAppStore((s) => s.codeFont)
  const fontSize = useAppStore((s) => s.fontSize)
  const lineHeight = useAppStore((s) => s.lineHeight)

  // Synchronous HTML computation — updates in the same render as the tab switch,
  // preventing intermediate frames with stale content that cause scrollbar flash
  const html = useMemo(() => {
    if (!ready || !tab.content) return ''
    const result = renderMarkdown(tab.content)
    mermaidBlocksRef.current = result.mermaidBlocks
    return result.html
    // eslint-disable-next-line react-hooks/exhaustive-deps -- themeKey forces re-render on theme change
  }, [ready, tab.content, themeKey])

  const { highlightedHtml, matchCount, currentIndex, next, prev, clear } = useDocumentSearch(
    contentRef,
    searchOpen ? searchQuery : '',
    html,
  )

  useEffect(() => {
    const isDark = document.documentElement.classList.contains('dark')
    void initMarkdown().then(() => {
      initMermaid(isDark)
      setReady(true)
    })
  }, [])

  useEffect(() => {
    if (mermaidBlocksRef.current.length > 0) {
      void renderMermaidBlocks(mermaidBlocksRef.current)
    }
  }, [html])

  useEffect(() => {
    const observer = new MutationObserver(() => {
      const isDark = document.documentElement.classList.contains('dark')
      updateMermaidTheme(isDark)
      setThemeKey((k) => k + 1)
    })

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    })

    return () => observer.disconnect()
  }, [])

  // Copy code button handler
  useEffect(() => {
    const container = contentRef.current
    if (!container) return
    const handler = (e: MouseEvent) => {
      if (!(e.target instanceof HTMLElement)) return
      const btn = e.target.closest('.copy-code-btn')
      if (!btn) return
      const encoded = btn.getAttribute('data-code')
      if (!encoded) return
      const code = decodeURIComponent(atob(encoded))
      void navigator.clipboard.writeText(code)
      btn.textContent = 'Copied!'
      setTimeout(() => {
        btn.textContent = 'Copy'
      }, 2000)
    }
    container.addEventListener('click', handler)
    return () => container.removeEventListener('click', handler)
  }, [html])

  // Scroll position: save on scroll (debounced)
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return
    let timer: ReturnType<typeof setTimeout>
    const handler = () => {
      clearTimeout(timer)
      timer = setTimeout(() => {
        updateTabScroll(tab.id, el.scrollTop)
      }, 150)
    }
    el.addEventListener('scroll', handler, { passive: true })
    return () => {
      clearTimeout(timer)
      el.removeEventListener('scroll', handler)
    }
  }, [tab.id, updateTabScroll])

  // Scroll position: restore on tab switch — useLayoutEffect runs before paint,
  // so the browser never renders an intermediate frame at the wrong scroll position
  useLayoutEffect(() => {
    if (prevTabIdRef.current !== tab.id) {
      const el = scrollRef.current
      if (el) {
        el.scrollTo(0, tab.scrollPosition)
      }
      prevTabIdRef.current = tab.id
    }
  }, [tab.id, tab.scrollPosition])

  if (!ready) {
    return <div className="flex-1 overflow-y-auto px-12 py-8 opacity-50">Loading...</div>
  }

  const handleCloseSearch = () => {
    setSearchOpen(false)
    setSearchQuery('')
    clear()
  }

  // Content is from local markdown files rendered by md4x — trusted source, not user-generated web content
  return (
    <div ref={scrollRef} className="group/content relative flex-1 overflow-y-auto">
      {searchOpen && (
        <SearchBar
          matchCount={matchCount}
          currentIndex={currentIndex}
          onNext={next}
          onPrev={prev}
          onClose={handleCloseSearch}
          onQueryChange={setSearchQuery}
        />
      )}
      <Button
        variant="ghost"
        size="icon-xs"
        className="absolute top-2 right-3 z-10 text-muted-foreground opacity-0 transition-opacity hover:opacity-100 group-hover/content:opacity-60"
        onClick={toggleWideMode}
        title={wideMode ? 'Constrained width' : 'Full width'}
      >
        <ArrowsLeftRight />
      </Button>
      <div
        ref={contentRef}
        className="mx-auto px-12 py-8 text-foreground markdown-body transition-[max-width] duration-200 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={
          {
            maxWidth: wideMode ? '100%' : '52rem',
            '--md-content-font': getContentFontFamily(contentFont),
            '--md-code-font': getCodeFontFamily(codeFont),
            '--md-font-size': `${fontSize * (zoomLevel / 100)}px`,
            '--md-line-height': lineHeight,
          } as React.CSSProperties
        }
        dangerouslySetInnerHTML={{ __html: highlightedHtml }}
      />
      <ZoomIndicator />
    </div>
  )
}
