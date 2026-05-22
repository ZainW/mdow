import { useCallback, useEffect, useReducer, useRef, type RefObject } from 'react'
import { applySearchHighlights, removeSearchHighlights } from '../lib/search-highlight'

interface SearchState {
  matchCount: number
  currentIndex: number
}

type SearchAction =
  | { type: 'set-results'; matchCount: number }
  | { type: 'set-index'; currentIndex: number }
  | { type: 'clear' }

function searchReducer(state: SearchState, action: SearchAction): SearchState {
  switch (action.type) {
    case 'set-results':
      return {
        matchCount: action.matchCount,
        currentIndex: action.matchCount > 0 ? 0 : -1,
      }
    case 'set-index':
      return { ...state, currentIndex: action.currentIndex }
    case 'clear':
      return { matchCount: 0, currentIndex: -1 }
    default:
      return state
  }
}

export function useDocumentSearch(
  containerRef: RefObject<HTMLElement | null>,
  query: string,
  renderKey: unknown,
) {
  const [{ matchCount, currentIndex }, dispatch] = useReducer(searchReducer, {
    matchCount: 0,
    currentIndex: -1,
  })
  const activeMarkRef = useRef<Element | null>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return undefined

    const timer = setTimeout(() => {
      removeSearchHighlights(container)
      activeMarkRef.current = null
      const count = applySearchHighlights(container, query)
      dispatch({ type: 'set-results', matchCount: count })
    }, 120)

    return () => {
      clearTimeout(timer)
      removeSearchHighlights(container)
    }
  }, [containerRef, query, renderKey])

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    if (activeMarkRef.current?.isConnected) {
      activeMarkRef.current.classList.remove('search-highlight-active')
    }
    activeMarkRef.current = null

    if (currentIndex < 0) return

    const mark = container.querySelector(`mark[data-match-index="${currentIndex}"]`)
    if (mark) {
      mark.classList.add('search-highlight-active')
      mark.scrollIntoView({ block: 'center', behavior: 'smooth' })
      activeMarkRef.current = mark
    }
  }, [currentIndex, containerRef])

  const next = useCallback(() => {
    if (matchCount > 0) {
      dispatch({
        type: 'set-index',
        currentIndex: (currentIndex + 1) % matchCount,
      })
    }
  }, [matchCount, currentIndex])

  const prev = useCallback(() => {
    if (matchCount > 0) {
      dispatch({
        type: 'set-index',
        currentIndex: (currentIndex - 1 + matchCount) % matchCount,
      })
    }
  }, [matchCount, currentIndex])

  const clear = useCallback(() => {
    const container = containerRef.current
    if (container) removeSearchHighlights(container)
    dispatch({ type: 'clear' })
    activeMarkRef.current = null
  }, [containerRef])

  return { matchCount, currentIndex, next, prev, clear }
}
