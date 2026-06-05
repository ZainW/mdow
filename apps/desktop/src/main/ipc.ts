import { ipcMain, shell, BrowserWindow, nativeTheme, app } from 'electron'
import { stat } from 'fs/promises'
import { openFileDialog, readFileContent, unwatchFile, setActiveFileWatch } from './file-service'
import { openFolderDialog, scanFolder, watchFolder } from './folder-service'
import { getRecents, addRecent, getAppState, saveAppState, setLastFolder } from './store'
import { isMac } from './platform'
import { applyWindowChrome } from './window-chrome'
import { validatePath, validateMarkdownPath, isAllowedExternalUrl } from './path-validation'
import { registerAllowedFile, registerAllowedPath, isPathAllowed } from './allowed-paths'
import { rebuildMenu } from './menu'
import { createDefaultCompanionService, type CompanionService } from './companion/service'
import {
  IPC,
  type CompanionProviderId,
  type CompanionSendRequest,
  type CompanionSettings,
} from '../shared/types'

type UpdaterModule = typeof import('./updater')

let updaterModulePromise: Promise<UpdaterModule> | null = null

function loadUpdater(): Promise<UpdaterModule> {
  updaterModulePromise ??= import('./updater')
  return updaterModulePromise
}

async function loadInitializedUpdater(
  getMainWindow: () => BrowserWindow | null,
): Promise<UpdaterModule> {
  const updater = await loadUpdater()
  updater.initAutoUpdater(getMainWindow)
  return updater
}

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

function getCompanionSettings(): CompanionSettings {
  const state = getAppState()
  return {
    provider: state.companionProvider,
    customCommand: state.companionCustomCommand,
  }
}

export function registerIpcHandlers(getMainWindow: () => BrowserWindow | null): () => void {
  const companionServices = new Map<number, CompanionService>()

  function companionServiceForSender(sender: Electron.WebContents): CompanionService {
    const existing = companionServices.get(sender.id)
    if (existing) {
      return existing
    }

    const service = createDefaultCompanionService({
      getSettings: getCompanionSettings,
      emitUpdate: (update) => {
        if (sender.isDestroyed()) return
        sender.send(IPC.COMPANION_UPDATE, update)
      },
    })
    companionServices.set(sender.id, service)
    sender.once('destroyed', () => {
      service.shutdown()
      companionServices.delete(sender.id)
    })
    return service
  }

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

  ipcMain.handle('updater:check', async (_event, opts?: { manual?: boolean }) => {
    const { checkForUpdates } = await loadInitializedUpdater(getMainWindow)
    checkForUpdates(opts)
  })
  ipcMain.handle('updater:set-scheduling', async (_event, enabled: boolean) => {
    const { setAutoUpdateScheduling } = await loadInitializedUpdater(getMainWindow)
    setAutoUpdateScheduling(enabled)
  })
  ipcMain.handle('updater:download', async () => {
    const { downloadUpdate } = await loadInitializedUpdater(getMainWindow)
    downloadUpdate()
  })
  ipcMain.handle('updater:install', async () => {
    const { installUpdate } = await loadInitializedUpdater(getMainWindow)
    installUpdate()
  })

  ipcMain.handle('companion:detect-providers', (event) =>
    companionServiceForSender(event.sender).detectProviders(),
  )
  ipcMain.handle('companion:get-settings', () => getCompanionSettings())
  ipcMain.handle('companion:save-settings', (_, settings: unknown) => {
    const companionSettings = validateCompanionSettings(settings)
    saveAppState({
      companionProvider: companionSettings.provider,
      companionCustomCommand: companionSettings.customCommand,
    })
  })
  ipcMain.handle('companion:send', (event, request: unknown) => {
    const companionRequest = validateCompanionSendRequest(request)
    return companionServiceForSender(event.sender).send(companionRequest)
  })
  ipcMain.handle('companion:cancel', (event) => {
    companionServices.get(event.sender.id)?.cancel()
  })
  ipcMain.handle('companion:shutdown', (event) => {
    companionServices.get(event.sender.id)?.shutdown()
    companionServices.delete(event.sender.id)
  })

  return () => {
    for (const service of companionServices.values()) {
      service.shutdown()
    }
    companionServices.clear()
  }
}

function validateCompanionSettings(value: unknown): CompanionSettings {
  if (!isRecord(value)) {
    throw new Error('invalid-companion-settings')
  }
  if (!isCompanionProviderId(value.provider) || typeof value.customCommand !== 'string') {
    throw new Error('invalid-companion-settings')
  }
  return { provider: value.provider, customCommand: value.customCommand }
}

function validateCompanionSendRequest(value: unknown): CompanionSendRequest {
  if (!isRecord(value)) {
    throw new Error('invalid-companion-request')
  }
  if (
    typeof value.messageId !== 'string' ||
    value.messageId.trim() === '' ||
    typeof value.text !== 'string' ||
    value.text.trim() === '' ||
    !isCompanionProviderId(value.provider) ||
    !isNullableString(value.activePath) ||
    !isNullableString(value.openFolderPath)
  ) {
    throw new Error('invalid-companion-request')
  }
  return {
    messageId: value.messageId,
    text: value.text,
    provider: value.provider,
    activePath: value.activePath,
    openFolderPath: value.openFolderPath,
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isCompanionProviderId(value: unknown): value is CompanionProviderId {
  if (typeof value !== 'string') {
    return false
  }
  switch (value) {
    case 'auto':
    case 'opencode':
    case 'codex':
    case 'custom':
      return true
    default:
      return false
  }
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === 'string'
}
