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

let folderWatcher: FSWatcher | null = null

const MD_EXTENSIONS = new Set(['.md', '.markdown', '.mdx'])

function isMdFile(name: string): boolean {
  const dotIndex = name.lastIndexOf('.')
  if (dotIndex === -1) return false
  const ext = name.substring(dotIndex).toLowerCase()
  return MD_EXTENSIONS.has(ext)
}

export async function scanFolder(folderPath: string): Promise<TreeNode[]> {
  const entries = await readdir(folderPath, { withFileTypes: true })
  const nodes: TreeNode[] = []

  const sorted = entries.sort((a, b) => {
    if (a.isDirectory() && !b.isDirectory()) return -1
    if (!a.isDirectory() && b.isDirectory()) return 1
    return a.name.localeCompare(b.name)
  })

  for (const entry of sorted) {
    if (entry.name.startsWith('.')) continue

    const fullPath = join(folderPath, entry.name)

    if (entry.isDirectory()) {
      const children = await scanFolder(fullPath)
      if (children.length > 0) {
        nodes.push({ name: entry.name, path: fullPath, isDirectory: true, children })
      }
    } else if (isMdFile(entry.name)) {
      nodes.push({ name: entry.name, path: fullPath, isDirectory: false })
    }
  }

  return nodes
}

export async function openFolderDialog(win: BrowserWindow) {
  const result = await dialog.showOpenDialog(win, {
    properties: ['openDirectory'],
  })

  if (result.canceled || result.filePaths.length === 0) return null

  const folderPath = result.filePaths[0]
  const tree = await scanFolder(folderPath)
  return { path: folderPath, tree }
}

export function watchFolder(
  folderPath: string,
  onChange: (tree: TreeNode[]) => void
): void {
  unwatchFolder()

  folderWatcher = watch(folderPath, {
    ignoreInitial: true,
    ignored: /(^|[/\\])\./,
    depth: 10,
  })

  let debounceTimer: ReturnType<typeof setTimeout> | null = null

  const handleChange = () => {
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(async () => {
      try {
        const tree = await scanFolder(folderPath)
        onChange(tree)
      } catch {
        // Folder might have been deleted
      }
    }, 1000)
  }

  folderWatcher.on('add', handleChange)
  folderWatcher.on('unlink', handleChange)
  folderWatcher.on('addDir', handleChange)
  folderWatcher.on('unlinkDir', handleChange)
}

export function unwatchFolder(): void {
  if (folderWatcher) {
    folderWatcher.close()
    folderWatcher = null
  }
}
