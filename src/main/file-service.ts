import { dialog, BrowserWindow } from 'electron'
import { readFile } from 'fs/promises'
import { watch, type FSWatcher } from 'chokidar'

let fileWatcher: FSWatcher | null = null

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

export function watchFile(filePath: string, onChange: (content: string) => void): void {
  unwatchFile()
  fileWatcher = watch(filePath, { ignoreInitial: true })
  let debounceTimer: ReturnType<typeof setTimeout> | null = null

  fileWatcher.on('change', () => {
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => {
      readFile(filePath, 'utf-8')
        .then((content) => onChange(content))
        .catch(() => {
          // File might be temporarily unavailable during save
        })
    }, 300)
  })
}

export function unwatchFile(): void {
  if (fileWatcher) {
    void fileWatcher.close()
    fileWatcher = null
  }
}
