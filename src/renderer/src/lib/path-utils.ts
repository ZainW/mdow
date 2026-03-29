export function basename(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

export function shortenPath(path: string, maxLen = 40): string {
  if (path.length <= maxLen) return path
  const parts = path.split(/[/\\]/)
  if (parts.length <= 2) return path
  return `.../${parts.slice(-2).join('/')}`
}
