import { readdirSync, realpathSync, statSync } from 'node:fs'
import { basename, join } from 'node:path'
import { isSupportedDocument } from './documents'

const IGNORED_DIRECTORIES = new Set(['.git', 'node_modules', 'target', 'dist', 'build'])
const MAX_ENTRIES = 20_000

export interface WorkspaceEntry {
  path: string
  name: string
  kind: 'directory' | 'file'
  children: WorkspaceEntry[]
}

export interface WorkspaceRow {
  path: string
  name: string
  kind: 'directory' | 'file'
  depth: number
  expanded: boolean
}

export type WorkspaceError = 'missing' | 'not-directory' | 'read-failed'

export const WORKSPACE_ERROR_COPY: Record<WorkspaceError, { title: string; body: string }> = {
  missing: { title: 'Folder not found', body: 'This folder may have been moved or renamed.' },
  'not-directory': {
    title: 'This is not a folder',
    body: 'Choose a folder to show its Markdown files.',
  },
  'read-failed': {
    title: "Couldn't read folder",
    body: 'Mdow could not read this folder. Check that you have permission to access it.',
  },
}

export type ScanResult = { ok: true; root: WorkspaceEntry } | { ok: false; error: WorkspaceError }

/**
 * Recursive tree of supported documents. Skips dotfiles, VCS and build folders, and folders
 * with nothing to read, so the sidebar only lists what Mdow can open.
 */
export function scanWorkspace(root: string): ScanResult {
  let canonical: string
  try {
    canonical = realpathSync(root)
    if (!statSync(canonical).isDirectory()) return { ok: false, error: 'not-directory' }
  } catch (error) {
    return {
      ok: false,
      error: (error as NodeJS.ErrnoException).code === 'ENOENT' ? 'missing' : 'read-failed',
    }
  }
  const budget = { remaining: MAX_ENTRIES }
  const visited = new Set([canonical])
  let children: WorkspaceEntry[]
  try {
    children = scanDirectory(canonical, visited, budget, true)
  } catch {
    return { ok: false, error: 'read-failed' }
  }
  return {
    ok: true,
    root: { path: canonical, name: basename(canonical) || canonical, kind: 'directory', children },
  }
}

function scanDirectory(
  dir: string,
  visited: Set<string>,
  budget: { remaining: number },
  isRoot = false,
): WorkspaceEntry[] {
  let names: import('node:fs').Dirent[]
  try {
    names = readdirSync(dir, { withFileTypes: true })
  } catch (error) {
    if (isRoot) throw error
    return []
  }
  const dirs: WorkspaceEntry[] = []
  const files: WorkspaceEntry[] = []
  for (const entry of names) {
    if (budget.remaining <= 0) break
    if (entry.name.startsWith('.')) continue
    const path = join(dir, entry.name)
    let isDir = entry.isDirectory()
    let isFile = entry.isFile()
    if (entry.isSymbolicLink()) {
      try {
        const stat = statSync(path)
        isDir = stat.isDirectory()
        isFile = stat.isFile()
      } catch {
        continue
      }
    }
    if (isDir) {
      if (IGNORED_DIRECTORIES.has(entry.name)) continue
      let real: string
      try {
        real = realpathSync(path)
      } catch {
        continue
      }
      if (visited.has(real)) continue
      visited.add(real)
      budget.remaining--
      const children = scanDirectory(path, visited, budget)
      if (children.length > 0) dirs.push({ path, name: entry.name, kind: 'directory', children })
    } else if (isFile && isSupportedDocument(entry.name)) {
      budget.remaining--
      files.push({ path, name: entry.name, kind: 'file', children: [] })
    }
  }
  return [...dirs.toSorted(byName), ...files.toSorted(byName)]
}

function byName(a: WorkspaceEntry, b: WorkspaceEntry) {
  return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
}

export function visibleRows(root: WorkspaceEntry, collapsed: ReadonlySet<string>): WorkspaceRow[] {
  const rows: WorkspaceRow[] = []
  const walk = (entries: WorkspaceEntry[], depth: number) => {
    for (const entry of entries) {
      const expanded = entry.kind === 'directory' && !collapsed.has(entry.path)
      rows.push({ path: entry.path, name: entry.name, kind: entry.kind, depth, expanded })
      if (expanded) walk(entry.children, depth + 1)
    }
  }
  walk(root.children, 0)
  return rows
}

export function workspaceFiles(root: WorkspaceEntry): string[] {
  const files: string[] = []
  const walk = (entries: WorkspaceEntry[]) => {
    for (const entry of entries) {
      if (entry.kind === 'file') files.push(entry.path)
      else walk(entry.children)
    }
  }
  walk(root.children)
  return files
}
