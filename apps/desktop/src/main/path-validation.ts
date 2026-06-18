import { resolve, normalize, extname } from 'path'
import { DOCUMENT_EXTENSIONS, MD_EXTENSIONS } from '../shared/types'

const MARKDOWN_EXTENSIONS = MD_EXTENSIONS

export function isMarkdownPath(path: string): boolean {
  return MARKDOWN_EXTENSIONS.has(extname(path).toLowerCase())
}

export function isDocumentPath(path: string): boolean {
  return DOCUMENT_EXTENSIONS.has(extname(path).toLowerCase())
}

export function validatePath(path: string): string {
  if (!path || typeof path !== 'string') {
    throw new Error('invalid-path')
  }

  if (path.includes('..')) {
    throw new Error('path-traversal')
  }

  return resolve(normalize(path))
}

export function validateMarkdownPath(path: string): string {
  const resolved = validatePath(path)
  if (!isMarkdownPath(resolved)) {
    throw new Error('invalid-extension')
  }
  return resolved
}

export function validateDocumentPath(path: string): string {
  const resolved = validatePath(path)
  if (!isDocumentPath(resolved)) {
    throw new Error('invalid-extension')
  }
  return resolved
}

export function isAllowedExternalUrl(url: string): boolean {
  try {
    const parsed = new URL(url)
    return parsed.protocol === 'http:' || parsed.protocol === 'https:'
  } catch {
    return false
  }
}
