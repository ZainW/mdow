import { useCallback, useEffect, useReducer, useRef, type RefObject } from 'react'
import { findMarkdownScroller, scrollToTarget } from '../lib/scroll-to'
import {
  clearSearchHighlights,
  findSearchRanges,
  paintSearchHighlights,
} from '../lib/search-highlight'

interface SearchState {
  matchCount: number
  currentIndex: number
  /** Bumped whenever the active match should be scrolled into view. */
  reveal: number
}

type SearchAction =
  | { type: 'set-results'; matchCount: number }
  | { type: 'refresh-results'; matchCount: number }
  | { type: 'set-index'; currentIndex: number }
  | { type: 'clear' }

function searchReducer(state: SearchState, action: SearchAction): SearchState {
  switch (action.type) {
    case 'set-results':
      return {
        matchCount: action.matchCount,
        currentIndex: action.matchCount > 0 ? 0 : -1,
        reveal: state.reveal + 1,
      }
    case 'refresh-results':
      // The document changed under an open search; keep the reader's place without jumping.
      return {
        ...state,
        matchCount: action.matchCount,
        currentIndex:
          action.matchCount > 0
            ? Math.min(Math.max(state.currentIndex, 0), action.matchCount - 1)
            : -1,
      }
    case 'set-index':
      return { ...state, currentIndex: action.currentIndex, reveal: state.reveal + 1 }
    case 'clear':
      return { matchCount: 0, currentIndex: -1, reveal: state.reveal }
    default:
      return state
  }
}

const QUERY_DEBOUNCE_MS = 120
const MUTATION_DEBOUNCE_MS = 150

export function useDocumentSearch(
  containerRef: RefObject<HTMLElement | null>,
  query: string,
  renderKey: unknown,
) {
  const [{ matchCount, currentIndex, reveal }, dispatch] = useReducer(searchReducer, {
    matchCount: 0,
    currentIndex: -1,
    reveal: 0,
  })
  const rangesRef = useRef<Range[]>([])
  const indexRef = useRef(currentIndex)
  const revealedRef = useRef(reveal)

  useEffect(() => {
    const container = containerRef.current
    if (!container || !query) {
      rangesRef.current = []
      clearSearchHighlights()
      dispatch({ type: 'clear' })
      return undefined
    }

    const search = (action: 'set-results' | 'refresh-results') => {
      const ranges = findSearchRanges(container, query)
      rangesRef.current = ranges
      const index =
        action === 'set-results' ? 0 : Math.min(Math.max(indexRef.current, 0), ranges.length - 1)
      paintSearchHighlights(ranges, index)
      dispatch({ type: action, matchCount: ranges.length })
    }

    const timer = setTimeout(() => search('set-results'), QUERY_DEBOUNCE_MS)

    // Blocks re-render under an open search (code highlighting, diagrams, live reload), which
    // detaches the matched text nodes. Re-run quietly when that happens.
    let mutationTimer: ReturnType<typeof setTimeout> | undefined
    const observer = new MutationObserver(() => {
      clearTimeout(mutationTimer)
      mutationTimer = setTimeout(() => search('refresh-results'), MUTATION_DEBOUNCE_MS)
    })
    observer.observe(container, { childList: true, subtree: true, characterData: true })

    return () => {
      clearTimeout(timer)
      clearTimeout(mutationTimer)
      observer.disconnect()
      clearSearchHighlights()
    }
  }, [containerRef, query, renderKey])

  useEffect(() => {
    indexRef.current = currentIndex
    const ranges = rangesRef.current
    if (currentIndex < 0 || !ranges[currentIndex]) return undefined
    paintSearchHighlights(ranges, currentIndex)
    // Scroll only for a new query or an explicit next/prev, not for quiet refreshes.
    if (revealedRef.current === reveal) return undefined
    revealedRef.current = reveal
    const container = containerRef.current
    const scroller = container && findMarkdownScroller(container)
    if (!scroller) return undefined
    return scrollToTarget(scroller, ranges[currentIndex], { block: 'center' })
  }, [reveal, currentIndex, containerRef])

  const next = useCallback(() => {
    if (matchCount > 0) {
      dispatch({ type: 'set-index', currentIndex: (currentIndex + 1) % matchCount })
    }
  }, [matchCount, currentIndex])

  const prev = useCallback(() => {
    if (matchCount > 0) {
      dispatch({ type: 'set-index', currentIndex: (currentIndex - 1 + matchCount) % matchCount })
    }
  }, [matchCount, currentIndex])

  const clear = useCallback(() => {
    rangesRef.current = []
    clearSearchHighlights()
    dispatch({ type: 'clear' })
  }, [])

  return { matchCount, currentIndex, next, prev, clear }
}
