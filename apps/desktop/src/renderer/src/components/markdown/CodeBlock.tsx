import { Fragment, useEffect, useRef, useState, type HTMLAttributes, type ReactNode } from 'react'
import {
  getCachedHighlight,
  highlightCode,
  isHighlightableLanguage,
  type HighlightedLines,
} from '../../lib/highlight'
import { onceNearViewport } from '../../lib/near-viewport'
import { cn } from '../../lib/utils'

type CodeBlockProps = HTMLAttributes<HTMLPreElement> & {
  language?: string
  class?: string
  /** Raw fence body, attached by `renderMarkdown`. */
  code?: string
  highlights?: unknown
  filename?: string
  meta?: string
}

function renderLines(lines: HighlightedLines, highlights: unknown): ReactNode {
  const marked = Array.isArray(highlights) ? new Set(highlights) : null
  // Lines and tokens are positional and never reorder, so indexes are stable keys.
  return lines.map((line, index) => (
    // oxlint-disable-next-line react/no-array-index-key
    <Fragment key={index}>
      <span className={marked?.has(index + 1) ? 'line highlight' : 'line'}>
        {line.map((token, tokenIndex) => (
          // oxlint-disable-next-line react/no-array-index-key
          <span key={tokenIndex} style={token.style}>
            {token.content}
          </span>
        ))}
      </span>
      {index < lines.length - 1 ? '\n' : null}
    </Fragment>
  ))
}

/**
 * Syntax highlighting is deferred until a block nears the viewport: a huge document renders as
 * plain monospace text first, and only the blocks a reader actually reaches pay for Shiki's
 * tokenizer and the extra spans.
 */
function useLazyHighlight(
  ref: React.RefObject<HTMLElement | null>,
  code: string | undefined,
  language: string | undefined,
): HighlightedLines | null {
  const canHighlight = typeof code === 'string' && isHighlightableLanguage(language)
  const key = canHighlight ? `${language}\u0000${code}` : null
  // Remember which source the tokens belong to: a live reload can hand this block new code.
  const [highlighted, setHighlighted] = useState<{ key: string; lines: HighlightedLines } | null>(
    null,
  )
  const cached = canHighlight ? getCachedHighlight(code, language) : undefined

  useEffect(() => {
    const el = ref.current
    if (!el || !canHighlight || !key || getCachedHighlight(code, language)) return undefined

    let cancelled = false
    const stop = onceNearViewport(el, () => {
      void highlightCode(code, language).then((lines) => {
        if (!cancelled && lines) setHighlighted({ key, lines })
      })
    })
    return () => {
      cancelled = true
      stop()
    }
  }, [ref, canHighlight, key, code, language])

  if (!key) return null
  return cached ?? (highlighted?.key === key ? highlighted.lines : null)
}

export function CodeBlock({
  children,
  language,
  code,
  highlights,
  filename: _filename,
  meta: _meta,
  ...props
}: CodeBlockProps) {
  const ref = useRef<HTMLDivElement>(null)
  const lines = useLazyHighlight(ref, code, language)
  const { class: className, ...preProps } = props

  return (
    <div ref={ref} className="code-block-wrapper relative">
      <div className="code-block-header">
        <span className="code-lang-badge">{language}</span>
        {/* Icons and label live in CSS (masks + generated content): a long document can hold
            thousands of code blocks, and per-block SVG subtrees add up. */}
        <button
          className="copy-code-btn"
          type="button"
          data-copy-code
          aria-label="Copy code"
          title="Copy code"
        >
          <span className="copy-code-icon" aria-hidden />
        </button>
      </div>
      <pre className={cn(className, lines && 'shiki')} {...preProps}>
        {lines ? (
          // A fresh key swaps the whole <code> element, so text nodes the in-page search wrapped
          // in <mark>s are dropped with their parent instead of confusing React's reconciler.
          <code key="highlighted" className={language ? `language-${language}` : undefined}>
            {renderLines(lines, highlights)}
          </code>
        ) : (
          children
        )}
      </pre>
    </div>
  )
}
