import { useEffect, useReducer, useRef } from 'react'
import type { RenderResult } from '../lib/markdown'
import { useAppStore } from '../store/app-store'

function getTabRenderFromStore(tabId: string): RenderResult | undefined {
  return useAppStore.getState().renderCache.get(tabId)
}

function setTabRenderInStore(tabId: string, result: RenderResult): void {
  useAppStore.getState().setRenderCache(tabId, result)
}

async function renderMarkdownContent(content: string): Promise<RenderResult> {
  const { renderMarkdown } = await import('../lib/markdown')
  return renderMarkdown(content)
}

export interface RenderUi {
  result: RenderResult | null
  version: number
  error: boolean
}

export type RenderAction =
  | { type: 'reset' }
  | { type: 'clear-tab' }
  | { type: 'start' }
  | { type: 'ready'; result: RenderResult; version: number }
  | { type: 'error' }

export function renderReducer(state: RenderUi, action: RenderAction): RenderUi {
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

export function useMarkdownRender({
  tabId,
  content,
  retryKey,
}: {
  tabId: string
  content: string
  retryKey: number
}): {
  renderResult: RenderResult | null
  renderError: boolean
  isRendering: boolean
  renderVersion: number
} {
  const [renderUi, dispatchRender] = useReducer(renderReducer, {
    result: null,
    version: 0,
    error: false,
  })
  const renderVersionRef = useRef(0)
  const lastRenderedTabIdRef = useRef(tabId)
  const renderResult = renderUi.result
  const renderError = renderUi.error

  useEffect(() => {
    if (!content) {
      dispatchRender({ type: 'reset' })
      return undefined
    }
    if (lastRenderedTabIdRef.current !== tabId) {
      lastRenderedTabIdRef.current = tabId
      dispatchRender({ type: 'clear-tab' })
    }

    const cached = getTabRenderFromStore(tabId)
    if (cached) {
      setTabRenderInStore(tabId, cached)
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
    void renderMarkdownContent(content)
      .then((res) => {
        if (cancelled) return
        setTabRenderInStore(tabId, res)
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
  }, [tabId, content, retryKey])

  useEffect(() => {
    const headings = renderResult?.headings ?? []
    useAppStore.setState({
      docHeadings: headings,
      activeHeadingId: headings[0]?.id ?? null,
    })
  }, [renderResult])

  return {
    renderResult,
    renderError,
    isRendering: Boolean(content) && !renderResult && !renderError,
    renderVersion: renderUi.version,
  }
}
