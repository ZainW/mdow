import { resolve, sep } from 'path'

const allowedPaths = new Set<string>()

export function registerAllowedPath(path: string): void {
  allowedPaths.add(resolve(path))
}

export function registerAllowedFile(filePath: string): void {
  const resolved = resolve(filePath)
  allowedPaths.add(resolved)
  allowedPaths.add(resolve(resolved, '..'))
}

export function isPathAllowed(filePath: string): boolean {
  const resolved = resolve(filePath)
  for (const base of allowedPaths) {
    if (resolved === base || resolved.startsWith(base + sep)) {
      return true
    }
  }
  return false
}

export function clearAllowedPaths(): void {
  allowedPaths.clear()
}
