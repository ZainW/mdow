import {
  useEffect,
  useLayoutEffect,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
} from 'react'
import { useShallow } from 'zustand/react/shallow'
import { renderMarkdown, type RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlock, updateMermaidTheme } from '../lib/mermaid'
import { useDocumentSearch } from '../hooks/useDocumentSearch'
import { useAppStore, type Tab } from '../store/app-store'
import { isMarkdownPath } from '../lib/path-utils'
import { getContentFontFamily, getCodeFontFamily } from '../lib/typography'
import { SearchBar } from './SearchBar'
import { ZoomIndicator } from './ZoomIndicator'
import { DocumentSkeleton } from './DocumentSkeleton'
import { MarkdownContent, resolveRelativePath } from './markdown/components'

interface MarkdownViewProps {
  tab: Tab
  onOpenMarkdownLink?: (path: string) => void
}

function getTabRenderFromStore(tabId: string): RenderResult | undefined {
  return useAppStore.getState().renderCache.get(tabId)
}

function setTabRenderInStore(tabId: string, result: RenderResult): void {
  useAppStore.getState().setRenderCache(tabId, result)
}

interface RenderUi {
  result: RenderResult | null
  version: number
  error: boolean
}

type RenderAction =
  | { type: 'reset' }
  | { type: 'clear-tab' }
  | { type: 'start' }
  | { type: 'ready'; result: RenderResult; version: number }
  | { type: 'error' }

function renderReducer(state: RenderUi, action: RenderAction): RenderUi {
  switch (action.type) {
    case 'reset':
      return { result: null, version: 0, error: false }
    case 'clear-tab':
      return { result: null, version: state.version, error: false }
    case 'start':
      return { ...state, error: false }
    case 'ready':
      return { result: action.result, version: action.version, error: false }
    case 'error':
      return { result: null, version: state.version, error: true }
    default:
      return state
  }
}

