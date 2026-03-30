import { ipcMain, shell, BrowserWindow } from 'electron'
import { openFileDialog, readFileContent, watchFile, unwatchFile } from './file-service'
import { openFolderDialog, scanFolder, watchFolder } from './folder-service'
import { getRecents, addRecent, getAppState, saveAppState, setLastFolder } from './store'

function setupFileWatcher(win: BrowserWindow, path: string): void {
  watchFile(path, (event) => {
    if (event.type === 'changed') {
      win.webContents.send('file:changed', { path, content: event.content })
    } else if (event.type === 'deleted') {
      win.webContents.send('file:deleted', path)
    }
  })
}

export function registerIpcHandlers(getMainWindow: () => BrowserWindow | null): void {
  ipcMain.handle('file:open-dialog', async () => {
    const win = getMainWindow()
    if (!win) return null
    const result = await openFileDialog(win)
    if (result) {
      addRecent(result.path)
      setupFileWatcher(win, result.path)
    }
    return result
  })

  ipcMain.handle('file:read', async (_, path: string) => {
    try {
      const content = await readFileContent(path)
      const win = getMainWindow()
      if (win) {
        addRecent(path)
        setupFileWatcher(win, path)
      }
      return content
    } catch (err: unknown) {
      const code = (err as NodeJS.ErrnoException).code
      if (code === 'ENOENT') throw new Error('not-found')
      if (code === 'EACCES') throw new Error('permission-denied')
      throw new Error('read-error')
    }
  })

  ipcMain.handle('file:unwatch', (_, path: string) => {
    unwatchFile(path)
  })

  ipcMain.handle('folder:open-dialog', async () => {
    const win = getMainWindow()
    if (!win) return null
    const result = await openFolderDialog(win)
    if (result) {
      setLastFolder(result.path)
      watchFolder(result.path, (tree) => {
        win.webContents.send('folder:changed', tree)
      })
    }
    return result
  })

  ipcMain.handle('folder:read-tree', async (_, folderPath: string) => {
    const win = getMainWindow()
    const tree = await scanFolder(folderPath)
    if (win) {
      setLastFolder(folderPath)
      watchFolder(folderPath, (newTree) => {
        win.webContents.send('folder:changed', newTree)
      })
    }
    return tree
  })

  ipcMain.handle('store:get-recents', () => getRecents())
  ipcMain.handle('store:get-state', () => getAppState())
  ipcMain.handle('store:save-state', (_, state: Record<string, unknown>) =>
    saveAppState(state as Parameters<typeof saveAppState>[0]),
  )
  ipcMain.handle('store:add-recent', (_, filePath: string) => addRecent(filePath))

  ipcMain.handle('shell:show-in-folder', (_, filePath: string) => {
    shell.showItemInFolder(filePath)
  })

  ipcMain.handle('window:set-title', (_, title: string, filePath?: string) => {
    const win = getMainWindow()
    if (!win) return
    win.setTitle(title)
    if (process.platform === 'darwin' && filePath) {
      win.setRepresentedFilename(filePath)
    }
  })
}
