import { extname } from 'node:path'

export const MARKDOWN_EXTENSIONS = new Set(['.md', '.markdown', '.mdx'])
export const HTML_EXTENSIONS = new Set(['.html', '.htm'])

export type DocumentKind = 'markdown' | 'html'

export function documentKind(path: string): DocumentKind | null {
  const ext = extname(path).toLowerCase()
  if (MARKDOWN_EXTENSIONS.has(ext)) return 'markdown'
  if (HTML_EXTENSIONS.has(ext)) return 'html'
  return null
}

export function isSupportedDocument(path: string) {
  return documentKind(path) !== null
}

export type DocumentError = 'unsupported' | 'missing' | 'not-utf8' | 'read-failed'

export const DOCUMENT_ERROR_COPY: Record<DocumentError, { title: string; body: string }> = {
  unsupported: {
    title: "Mdow can't open this file",
    body: 'Mdow opens Markdown (.md, .markdown, .mdx) and HTML (.html, .htm) files.',
  },
  missing: {
    title: 'File not found',
    body: 'This file may have been moved, renamed, or deleted.',
  },
  'not-utf8': {
    title: "This file isn't UTF-8 text",
    body: 'Mdow can only read text files saved as UTF-8.',
  },
  'read-failed': {
    title: "Couldn't read file",
    body: 'Mdow could not read this file. Check that you have permission to access it.',
  },
}
