import { useRef, useState, type CSSProperties } from 'react'
import { useShallow } from 'zustand/react/shallow'
import { useDocumentSearch } from '../hooks/useDocumentSearch'
import { useMarkdownRender } from '../hooks/useMarkdownRender'
import { useScrollRestoration } from '../hooks/useScrollRestoration'
import { useHeadingObserver } from '../hooks/useHeadingObserver'
import { useMermaidThemeSync } from '../hooks/useMermaidThemeSync'
import { useContentClickHandlers } from '../hooks/useContentClickHandlers'
import { useAppStore, type Tab } from '../store/app-store'
import { getContentFontFamily, getCodeFontFamily } from '../lib/typography'
import { SearchBar } from './SearchBar'
import { ZoomIndicator } from './ZoomIndicator'
import { DocumentSkeleton } from './DocumentSkeleton'
import { MarkdownContent } from './markdown/components'

interface MarkdownViewProps {
  tab: Tab
  onOpenMarkdownLink?: (path: string) => void
}

export function MarkdownView({ tab, onOpenMarkdownLink }: MarkdownViewProps) {
  const contentRef = useRef<HTMLDivElement>(null)
  const scrollRef = useRef<HTMLDivElement>(null)
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

  const { renderResult, renderError, isRendering, renderVersion } = useMarkdownRender({
    tabId: tab.id,
    content: tab.content,
    retryKey,
  })

  const { matchCount, currentIndex, next, prev, clear } = useDocumentSearch(
    contentRef,
    searchOpen ? searchQuery : '',
    renderVersion,
  )

  useMermaidThemeSync(renderResult)
  useHeadingObserver({ scrollRef, contentRef, renderResult })
  useScrollRestoration({
    scrollRef,
    tabId: tab.id,
    scrollPosition: tab.scrollPosition,
    updateTabScroll,
  })
  useContentClickHandlers({
    contentRef,
    tabPath: tab.path,
    renderResult,
    onOpenMarkdownLink,
  })

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
        ) : renderResult ? (
          <div key={renderVersion} className="document-content-in">
            <MarkdownContent result={renderResult} docPath={tab.path} />
          </div>
        ) : isRendering ? (
          <DocumentSkeleton />
        ) : null}
      </div>
      <ZoomIndicator />
    </div>
  )
}
