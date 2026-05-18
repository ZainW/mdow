import { dialog, BrowserWindow } from 'electron'
import { readFile } from 'fs/promises'
import { watch, type FSWatcher } from 'chokidar'

interface FileWatcherState {
  watcher: FSWatcher
  debounceTimer: ReturnType<typeof setTimeout> | null
  onChange: (event: FileWatchEvent) => void
}

const fileWatchers = new Map<string, FileWatcherState>()

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

export type FileWatchEvent = { type: 'changed'; content: string } | { type: 'deleted' }

export function watchFile(filePath: string, onChange: (event: FileWatchEvent) => void): void {
  const existing = fileWatchers.get(filePath)
  if (existing) {
    existing.onChange = onChange
    return
  }

  const watcher = watch(filePath, { ignoreInitial: true })
  const state: FileWatcherState = { watcher, debounceTimer: null, onChange }

  watcher.on('change', () => {
    if (state.debounceTimer) clearTimeout(state.debounceTimer)
    state.debounceTimer = setTimeout(() => {
      state.debounceTimer = null
      readFile(filePath, 'utf-8')
        .then((content) => state.onChange({ type: 'changed', content }))
        .catch(() => {
          // File might be temporarily unavailable during save
        })
    }, 300)
  })

  watcher.on('unlink', () => {
    state.onChange({ type: 'deleted' })
  })

  fileWatchers.set(filePath, state)
}

export function unwatchFile(filePath: string): void {
  const state = fileWatchers.get(filePath)
  if (state) {
    if (state.debounceTimer) clearTimeout(state.debounceTimer)
    void state.watcher.close()
    fileWatchers.delete(filePath)
  }
}

export function unwatchAllFiles(): void {
  for (const state of fileWatchers.values()) {
    if (state.debounceTimer) clearTimeout(state.debounceTimer)
    void state.watcher.close()
  }
  fileWatchers.clear()
}
