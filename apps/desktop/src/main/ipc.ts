import { ipcMain, shell, BrowserWindow, nativeTheme } from 'electron'
import { openFileDialog, readFileContent, watchFile, unwatchFile } from './file-service'
import { openFolderDialog, scanFolder, watchFolder } from './folder-service'
import { getRecents, addRecent, getAppState, saveAppState, setLastFolder } from './store'
import { checkForUpdates, downloadUpdate, installUpdate, setAutoUpdateScheduling } from './updater'

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
      let code: unknown
      if (err instanceof Error && 'code' in err) {
        code = err.code
      }
      if (code === 'ENOENT') throw new Error('not-found', { cause: err })
      if (code === 'EACCES') throw new Error('permission-denied', { cause: err })
      throw new Error('read-error', { cause: err })
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

  ipcMain.handle('theme:set', (_, theme: string) => {
    const valid: Array<typeof nativeTheme.themeSource> = ['light', 'dark', 'system']
    const source = valid.find((v) => v === theme)
    if (!source) return
    nativeTheme.themeSource = source
    saveAppState({ theme: source })
  })

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

  ipcMain.handle('window:close', () => {
    getMainWindow()?.close()
  })

  ipcMain.handle('updater:check', (_event, opts?: { manual?: boolean }) => checkForUpdates(opts))
  ipcMain.handle('updater:set-scheduling', (_event, enabled: boolean) =>
    setAutoUpdateScheduling(enabled),
  )
  ipcMain.handle('updater:download', () => downloadUpdate())
  ipcMain.handle('updater:install', () => installUpdate())
}
