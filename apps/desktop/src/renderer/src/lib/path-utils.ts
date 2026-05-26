export function basename(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

// Pick the path separator. A path that only contains backslashes is Windows;
// anything else (or a mixed/empty path) is treated as POSIX. Centralized so
// utilities and components agree on the rule.
export function detectSep(path: string): '/' | '\\' {
  return path.includes('\\') && !path.includes('/') ? '\\' : '/'
}

export function isMarkdownPath(path: string): boolean {
  const lower = path.toLowerCase()
  return lower.endsWith('.md') || lower.endsWith('.markdown') || lower.endsWith('.mdx')
}

export function shortenPath(path: string, maxLen = 40): string {
  if (path.length <= maxLen) return path
  const parts = path.split(/[/\\]/)
  if (parts.length <= 2) return path
  return `.../${parts.slice(-2).join('/')}`
}

export function parentDir(path: string, segments = 2): string {
  const parts = path.split(/[/\\]/).filter(Boolean)
  if (parts.length <= 1) return ''
  const dirs = parts.slice(0, -1)
  return dirs.slice(-segments).join('/')
}

export function resolveRelativePath(fromPath: string, relativePath: string): string {
  const sep = detectSep(fromPath)
  if (/^[/\\]|^[A-Za-z]:[/\\]/.test(relativePath)) {
    return relativePath.replace(/[/\\]/g, sep)
  }

  const baseParts = fromPath.split(/[/\\]/).filter(Boolean)
  if (baseParts.length > 0) baseParts.pop()

  for (const part of relativePath.split(/[/\\]/).filter(Boolean)) {
    if (part === '.') continue
    if (part === '..') baseParts.pop()
    else baseParts.push(part)
  }

  const isWindowsAbsolute = /^[A-Za-z]:/.test(fromPath)
  if (isWindowsAbsolute) return `${baseParts[0]}${sep}${baseParts.slice(1).join(sep)}`
  if (fromPath.startsWith('/')) return `${sep}${baseParts.join(sep)}`
  return baseParts.join(sep)
}

// Drop middle segments from a long path so the user still sees the leading
// folder + filename. Used in the error view's path label.
export function truncatePathMiddle(path: string, maxLen = 56): string {
  if (path.length <= maxLen) return path
  const sep = detectSep(path)
  const parts = path.split(/[/\\]/).filter(Boolean)
  const last = parts[parts.length - 1] ?? ''
  const head = `…${sep}`
  // If the basename alone fits with a short prefix, drop everything else
  if (parts.length > 2) {
    const first = parts[0]
    const collapsed = `${first}${sep}…${sep}${last}`
    if (collapsed.length <= maxLen) return collapsed
  }
  // Basename too long even with no leading folder — hard-truncate the basename
  return head + last.slice(Math.max(0, last.length - (maxLen - head.length)))
}
