import { useEffect, useRef, useState } from 'react'
import type { DocumentReview } from '../../hooks/useDocumentReview'
import type { RenderResult } from '../../lib/markdown'
import { renderDocument } from '../../lib/markdown-client'
import { buildReviewTree } from '../../lib/review-diff'
import { clearReviewHighlights, paintReviewHighlights } from '../../lib/review-highlight'
import { MarkdownContent } from '../markdown/components'

export interface ReviewRender {
  result: RenderResult
  changeCount: number
}

/**
 * Renders the proposed version of a document with its changes marked. Resolves to null until
 * both versions are parsed, so the reader keeps showing the current text instead of flashing.
 */
export function useReviewRender(review: DocumentReview | null): ReviewRender | null {
  const [rendered, setRendered] = useState<{ source: string; value: ReviewRender } | null>(null)
  const before = review?.before
  const after = review?.after

  useEffect(() => {
    if (before === undefined || after === undefined) return undefined
    let cancelled = false
    void Promise.all([renderDocument(before), renderDocument(after)])
      .then(([previous, proposed]) => {
        if (cancelled) return
        const { tree, changeCount } = buildReviewTree(previous.tree, proposed.tree)
        setRendered({ source: after, value: { result: { ...proposed, tree }, changeCount } })
      })
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [before, after])

  return review && rendered?.source === review.after ? rendered.value : null
}

/**
 * The proposed document, rendered like the reader renders it: replaced blocks sit above their
 * replacements and changed words are highlighted.
 */
export function MarkdownReview({ rendered, docPath }: { rendered: ReviewRender; docPath: string }) {
  const containerRef = useRef<HTMLDivElement>(null)

  // Repaint word marks after every DOM change: code blocks highlight lazily and replace their
  // text nodes, which would otherwise leave the marks pointing at detached nodes.
  useEffect(() => {
    const container = containerRef.current
    if (!container) return undefined
    let frame = requestAnimationFrame(() => paintReviewHighlights(container))
    const observer = new MutationObserver(() => {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(() => paintReviewHighlights(container))
    })
    observer.observe(container, { childList: true, subtree: true, characterData: true })
    return () => {
      cancelAnimationFrame(frame)
      observer.disconnect()
      clearReviewHighlights()
    }
  }, [rendered])

  return (
    <div ref={containerRef} data-review="">
      <MarkdownContent result={rendered.result} docPath={docPath} />
    </div>
  )
}
