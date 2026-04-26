import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { initMarkdown, renderMarkdown, type RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlocks, updateMermaidTheme } from '../lib/mermaid'
import { useDocumentSearch } from '../hooks/useDocumentSearch'
import { useAppStore, type Tab } from '../store/app-store'
import { getContentFontFamily, getCodeFontFamily } from './SettingsDialog'
import { SearchBar } from './SearchBar'
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
  const zoomLevel = useAppStore((s) => s.zoomLevel)
  const updateTabScroll = useAppStore((s) => s.updateTabScroll)
  const searchOpen = useAppStore((s) => s.searchOpen)
  const setSearchOpen = useAppStore((s) => s.setSearchOpen)
  const contentFont = useAppStore((s) => s.contentFont)
  const codeFont = useAppStore((s) => s.codeFont)
  const fontSize = useAppStore((s) => s.fontSize)
  const lineHeight = useAppStore((s) => s.lineHeight)

  const setDocHeadings = useAppStore((s) => s.setDocHeadings)
  const setActiveHeadingId = useAppStore((s) => s.setActiveHeadingId)

  const [renderResult, setRenderResult] = useState<RenderResult>({
    html: '',
    mermaidBlocks: [],
    headings: [],
  })
  const lastRenderedTabIdRef = useRef(tab.id)

  useEffect(() => {
    if (!ready) return undefined
    if (!tab.content) {
      setRenderResult({ html: '', mermaidBlocks: [], headings: [] })
      return undefined
    }
    // Clear stale content on tab switch to avoid showing the old document briefly.
    // For same-tab re-renders (theme change, content edit), keep showing the
    // previous html until the new render lands to avoid a flash of empty.
    if (lastRenderedTabIdRef.current !== tab.id) {
      lastRenderedTabIdRef.current = tab.id
      setRenderResult({ html: '', mermaidBlocks: [], headings: [] })
    }
    let cancelled = false
    void renderMarkdown(tab.content).then((res) => {
      if (cancelled) return
      setRenderResult(res)
    })
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- themeKey forces re-render on theme change
  }, [ready, tab.id, tab.content, themeKey])

  const html = renderResult.html
  mermaidBlocksRef.current = renderResult.mermaidBlocks

  useEffect(() => {
    setDocHeadings(renderResult.headings)
    setActiveHeadingId(renderResult.headings[0]?.id ?? null)
  }, [renderResult.headings, setDocHeadings, setActiveHeadingId])

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
    if (!container) return undefined
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

  // Scroll-spy: track which heading is currently in view
  useEffect(() => {
    const root = scrollRef.current
    const container = contentRef.current
    if (!root || !container) return undefined
    const headingEls = container.querySelectorAll<HTMLElement>('h1[id], h2[id], h3[id], h4[id]')
    if (headingEls.length === 0) return undefined

    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((e) => e.isIntersecting)
          .toSorted((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
        if (visible[0]) {
          setActiveHeadingId(visible[0].target.id)
        }
      },
      { root, rootMargin: '0px 0px -75% 0px', threshold: 0 },
    )
    for (const el of headingEls) observer.observe(el)
    return () => observer.disconnect()
  }, [html, setActiveHeadingId])

  // Scroll position: save on scroll (debounced)
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return undefined
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

  // Content is from local markdown files rendered by comark — trusted source, not user-generated web content
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
      <div
        ref={contentRef}
        className="mx-auto px-12 py-8 text-foreground markdown-body transition-[max-width] duration-200 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={
          {
            maxWidth: wideMode ? '100%' : '52rem',
            '--md-content-font': getContentFontFamily(contentFont),
            '--md-code-font': getCodeFontFamily(codeFont),
            '--md-font-size': `${fontSize * (zoomLevel / 100)}px`,
            '--md-line-height': String(lineHeight),
          } as React.CSSProperties
        }
        dangerouslySetInnerHTML={{ __html: highlightedHtml }}
      />
      <ZoomIndicator />
    </div>
  )
}
