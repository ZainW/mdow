/// <reference lib="webworker" />
// File reads and Markdown/HTML parsing can be expensive; keep that work off the UI thread.
import { loadDocument } from './document-loader'
import type { Block, OutlineEntry } from './markdown'

declare const self: Worker

const CHUNK_SIZE = 512

self.onmessage = (event: MessageEvent<{ id: number; path: string }>) => {
  const { id, path } = event.data
  let result
  try {
    result = loadDocument(path)
  } catch {
    result = { ok: false as const, error: 'read-failed' as const }
  }
  if (
    !result.ok ||
    (result.parsed.blocks.length <= CHUNK_SIZE && result.parsed.outline.length <= CHUNK_SIZE)
  ) {
    self.postMessage({ id, type: 'result', result })
    return
  }

  // Drop the lexer's temporary token tree before the large parsed model is copied back to the UI.
  try {
    Bun.gc(true)
  } catch {}
  self.postMessage({ id, type: 'start', title: result.parsed.title, mtimeMs: result.mtimeMs })
  postChunks(id, 'blocks', result.parsed.blocks)
  postChunks(id, 'outline', result.parsed.outline)
  self.postMessage({ id, type: 'done' })
}

function postChunks(id: number, type: 'blocks' | 'outline', values: (Block | OutlineEntry)[]) {
  const sentinel = values.at(-1)
  if (!sentinel) return
  for (let start = 0; start < values.length; start += CHUNK_SIZE) {
    const end = Math.min(values.length, start + CHUNK_SIZE)
    const chunk = values.slice(start, end)
    self.postMessage({ id, type, values: chunk })
    // The main thread now owns the cloned chunk; replace worker references promptly.
    for (let index = start; index < end; index++) values[index] = sentinel
  }
}
