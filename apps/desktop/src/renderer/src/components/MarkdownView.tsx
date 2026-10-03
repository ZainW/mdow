import { useRef, useState, type CSSProperties } from 'react'
import { useShallow } from 'zustand/react/shallow'
import { useDocumentSearch } from '../hooks/useDocumentSearch'
import { useMarkdownRender } from '../hooks/useMarkdownRender'
import { useScrollRestoration } from '../hooks/useScrollRestoration'
import { useHeadingObserver } from '../hooks/useHeadingObserver'
import { useMermaidThemeSync } from '../hooks/useMermaidThemeSync'
import { useContentClickHandlers } from '../hooks/useContentClickHandlers'
import { useDocumentReview } from '../hooks/useDocumentReview'
import { useAppStore, type Tab } from '../store/app-store'
import {
  MARKDOWN_FONT_SIZE,
  MARKDOWN_LINE_HEIGHT,
  getContentFontFamily,
  getCodeFontFamily,
} from '../lib/typography'
import { cn } from '../lib/utils'
import { SearchBar } from './SearchBar'
import { ZoomIndicator } from './ZoomIndicator'
import { DocumentSkeleton } from './DocumentSkeleton'
import { MarkdownContent } from './markdown/components'
import { MarkdownReview, useReviewRender } from './review/MarkdownReview'
import { ReviewBar } from './review/ReviewBar'
import type { ReadingWidth } from '../../../shared/types'

const READING_WIDTHS = {
  standard: '48rem',
  comfortable: '56rem',
  wide: '68rem',
} as const satisfies Record<ReadingWidth, string>

interface MarkdownViewProps {
  tab: Tab
  isActive?: boolean
  onOpenMarkdownLink?: (path: string) => void
}

export function MarkdownView({ tab, isActive = true, onOpenMarkdownLink }: MarkdownViewProps) {
  const contentRef = useRef<HTMLDivElement>(null)
  const scrollRef = useRef<HTMLDivElement>(null)
  const [searchQuery, setSearchQuery] = useState('')
  const [retryKey, setRetryKey] = useState(0)

  const { wideMode, readingWidth, zoomLevel, contentFont, codeFont } = useAppStore(
    useShallow((s) => ({
      wideMode: s.wideMode,
      readingWidth: s.readingWidth,
      zoomLevel: s.zoomLevel,
      contentFont: s.contentFont,
      codeFont: s.codeFont,
    })),
  )

  const { searchOpen, setSearchOpen, updateTabScroll } = useAppStore(
    useShallow((s) => ({
      searchOpen: s.searchOpen,
      setSearchOpen: s.setSearchOpen,
      updateTabScroll: s.updateTabScroll,
    })),
  )

  const { renderResult, renderError, isRendering, isPartial, renderVersion } = useMarkdownRender({
    tabId: tab.id,
    content: tab.content,
    retryKey,
    allowPreview: tab.scrollPosition === 0 && !tab.scrollAnchor,
  })

  // A change the companion suggests for this document is reviewed right here, in place.
  const review = useDocumentReview(tab.path, tab.content)
  const reviewRender = useReviewRender(review)

  const { matchCount, currentIndex, next, prev, clear } = useDocumentSearch(
    contentRef,
    searchOpen && isActive ? searchQuery : '',
    renderVersion,
  )

  useMermaidThemeSync(renderResult)
  useHeadingObserver({ scrollRef, contentRef, renderResult })
  useScrollRestoration({
    scrollRef,
    contentRef,
    tabId: tab.id,
    scrollPosition: tab.scrollPosition,
    scrollAnchor: tab.scrollAnchor,
    renderVersion,
    partial: isPartial,
    failed: renderError,
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
    <div
      ref={scrollRef}
      data-markdown-scroller=""
      className="group/content relative flex-1 overflow-y-auto"
    >
      {searchOpen &&
        isActive && (
          // Zero-height sticky rail: the bar floats over the document instead of pushing it down.
          <div className="pointer-events-none sticky top-0 z-20 flex h-0 items-start justify-end">
            <SearchBar
              matchCount={matchCount}
              currentIndex={currentIndex}
              onNext={next}
              onPrev={prev}
              onClose={handleCloseSearch}
              onQueryChange={setSearchQuery}
            />
          </div>
        )}
      <div
        data-reading-frame=""
        className={cn('flex min-h-full w-full', wideMode ? 'justify-start' : 'justify-center')}
      >
        <div
          ref={contentRef}
          id={`tabpanel-${tab.id}`}
          role="tabpanel"
          aria-labelledby={`tab-${tab.id}`}
          aria-busy={isRendering}
          data-reading-column=""
          className={cn(
            'markdown-body box-border w-full min-w-0 px-12 py-8 text-foreground',
            reviewRender && 'pb-28',
          )}
          style={
            {
              maxWidth: wideMode ? undefined : READING_WIDTHS[readingWidth],
              '--md-content-font': getContentFontFamily(contentFont),
              '--md-code-font': getCodeFontFamily(codeFont),
              '--md-font-size': `${MARKDOWN_FONT_SIZE * (zoomLevel / 100)}px`,
              '--md-line-height': String(MARKDOWN_LINE_HEIGHT),
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
          ) : review && reviewRender ? (
            <MarkdownReview key={review.toolCallId} rendered={reviewRender} docPath={tab.path} />
          ) : renderResult ? (
            // Keyed by tab, not render: a reload of the same file reconciles in place (sections
            // with unchanged content are skipped), while switching documents starts fresh.
            <div key={tab.id}>
              <MarkdownContent result={renderResult} docPath={tab.path} />
              {isPartial && (
                <output className="block py-6 text-center text-xs text-muted-foreground">
                  Loading the rest of the document…
                </output>
              )}
            </div>
          ) : isRendering ? (
            <DocumentSkeleton />
          ) : null}
        </div>
      </div>
      {review && reviewRender && (
        <ReviewBar
          key={review.toolCallId}
          review={review}
          changeCount={reviewRender.changeCount}
          containerRef={contentRef}
        />
      )}
      <ZoomIndicator />
    </div>
  )
}
