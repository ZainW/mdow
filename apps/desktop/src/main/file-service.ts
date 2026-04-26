import { dialog, BrowserWindow } from 'electron'
import { readFile, writeFile as fsWriteFile, readdir } from 'fs/promises'
import { watch, type FSWatcher } from 'chokidar'
import { join } from 'path'

const fileWatchers = new Map<string, FSWatcher>()

export async function openFileDialog(win: BrowserWindow) {
  const result = await dialog.showOpenDialog(win, {
    properties: ['openFile'],
    filters: [{ name: 'Markdown', extensions: ['md', 'markdown', 'mdx'] }],
  })

  if (result.canceled || result.filePaths.length === 0) return null

  const path = result.filePaths[0]
  const content = await readFile(path, 'utf-8')
  return { path, content }
}

export async function readFileContent(path: string): Promise<string> {
  return readFile(path, 'utf-8')
}

export async function writeFile(path: string, content: string): Promise<void> {
  await fsWriteFile(path, content, 'utf8')
}

export type FileWatchEvent = { type: 'changed'; content: string } | { type: 'deleted' }

export function watchFile(filePath: string, onChange: (event: FileWatchEvent) => void): void {
  if (fileWatchers.has(filePath)) return
  const watcher = watch(filePath, { ignoreInitial: true })
  let debounceTimer: ReturnType<typeof setTimeout> | null = null

  watcher.on('change', () => {
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => {
      readFile(filePath, 'utf-8')
        .then((content) => onChange({ type: 'changed', content }))
        .catch(() => {
          // File might be temporarily unavailable during save
        })
    }, 300)
  })

  watcher.on('unlink', () => {
    onChange({ type: 'deleted' })
  })

  fileWatchers.set(filePath, watcher)
}

export function unwatchFile(filePath: string): void {
  const watcher = fileWatchers.get(filePath)
  if (watcher) {
    void watcher.close()
    fileWatchers.delete(filePath)
  }
}

export function unwatchAllFiles(): void {
  for (const watcher of fileWatchers.values()) {
    void watcher.close()
  }
  fileWatchers.clear()
}

export function generateUniqueName(existing: string[], base: string): string {
  if (!existing.includes(base)) return base
  const dot = base.lastIndexOf('.')
  const stem = dot > 0 ? base.slice(0, dot) : base
  const ext = dot > 0 ? base.slice(dot) : ''
  let n = 2
  while (existing.includes(`${stem}-${n}${ext}`)) n++
  return `${stem}-${n}${ext}`
}

export async function createFileInFolder(folderPath: string): Promise<{ path: string }> {
  const entries = await readdir(folderPath)
  const name = generateUniqueName(entries, 'Untitled.md')
  const path = join(folderPath, name)
  await fsWriteFile(path, '', 'utf8')
  return { path }
}
