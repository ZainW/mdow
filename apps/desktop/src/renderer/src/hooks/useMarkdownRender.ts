import { startTransition, useEffect, useReducer, useRef } from 'react'
import type { RenderResult } from '../lib/markdown'
import { sliceDocumentHead } from '../lib/markdown-preview'
import { useAppStore } from '../store/app-store'

function getTabRenderFromStore(tabId: string): RenderResult | undefined {
  return useAppStore.getState().renderCache.get(tabId)
}

function setTabRenderInStore(tabId: string, result: RenderResult): void {
  useAppStore.getState().setRenderCache(tabId, result)
}

type RenderDocument = (text: string) => Promise<RenderResult>

function loadRenderDocument(): Promise<RenderDocument> {
  return import('../lib/markdown-client').then((mod) => mod.renderDocument)
}

export interface RenderUi {
  result: RenderResult | null
  version: number
  error: boolean
  /** The result covers only the opening of the document; the full render is still running. */
  partial: boolean
}

export type RenderAction =
  | { type: 'reset' }
  | { type: 'clear-tab' }
  | { type: 'start' }
  | { type: 'ready'; result: RenderResult; version: number }
  | { type: 'preview'; result: RenderResult; version: number }
  | { type: 'error' }

export function renderReducer(state: RenderUi, action: RenderAction): RenderUi {
  switch (action.type) {
    case 'reset':
      return { result: null, version: 0, error: false, partial: false }
    case 'clear-tab':
      return { result: null, version: state.version, error: false, partial: false }
    case 'start':
      return { ...state, error: false }
    case 'ready':
      return { result: action.result, version: action.version, error: false, partial: false }
    case 'preview':
      return { result: action.result, version: action.version, error: false, partial: true }
    case 'error':
      return { result: null, version: state.version, error: true, partial: false }
    default:
      return state
  }
}

export function useMarkdownRender({
  tabId,
  content,
  retryKey,
  allowPreview = false,
}: {
  tabId: string
  content: string
  retryKey: number
  /** Show the opening of a very large document while the rest parses. */
  allowPreview?: boolean
}): {
  renderResult: RenderResult | null
  renderError: boolean
  isRendering: boolean
  isPartial: boolean
  renderVersion: number
} {
  const [renderUi, dispatchRender] = useReducer(renderReducer, {
    result: null,
    version: 0,
    error: false,
    partial: false,
  })
  // Read when a render starts, not a render dependency: scrolling a preview must not restart it.
  const allowPreviewRef = useRef(allowPreview)
  useEffect(() => {
    allowPreviewRef.current = allowPreview
  }, [allowPreview])
  const renderVersionRef = useRef(0)
  const lastRenderedTabIdRef = useRef(tabId)
  const renderResult = renderUi.result
  // Whether this tab already shows a full render (read when a render starts).
  const renderedRef = useRef(false)
  renderedRef.current = renderResult !== null && !renderUi.partial
  const renderError = renderUi.error

  useEffect(() => {
    if (!content) {
      dispatchRender({ type: 'reset' })
      return undefined
    }
    // What's on screen belongs to the previous document when the pane switched tabs.
    const showingThisDocument = lastRenderedTabIdRef.current === tabId && renderedRef.current
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
    let done = false
    let previewShown = false
    dispatchRender({ type: 'start' })

    // Only fresh opens get a preview: restoring a saved position needs the whole document, and a
    // reload or retry of a document already on screen keeps showing it until the new render lands.
    const head = allowPreviewRef.current && !showingThisDocument ? sliceDocumentHead(content) : null

    void loadRenderDocument()
      .then((renderDocument) => {
        if (head) {
          renderDocument(head)
            .then((res) => {
              if (cancelled || done) return
              previewShown = true
              renderVersionRef.current += 1
              dispatchRender({ type: 'preview', result: res, version: renderVersionRef.current })
            })
            .catch(() => {
              // The full render reports errors.
            })
        }
        return renderDocument(content)
      })
      .then((res) => {
        done = true
        if (cancelled) return
        setTabRenderInStore(tabId, res)
        renderVersionRef.current += 1
        const version = renderVersionRef.current
        const ready = () => dispatchRender({ type: 'ready', result: res, version })
        // Someone may already be reading the preview; build the full tree in interruptible
        // slices so scrolling and input stay responsive meanwhile.
        if (previewShown) startTransition(ready)
        else ready()
      })
      .catch(() => {
        done = true
        if (cancelled) return
        dispatchRender({ type: 'error' })
      })
    return () => {
      cancelled = true
    }
  }, [tabId, content, retryKey])

  useEffect(() => {
    // A preview's outline would only list the opening headings; wait for the full document.
    if (renderUi.partial) return
    const headings = renderResult?.headings ?? []
    useAppStore.setState({
      docHeadings: headings,
      activeHeadingId: headings[0]?.id ?? null,
    })
  }, [renderResult, renderUi.partial])

  return {
    renderResult,
    renderError,
    isRendering: Boolean(content) && (!renderResult || renderUi.partial) && !renderError,
    isPartial: renderUi.partial,
    renderVersion: renderUi.version,
  }
}
