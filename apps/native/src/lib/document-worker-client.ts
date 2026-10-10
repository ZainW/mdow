import { loadDocument, type LoadedDocument } from './document-loader'
import type { Block, OutlineEntry, ParsedDocument } from './markdown'

type WorkerResponse =
  | { id: number; type: 'result'; result: LoadedDocument }
  | { id: number; type: 'start'; title: string | null; mtimeMs: number }
  | { id: number; type: 'blocks'; values: Block[] }
  | { id: number; type: 'outline'; values: OutlineEntry[] }
  | { id: number; type: 'done' }

interface PendingRequest {
  path: string
  resolve: (result: LoadedDocument) => void
  partial?: { title: string | null; mtimeMs: number; blocks: Block[]; outline: OutlineEntry[] }
}

let worker: Worker | null | undefined
let nextId = 0
const pending = new Map<number, PendingRequest>()

function documentWorker(): Worker | null {
  if (worker !== undefined) return worker
  try {
    // Compiled binaries embed workers as separate entrypoints rooted at the build directory.
    const compiled = import.meta.url.startsWith('file:///$bunfs/')
    worker = new Worker(
      compiled
        ? './src/lib/document-worker.ts'
        : new URL('./document-worker.ts', import.meta.url).href,
    )
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const request = pending.get(event.data.id)
      if (!request) return
      if (event.data.type === 'result') {
        finish(event.data.id, event.data.result)
      } else if (event.data.type === 'start') {
        request.partial = {
          title: event.data.title,
          mtimeMs: event.data.mtimeMs,
          blocks: [],
          outline: [],
        }
      } else if (event.data.type === 'blocks') {
        request.partial?.blocks.push(...event.data.values)
      } else if (event.data.type === 'outline') {
        request.partial?.outline.push(...event.data.values)
      } else {
        const partial = request.partial
        if (!partial) {
          finish(event.data.id, { ok: false, error: 'read-failed' })
          return
        }
        const parsed: ParsedDocument = {
          title: partial.title,
          blocks: partial.blocks,
          outline: partial.outline,
          slugs: new Map(partial.outline.map((entry) => [entry.slug, entry.blockIndex])),
        }
        finish(event.data.id, { ok: true, parsed, mtimeMs: partial.mtimeMs })
      }
    }
    worker.onerror = () => {
      const failed = worker
      worker = null
      failed?.terminate()
      const requests = [...pending.values()]
      pending.clear()
      for (const request of requests) {
        setTimeout(() => request.resolve(loadSafely(request.path)), 0)
      }
    }
    worker.unref()
  } catch {
    worker = null
  }
  return worker
}

function finish(id: number, result: LoadedDocument) {
  const request = pending.get(id)
  if (!request) return
  pending.delete(id)
  request.resolve(result)
}

/** Load a document in a worker, falling back to a deferred main-thread load if workers are absent. */
export function loadDocumentAsync(path: string): Promise<LoadedDocument> {
  return new Promise((resolve) => {
    const target = documentWorker()
    if (!target) {
      setTimeout(() => resolve(loadSafely(path)), 0)
      return
    }

    const id = nextId++
    pending.set(id, { path, resolve })
    try {
      target.postMessage({ id, path })
    } catch {
      pending.delete(id)
      setTimeout(() => resolve(loadSafely(path)), 0)
    }
  })
}

function loadSafely(path: string): LoadedDocument {
  try {
    return loadDocument(path)
  } catch {
    return { ok: false, error: 'read-failed' }
  }
}
