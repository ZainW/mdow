import { dialog, BrowserWindow } from 'electron'
import type { Dirent } from 'node:fs'
import { readdir } from 'fs/promises'
import { join } from 'path'
import { watch, type FSWatcher } from 'chokidar'
import type { TreeNode, ScanResult } from '../shared/types'

interface FolderWatchState {
  watcher: FSWatcher
  debounceTimer: ReturnType<typeof setTimeout> | null
}

const activeFolderWatchers = new Map<string, FolderWatchState>()

const MD_EXTENSIONS = new Set(['.md', '.markdown', '.mdx'])

const IGNORED_DIRS = new Set([
  'node_modules',
  'dist',
  'out',
  'build',
  'target',
  '.next',
  '.turbo',
  'coverage',
])

const MAX_FILES = 5000
const MAX_DEPTH = 8

function isMdFile(name: string): boolean {
  const dotIndex = name.lastIndexOf('.')
  if (dotIndex === -1) return false
  const ext = name.substring(dotIndex).toLowerCase()
  return MD_EXTENSIONS.has(ext)
}

function isMarkdownPath(path: string): boolean {
  return isMdFile(path.split(/[/\\]/).pop() ?? '')
}

function shouldSkipEntry(name: string): boolean {
  if (name.startsWith('.')) return true
  return IGNORED_DIRS.has(name)
}

interface ScanState {
  fileCount: number
  truncated: boolean
}

async function appendEntry(
  nodes: TreeNode[],
  entry: Dirent,
  folderPath: string,
  depth: number,
  state: ScanState,
): Promise<void> {
  if (state.truncated) return
  if (shouldSkipEntry(entry.name)) return

  const fullPath = join(folderPath, entry.name)

  if (entry.isDirectory()) {
    if (depth >= MAX_DEPTH) return
    const children = await scanInto(fullPath, depth + 1, state)
    if (children.length > 0) {
      nodes.push({ name: entry.name, path: fullPath, isDirectory: true, children })
    }
    return
  }

  if (isMdFile(entry.name)) {
    if (state.fileCount >= MAX_FILES) {
      state.truncated = true
      return
    }
    state.fileCount += 1
    nodes.push({ name: entry.name, path: fullPath, isDirectory: false })
  }
}

async function scanEntriesAt(
  sorted: Dirent[],
  index: number,
  folderPath: string,
  depth: number,
  state: ScanState,
  nodes: TreeNode[],
): Promise<void> {
  if (index >= sorted.length || state.truncated) return
  await appendEntry(nodes, sorted[index], folderPath, depth, state)
  await scanEntriesAt(sorted, index + 1, folderPath, depth, state, nodes)
}

async function scanInto(folderPath: string, depth: number, state: ScanState): Promise<TreeNode[]> {
  if (state.truncated) return []

  const entries = await readdir(folderPath, { withFileTypes: true })
  const nodes: TreeNode[] = []

  const sorted = entries.toSorted((a, b) => {
    if (a.isDirectory() && !b.isDirectory()) return -1
    if (!a.isDirectory() && b.isDirectory()) return 1
    return a.name.localeCompare(b.name)
  })

  await scanEntriesAt(sorted, 0, folderPath, depth, state, nodes)

  return nodes
}

export async function scanFolder(folderPath: string): Promise<ScanResult> {
  const state: ScanState = { fileCount: 0, truncated: false }
  const tree = await scanInto(folderPath, 0, state)
  return { tree, truncated: state.truncated }
}

export function insertFileNode(
  nodes: TreeNode[],
  filePath: string,
  fileName: string,
  parentPath: string,
): boolean {
  const target = parentPath === '' ? nodes : findParentChildren(nodes, parentPath)
  if (!target) return false
  if (target.some((n) => n.path === filePath)) return false

  const insertIndex = target.findIndex((n) => !n.isDirectory && n.name.localeCompare(fileName) > 0)
  const newNode: TreeNode = { name: fileName, path: filePath, isDirectory: false }
  if (insertIndex === -1) {
    target.push(newNode)
  } else {
    target.splice(insertIndex, 0, newNode)
  }
  return true
}

function findParentChildren(nodes: TreeNode[], dirPath: string): TreeNode[] | null {
  for (const node of nodes) {
    if (node.isDirectory && node.path === dirPath) return node.children ?? null
    if (node.isDirectory && node.children) {
      const found = findParentChildren(node.children, dirPath)
      if (found) return found
    }
  }
  return null
}

