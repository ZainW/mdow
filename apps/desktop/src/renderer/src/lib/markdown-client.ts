import { memoizeAsync } from './cache-storage'
import type { RenderResult } from './markdown'

// Parsing a multi-megabyte document takes hundreds of milliseconds. Large documents parse in a
// worker so the window stays responsive meanwhile; small ones stay on the main thread, where
// they finish sooner than a worker round trip would.
const WORKER_MIN_CHARS = 256 * 1024

type Pending = { resolve: (result: RenderResult) => void; reject: (error: Error) => void }

let worker: Worker | null | undefined
let nextId = 0
const pending = new Map<number, Pending>()

function failAll(error: Error): void {
  for (const request of pending.values()) request.reject(error)
  pending.clear()
}

function getWorker(): Worker | null {
  if (worker !== undefined) return worker
  if (typeof Worker === 'undefined') return (worker = null)
  try {
    const instance = new Worker(new URL('./markdown.worker.ts', import.meta.url), {
      type: 'module',
    })
    instance.onmessage = (
      event: MessageEvent<{ id: number; result?: RenderResult; error?: string }>,
    ) => {
      const request = pending.get(event.data.id)
      if (!request) return
      pending.delete(event.data.id)
      if (event.data.result) request.resolve(event.data.result)
      else request.reject(new Error(event.data.error ?? 'render-failed'))
    }
    instance.onerror = () => {
      // A worker that cannot start (or crashes) is retired; later renders use the main thread.
      worker = null
      instance.terminate()
      failAll(new Error('markdown-worker-failed'))
    }
    worker = instance
  } catch {
    worker = null
  }
  return worker
}

function renderInWorker(instance: Worker, text: string): Promise<RenderResult> {
  return new Promise((resolve, reject) => {
    const id = nextId++
    pending.set(id, { resolve, reject })
    instance.postMessage({ id, text })
  })
}

async function renderOnMainThread(text: string): Promise<RenderResult> {
  const { renderMarkdown } = await import('./markdown')
  return renderMarkdown(text)
}

async function renderDocumentUncached(text: string): Promise<RenderResult> {
  const instance = text.length >= WORKER_MIN_CHARS ? getWorker() : null
  if (instance) {
    try {
      return await renderInWorker(instance, text)
    } catch {
      // Fall through to the main thread.
    }
  }
  return renderOnMainThread(text)
}

/** Renders a document, off the main thread when it is large. Cached by document text. */
export const renderDocument = memoizeAsync(renderDocumentUncached, {
  maxEntries: 24,
  getKey: (text) => text,
})
