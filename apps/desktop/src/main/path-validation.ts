import { resolve, normalize, extname } from 'path'

const MARKDOWN_EXTENSIONS = new Set(['.md', '.markdown', '.mdx'])

export function isMarkdownPath(path: string): boolean {
  return MARKDOWN_EXTENSIONS.has(extname(path).toLowerCase())
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

export function isAllowedExternalUrl(url: string): boolean {
  try {
    const parsed = new URL(url)
    return parsed.protocol === 'http:' || parsed.protocol === 'https:'
  } catch {
    return false
  }
}
