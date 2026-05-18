import { dialog, BrowserWindow } from 'electron'
import { readdir } from 'fs/promises'
import { join } from 'path'
import { watch, type FSWatcher } from 'chokidar'

export interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

export interface ScanResult {
  tree: TreeNode[]
  truncated: boolean
}

let folderWatcher: FSWatcher | null = null
let folderDebounceTimer: ReturnType<typeof setTimeout> | null = null

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

async function scanInto(folderPath: string, depth: number, state: ScanState): Promise<TreeNode[]> {
  if (state.truncated) return []

  const entries = await readdir(folderPath, { withFileTypes: true })
  const nodes: TreeNode[] = []

  const sorted = entries.toSorted((a, b) => {
    if (a.isDirectory() && !b.isDirectory()) return -1
    if (!a.isDirectory() && b.isDirectory()) return 1
    return a.name.localeCompare(b.name)
  })

  for (const entry of sorted) {
    if (state.truncated) break
    if (shouldSkipEntry(entry.name)) continue

    const fullPath = join(folderPath, entry.name)

    if (entry.isDirectory()) {
      if (depth >= MAX_DEPTH) continue
      // oxlint-disable-next-line no-await-in-loop -- intentional sequential recursive tree walk
      const children = await scanInto(fullPath, depth + 1, state)
      if (children.length > 0) {
        nodes.push({ name: entry.name, path: fullPath, isDirectory: true, children })
      }
    } else if (isMdFile(entry.name)) {
      if (state.fileCount >= MAX_FILES) {
        state.truncated = true
        break
      }
      state.fileCount += 1
      nodes.push({ name: entry.name, path: fullPath, isDirectory: false })
    }
  }

  return nodes
}

export async function scanFolder(folderPath: string): Promise<ScanResult> {
  const state: ScanState = { fileCount: 0, truncated: false }
  const tree = await scanInto(folderPath, 0, state)
  return { tree, truncated: state.truncated }
}

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
  unwatchFolder()

  folderWatcher = watch(folderPath, {
    ignoreInitial: true,
    ignored: (path) => {
      const base = path.split(/[/\\]/).pop() ?? ''
      if (base.startsWith('.')) return true
      return IGNORED_DIRS.has(base)
    },
    depth: MAX_DEPTH,
  })

  const handleChange = () => {
    if (folderDebounceTimer) clearTimeout(folderDebounceTimer)
    folderDebounceTimer = setTimeout(() => {
      folderDebounceTimer = null
      void scanFolder(folderPath)
        .then((result) => {
          onChange(result)
        })
        .catch(() => {
          // Folder might have been deleted
        })
    }, 1000)
  }

  const handleFileChange = (path: string) => {
    if (isMarkdownPath(path)) handleChange()
  }

  folderWatcher.on('add', handleFileChange)
  folderWatcher.on('unlink', handleFileChange)
  folderWatcher.on('addDir', handleChange)
  folderWatcher.on('unlinkDir', handleChange)
}

export function unwatchFolder(): void {
  if (folderDebounceTimer) {
    clearTimeout(folderDebounceTimer)
    folderDebounceTimer = null
  }
  if (folderWatcher) {
    void folderWatcher.close()
    folderWatcher = null
  }
}
