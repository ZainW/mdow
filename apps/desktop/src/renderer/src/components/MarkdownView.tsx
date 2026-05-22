import { ComarkRenderer } from '@comark/react'
import {
  memo,
  useEffect,
  useLayoutEffect,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
  type HTMLAttributes,
} from 'react'
import { Check, Copy } from 'lucide-react'
import { renderMarkdown, type RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlocks, updateMermaidTheme } from '../lib/mermaid'
import { useDocumentSearch } from '../hooks/useDocumentSearch'
import { useAppStore, type Tab } from '../store/app-store'
import { getContentFontFamily, getCodeFontFamily } from '../lib/typography'
import { iconSize, iconStroke } from '../lib/icons'
import { SearchBar } from './SearchBar'
import { ZoomIndicator } from './ZoomIndicator'

interface MarkdownViewProps {
  tab: Tab
}

function CodeBlock({ children, ...props }: HTMLAttributes<HTMLPreElement>) {
  const preRef = useRef<HTMLPreElement>(null)
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current)
    }
  }, [])

  const handleCopy = () => {
    const code = preRef.current?.textContent ?? ''
    void navigator.clipboard.writeText(code)
    setCopied(true)
    if (timeoutRef.current) clearTimeout(timeoutRef.current)
    timeoutRef.current = setTimeout(() => setCopied(false), 1500)
  }

  return (
    <div className="code-block-wrapper relative">
      <pre {...props} ref={preRef}>
        {children}
      </pre>
      <button
        className="copy-code-btn"
        type="button"
        aria-label={copied ? 'Copied' : 'Copy code'}
        title="Copy code"
        data-copied={copied ? 'true' : undefined}
        onClick={handleCopy}
      >
        <Copy
          className="copy-icon copy-icon-default"
          size={iconSize.md}
          strokeWidth={iconStroke.default}
          aria-hidden
        />
        <Check
          className="copy-icon copy-icon-done"
          size={iconSize.md}
          strokeWidth={iconStroke.emphasis}
          aria-hidden
        />
      </button>
    </div>
  )
}

interface MermaidBlockProps extends HTMLAttributes<HTMLDivElement> {
  content?: string
}

function MermaidBlock({ content: _content, children: _children, ...props }: MermaidBlockProps) {
  return <div {...props} />
}

function TableWrap(props: HTMLAttributes<HTMLTableElement>) {
  // Wrap tables in a horizontal-scroll container so wide tables don't blow
  // out the markdown body. The wrapper carries the border + radius so the
  // table can scroll cleanly inside.
  return (
    <div className="table-wrap">
      <table {...props} />
    </div>
  )
}

const markdownComponents = {
  pre: CodeBlock,
  mermaid: MermaidBlock,
  table: TableWrap,
}

const MarkdownContent = memo(function MarkdownContent({ result }: { result: RenderResult }) {
  return <ComarkRenderer tree={result.tree} components={markdownComponents} />
})

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

export function MarkdownView({ tab }: MarkdownViewProps) {
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

  useEffect(() => {
    if (!tab.content) {
      dispatchRender({ type: 'reset' })
      return undefined
    }
    if (lastRenderedTabIdRef.current !== tab.id) {
      lastRenderedTabIdRef.current = tab.id
      dispatchRender({ type: 'clear-tab' })
    }
    let cancelled = false
    dispatchRender({ type: 'start' })
    void renderMarkdown(tab.content)
      .then((res) => {
        if (cancelled) return
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
  }, [tab.id, tab.content])

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
    if (renderResult?.mermaidBlocks.length) {
      void renderMermaidBlocks(renderResult.mermaidBlocks)
    }
  }, [renderResult])

  useEffect(() => {
    const observer = new MutationObserver(() => {
      const isDark = document.documentElement.classList.contains('dark')
      updateMermaidTheme(isDark)
      const blocks = mermaidBlocksRef.current
      if (blocks.length > 0) {
        void renderMermaidBlocks(blocks)
      }
    })

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    })

    return () => observer.disconnect()
  }, [])

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
          useAppStore.getState().setActiveHeadingId(visible[0].target.id)
        }
      },
      { root, rootMargin: '0px 0px -75% 0px', threshold: 0 },
    )
    for (const el of headingEls) observer.observe(el)
    return () => observer.disconnect()
  }, [renderResult])

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

  const handleCloseSearch = () => {
    setSearchOpen(false)
    setSearchQuery('')
    clear()
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
          <div className="rounded-lg border border-destructive/20 bg-destructive/5 p-4 text-sm text-destructive">
            This document could not be rendered.
          </div>
        ) : renderUi.result ? (
          <MarkdownContent key={renderUi.version} result={renderUi.result} />
        ) : null}
      </div>
      <ZoomIndicator />
    </div>
  )
}
