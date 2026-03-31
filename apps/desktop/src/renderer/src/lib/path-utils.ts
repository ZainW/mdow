export function basename(path: string): string {
  return path.split(/[/\\]/).pop() || path
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