export function removeFileNode(nodes: TreeNode[], filePath: string): boolean {
  const index = nodes.findIndex((n) => n.path === filePath)
  if (index !== -1) {
    nodes.splice(index, 1)
    return true
  }
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]
    if (node.isDirectory && node.children) {
      if (removeFileNode(node.children, filePath)) {
        if (node.children.length === 0) {
          nodes.splice(i, 1)
        }
        return true
      }
    }
  }
  return false
}

const folderTreeCache = new Map<string, ScanResult>()

export async function openFolderDialog(win: BrowserWindow) {
  const result = await dialog.showOpenDialog(win, {
    properties: ['openDirectory'],
  })

  if (result.canceled || result.filePaths.length === 0) return null

  const folderPath = result.filePaths[0]
  const { tree, truncated } = await scanFolder(folderPath)
  return { path: folderPath, tree, truncated }
}

export function watchFolder(folderPath: string, onChange: (result: ScanResult) => void): void {
  unwatchFolder(folderPath)

  const watcher = watch(folderPath, {
    ignoreInitial: true,
    ignored: (path) => {
      const base = path.split(/[/\\]/).pop() ?? ''
      if (base.startsWith('.')) return true
      return IGNORED_DIRS.has(base)
    },
    depth: MAX_DEPTH,
  })

  const watchState: FolderWatchState = { watcher, debounceTimer: null }
  let pendingChanges: Array<{ type: 'add' | 'unlink' | 'addDir' | 'unlinkDir'; path: string }> = []

  const flushChanges = () => {
    const changes = pendingChanges
    pendingChanges = []

    const cached = folderTreeCache.get(folderPath)
    const hasStructuralChange = changes.some((c) => c.type === 'addDir' || c.type === 'unlinkDir')

    if (!cached || hasStructuralChange) {
      void scanFolder(folderPath)
        .then((result) => {
          folderTreeCache.set(folderPath, result)
          onChange(result)
        })
        .catch(() => {})
      return
    }

    let modified = false
    for (const change of changes) {
      if (change.type === 'add') {
        const sep = change.path.includes('\\') ? '\\' : '/'
        const parts = change.path.split(sep)
        const fileName = parts.pop()!
        const parentPath = parts.join(sep)
        if (isMdFile(fileName)) {
          if (
            insertFileNode(
              cached.tree,
              change.path,
              fileName,
              parentPath === folderPath ? '' : parentPath,
            )
          ) {
            modified = true
          }
        }
      } else if (change.type === 'unlink') {
        if (removeFileNode(cached.tree, change.path)) {
          modified = true
        }
      }
    }

    if (modified) {
      onChange({ tree: cached.tree, truncated: cached.truncated })
    }
  }

  const scheduleFlush = (type: 'add' | 'unlink' | 'addDir' | 'unlinkDir', path: string) => {
    pendingChanges.push({ type, path })
    if (watchState.debounceTimer) clearTimeout(watchState.debounceTimer)
    watchState.debounceTimer = setTimeout(() => {
      watchState.debounceTimer = null
      flushChanges()
    }, 1000)
  }

  watcher.on('add', (path) => {
    if (isMarkdownPath(path)) scheduleFlush('add', path)
  })
  watcher.on('unlink', (path) => {
    if (isMarkdownPath(path)) scheduleFlush('unlink', path)
  })
  watcher.on('addDir', (path) => scheduleFlush('addDir', path))
  watcher.on('unlinkDir', (path) => scheduleFlush('unlinkDir', path))

  void scanFolder(folderPath).then((result) => {
    folderTreeCache.set(folderPath, result)
  })

  activeFolderWatchers.set(folderPath, watchState)
}

export function unwatchFolder(folderPath?: string): void {
  if (folderPath) {
    const state = activeFolderWatchers.get(folderPath)
    if (state) {
      if (state.debounceTimer) clearTimeout(state.debounceTimer)
      void state.watcher.close()
      activeFolderWatchers.delete(folderPath)
      folderTreeCache.delete(folderPath)
    }
  } else {
    for (const state of activeFolderWatchers.values()) {
      if (state.debounceTimer) clearTimeout(state.debounceTimer)
      void state.watcher.close()
    }
    activeFolderWatchers.clear()
    folderTreeCache.clear()
  }
}
