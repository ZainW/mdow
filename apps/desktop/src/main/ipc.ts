import { ipcMain, shell, BrowserWindow, nativeTheme, app } from 'electron'
import { stat } from 'fs/promises'
import { openFileDialog, readFileContent, unwatchFile, setActiveFileWatch } from './file-service'
import { openFolderDialog, scanFolder, watchFolder } from './folder-service'
import { getRecents, addRecent, getAppState, saveAppState, setLastFolder } from './store'
import { checkForUpdates, downloadUpdate, installUpdate, setAutoUpdateScheduling } from './updater'
import { isMac } from './platform'
import { applyWindowChrome } from './window-chrome'
import { validatePath, validateMarkdownPath, isAllowedExternalUrl } from './path-validation'
import { registerAllowedFile, registerAllowedPath, isPathAllowed } from './allowed-paths'
import { rebuildMenu } from './menu'

function setupFolderWatcher(getMainWindow: () => BrowserWindow | null, path: string): void {
  watchFolder(path, (scan) => {
    const win = getMainWindow()
    if (!win || win.isDestroyed()) return
    win.webContents.send('folder:changed', scan)
  })
}

function trackRecentFile(getMainWindow: () => BrowserWindow | null, filePath: string): void {
  addRecent(filePath)
  registerAllowedFile(filePath)
  if (isMac) {
    app.addRecentDocument(filePath)
  }
  rebuildMenu(getMainWindow)
}

export function registerIpcHandlers(getMainWindow: () => BrowserWindow | null): void {
  ipcMain.handle('file:open-dialog', async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const result = await openFileDialog(win)
    if (result) {
      trackRecentFile(() => win, result.path)
      setActiveFileWatch(result.path)
    }
    return result
  })

  ipcMain.handle('file:read', async (event, path: string) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) throw new Error('no-window')
    try {
      const resolved = validateMarkdownPath(path)
      const content = await readFileContent(resolved)
      trackRecentFile(() => win, resolved)
      setActiveFileWatch(resolved)
      return content
    } catch (err: unknown) {
      if (err instanceof Error) {
        if (err.message === 'invalid-path' || err.message === 'path-traversal') {
          throw new Error('invalid-path', { cause: err })
        }
        if (err.message === 'invalid-extension') {
          throw new Error('invalid-extension', { cause: err })
        }
        let code: unknown
        if ('code' in err) {
          code = err.code
        }
        if (code === 'ENOENT') throw new Error('not-found', { cause: err })
        if (code === 'EACCES') throw new Error('permission-denied', { cause: err })
      }
      throw new Error('read-error', { cause: err })
    }
  })

  ipcMain.handle('file:stat', async (_, path: string) => {
    try {
      const resolved = validatePath(path)
      const stats = await stat(resolved)
      return {
        exists: true,
        isFile: stats.isFile(),
        isDirectory: stats.isDirectory(),
      }
    } catch (err: unknown) {
      if (err instanceof Error && err.message === 'invalid-path') {
        throw new Error('invalid-path', { cause: err })
      }
      return { exists: false, isFile: false, isDirectory: false }
    }
  })

  ipcMain.handle('file:unwatch', (_, path: string) => {
    try {
      const resolved = validatePath(path)
      unwatchFile(resolved)
    } catch {
      // Ignore invalid paths on unwatch
    }
  })

  ipcMain.handle('file:set-active-watch', (_, path: string | null) => {
    if (path === null) {
      setActiveFileWatch(null)
      return
    }
    try {
      const resolved = validateMarkdownPath(path)
      registerAllowedFile(resolved)
      setActiveFileWatch(resolved)
    } catch {
      setActiveFileWatch(null)
    }
  })

  ipcMain.handle('folder:open-dialog', async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const result = await openFolderDialog(win)
    if (result) {
      setLastFolder(result.path)
      registerAllowedPath(result.path)
      setupFolderWatcher(() => win, result.path)
    }
    return result
  })

  ipcMain.handle('folder:open-path', async (event, folderPath: string) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) throw new Error('no-window')
    try {
      const resolved = validatePath(folderPath)
      const stats = await stat(resolved)
      if (!stats.isDirectory()) {
        throw new Error('not-a-directory')
      }
      const result = await scanFolder(resolved)
      setLastFolder(resolved)
      registerAllowedPath(resolved)
      setupFolderWatcher(() => win, resolved)
      return { path: resolved, ...result }
    } catch (err: unknown) {
      if (err instanceof Error) {
        if (err.message === 'invalid-path' || err.message === 'path-traversal') {
          throw new Error('invalid-path', { cause: err })
        }
        if (err.message === 'not-a-directory') {
          throw new Error('not-a-directory', { cause: err })
        }
        let code: unknown
        if ('code' in err) {
          code = err.code
        }
        if (code === 'ENOENT') throw new Error('not-found', { cause: err })
        if (code === 'EACCES') throw new Error('permission-denied', { cause: err })
      }
      throw new Error('read-error', { cause: err })
    }
  })

  ipcMain.handle('folder:read-tree', async (event, folderPath: string) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) throw new Error('no-window')
    try {
      const resolved = validatePath(folderPath)
      const result = await scanFolder(resolved)
      setLastFolder(resolved)
      registerAllowedPath(resolved)
      setupFolderWatcher(() => win, resolved)
      return result
    } catch (err: unknown) {
      if (
        err instanceof Error &&
        (err.message === 'invalid-path' || err.message === 'path-traversal')
      ) {
        throw new Error('invalid-path', { cause: err })
      }
      throw err
    }
  })

  ipcMain.handle('store:get-recents', () => getRecents())

  ipcMain.handle('store:get-state', () => getAppState())
  ipcMain.handle('store:save-state', (_, state: Record<string, unknown>) =>
    saveAppState(state as Parameters<typeof saveAppState>[0]),
  )

  ipcMain.handle('theme:set', (_, theme: string) => {
    const valid: Array<typeof nativeTheme.themeSource> = ['light', 'dark', 'system']
    const source = valid.find((v) => v === theme)
    if (!source) return
    nativeTheme.themeSource = source
    saveAppState({ theme: source })
    const win = getMainWindow()
    if (win) applyWindowChrome(win)
  })

  ipcMain.handle('shell:show-in-folder', (_, filePath: string) => {
    try {
      const resolved = validatePath(filePath)
      shell.showItemInFolder(resolved)
    } catch {
      // Ignore invalid paths
    }
  })

  ipcMain.handle('shell:open-external', (_, url: string) => {
    if (typeof url !== 'string' || !isAllowedExternalUrl(url)) {
      return false
    }
    void shell.openExternal(url)
    return true
  })

  ipcMain.handle('url:resolve-local', (_, filePath: string) => {
    try {
      const resolved = validatePath(filePath)
      if (!isPathAllowed(resolved)) {
        throw new Error('forbidden')
      }
      return `mdow-local://local/${encodeURIComponent(resolved)}`
    } catch {
      return ''
    }
  })

  ipcMain.handle('window:set-title', (_, title: string, filePath?: string) => {
    const win = getMainWindow()
    if (!win) return
    win.setTitle(title)
    if (isMac && filePath) {
      try {
        win.setRepresentedFilename(validatePath(filePath))
      } catch {
        // Ignore invalid paths for represented filename
      }
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