export function MarkdownView({ tab, onOpenMarkdownLink }: MarkdownViewProps) {
  const contentRef = useRef<HTMLDivElement>(null)
  const scrollRef = useRef<HTMLDivElement>(null)
  const prevTabIdRef = useRef(tab.id)
  const [searchQuery, setSearchQuery] = useState('')
  const [retryKey, setRetryKey] = useState(0)

  const { wideMode, zoomLevel, contentFont, codeFont, fontSize, lineHeight } = useAppStore(
    useShallow((s) => ({
      wideMode: s.wideMode,
      zoomLevel: s.zoomLevel,
      contentFont: s.contentFont,
      codeFont: s.codeFont,
      fontSize: s.fontSize,
      lineHeight: s.lineHeight,
    })),
  )

  const { searchOpen, setSearchOpen, updateTabScroll } = useAppStore(
    useShallow((s) => ({
      searchOpen: s.searchOpen,
      setSearchOpen: s.setSearchOpen,
      updateTabScroll: s.updateTabScroll,
    })),
  )

  const [renderUi, dispatchRender] = useReducer(renderReducer, {
    result: null,
    version: 0,
    error: false,
  })
  const renderVersionRef = useRef(0)
  const lastRenderedTabIdRef = useRef(tab.id)
  const mermaidBlocksRef = useRef<RenderResult['mermaidBlocks']>([])
  const renderResult = renderUi.result
  const renderError = renderUi.error
  const isRendering = Boolean(tab.content) && !renderResult && !renderError

  useEffect(() => {
    if (!tab.content) {
      dispatchRender({ type: 'reset' })
      return undefined
    }
    if (lastRenderedTabIdRef.current !== tab.id) {
      lastRenderedTabIdRef.current = tab.id
      dispatchRender({ type: 'clear-tab' })
    }

    const cached = getTabRenderFromStore(tab.id)
    if (cached) {
      renderVersionRef.current += 1
      dispatchRender({
        type: 'ready',
        result: cached,
        version: renderVersionRef.current,
      })
      return undefined
    }

    let cancelled = false
    dispatchRender({ type: 'start' })
    void renderMarkdown(tab.content)
      .then((res) => {
        if (cancelled) return
        setTabRenderInStore(tab.id, res)
        renderVersionRef.current += 1
        dispatchRender({
          type: 'ready',
          result: res,
          version: renderVersionRef.current,
        })
      })
      .catch(() => {
        if (cancelled) return
        dispatchRender({ type: 'error' })
      })
    return () => {
      cancelled = true
    }
  }, [tab.id, tab.content, retryKey])

  useEffect(() => {
    const headings = renderResult?.headings ?? []
    useAppStore.setState({
      docHeadings: headings,
      activeHeadingId: headings[0]?.id ?? null,
    })
  }, [renderResult])

  const { matchCount, currentIndex, next, prev, clear } = useDocumentSearch(
    contentRef,
    searchOpen ? searchQuery : '',
    renderUi.version ?? 0,
  )

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

  useEffect(() => {
    const root = scrollRef.current
    const container = contentRef.current
    if (!root || !container) return undefined
    const headingEls = container.querySelectorAll<HTMLElement>(
      'h1[id], h2[id], h3[id], h4[id], h5[id], h6[id]',
    )
    if (headingEls.length === 0) return undefined

    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((e) => e.isIntersecting)
          .toSorted((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
        if (visible[0]) {
          useAppStore.getState().setActiveHeadingId(visible[0].target.id)
        }
      },
      { root, rootMargin: '0px 0px -75% 0px', threshold: 0 },
    )
    for (const el of headingEls) observer.observe(el)
    return () => observer.disconnect()
  }, [renderResult])

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

  useLayoutEffect(() => {
    if (prevTabIdRef.current !== tab.id) {
      const el = scrollRef.current
      if (el) {
        el.scrollTo(0, tab.scrollPosition)
      }
      prevTabIdRef.current = tab.id
    }
  }, [tab.id, tab.scrollPosition])

  useEffect(() => {
    const container = contentRef.current
    if (!container) return undefined

    const handleCopyClick = (event: MouseEvent) => {
      const btn = (event.target as Element).closest<HTMLButtonElement>('[data-copy-code]')
      if (!btn || !container.contains(btn)) return
      const wrapper = btn.closest('.code-block-wrapper')
      const code = wrapper?.querySelector('pre')?.textContent ?? ''
      void navigator.clipboard.writeText(code).then(() => {
        btn.setAttribute('data-copied', 'true')
        btn.setAttribute('aria-label', 'Copied')
        window.setTimeout(() => {
          btn.removeAttribute('data-copied')
          btn.setAttribute('aria-label', 'Copy code')
        }, 1500)
      })
    }

    const handleLinkClick = (event: MouseEvent) => {
      const anchor = (event.target as Element).closest('a')
      if (!anchor || !container.contains(anchor)) return
      const href = anchor.getAttribute('href')
      if (!href) return

      if (href.startsWith('#')) {
        event.preventDefault()
        const id = decodeURIComponent(href.slice(1))
        const target = container.querySelector(`#${CSS.escape(id)}`) ?? document.getElementById(id)
        target?.scrollIntoView({ behavior: 'smooth', block: 'start' })
        return
      }

      if (/^[a-zA-Z][a-zA-Z\d+\-.]*:/.test(href) && !href.startsWith('mdow-local:')) {
        event.preventDefault()
        void window.api.openExternal(href)
        return
      }

      const resolved = resolveRelativePath(href, tab.path)
      if (isMarkdownPath(resolved)) {
        event.preventDefault()
        window.dispatchEvent(
          new CustomEvent('mdow:open-markdown-link', { detail: { path: resolved } }),
        )
        onOpenMarkdownLink?.(resolved)
      }
    }

    container.addEventListener('click', handleCopyClick)
    container.addEventListener('click', handleLinkClick)
    return () => {
      container.removeEventListener('click', handleCopyClick)
      container.removeEventListener('click', handleLinkClick)
    }
  }, [tab.path, renderResult, onOpenMarkdownLink])

  const handleCloseSearch = () => {
    setSearchOpen(false)
    setSearchQuery('')
    clear()
  }

  const handleRetry = () => {
    setRetryKey((key) => key + 1)
  }

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
        id={`tabpanel-${tab.id}`}
        role="tabpanel"
        aria-labelledby={`tab-${tab.id}`}
        aria-busy={isRendering}
        className="mx-auto px-12 py-8 text-foreground markdown-body transition-[max-width] duration-200 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={
          {
            maxWidth: wideMode ? '100%' : '48rem',
            '--md-content-font': getContentFontFamily(contentFont),
            '--md-code-font': getCodeFontFamily(codeFont),
            '--md-font-size': `${fontSize * (zoomLevel / 100)}px`,
            '--md-line-height': String(lineHeight),
          } as CSSProperties
        }
      >
        {renderError ? (
          <div className="rounded-lg border border-destructive/20 bg-destructive/5 p-4 text-sm">
            <p className="text-destructive">This document could not be rendered.</p>
            <button
              type="button"
              className="mt-3 rounded-md border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted"
              onClick={handleRetry}
            >
              Try again
            </button>
          </div>
        ) : renderUi.result ? (
          <div key={renderUi.version} className="document-content-in">
            <MarkdownContent result={renderUi.result} docPath={tab.path} />
          </div>
        ) : isRendering ? (
          <DocumentSkeleton />
        ) : null}
      </div>
      <ZoomIndicator />
    </div>
  )
}
