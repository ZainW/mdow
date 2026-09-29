import { readFileSync, statSync } from 'node:fs'
import { documentKind, type DocumentError } from './documents'
import { parseHtml, parseMarkdown, type ParsedDocument } from './markdown'

export type LoadedDocument =
  | { ok: true; parsed: ParsedDocument; mtimeMs: number }
  | { ok: false; error: DocumentError }

const decoder = new TextDecoder('utf-8', { fatal: true })

export function loadDocument(path: string): LoadedDocument {
  const kind = documentKind(path)
  if (!kind) return { ok: false, error: 'unsupported' }
  let bytes: Buffer
  let mtimeMs: number
  try {
    mtimeMs = statSync(path).mtimeMs
    bytes = readFileSync(path)
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code
    return { ok: false, error: code === 'ENOENT' || code === 'ENOTDIR' ? 'missing' : 'read-failed' }
  }
  let text: string
  try {
    text = decoder.decode(bytes)
  } catch {
    return { ok: false, error: 'not-utf8' }
  }
  if (text.charCodeAt(0) === 0xfeff) text = text.slice(1)
  const parsed = kind === 'html' ? parseHtml(text, path) : parseMarkdown(text, path)
  return { ok: true, parsed, mtimeMs }
}
