import { app, BrowserWindow, nativeTheme, protocol, net, shell } from 'electron'

protocol.registerSchemesAsPrivileged([
  {
    scheme: 'mdow-local',
    privileges: {
      standard: true,
      secure: true,
      supportFetchAPI: true,
    },
  },
])
import { join } from 'path'
import { pathToFileURL } from 'url'
import { is } from '@electron-toolkit/utils'
import { registerIpcHandlers } from './ipc'
import { createMenu } from './menu'
import { addRecent, getWindowBounds, saveWindowBounds, getAppState } from './store'
import { unwatchFolder } from './folder-service'
import { readFileContent, unwatchAllFiles, setActiveFileWatch } from './file-service'
import { isMac, isLinux } from './platform'
import { applyWindowChrome, getWindowChromeOptions } from './window-chrome'
import { isDocumentPath, validateDocumentPath, validatePath } from './path-validation'
import { registerAllowedFile, isPathAllowed, clearAllowedPaths } from './allowed-paths'

const windows = new Set<BrowserWindow>()
const windowPaths = new Map<BrowserWindow, string>()
const AUTO_UPDATER_INIT_DELAY_MS = 10_000
let cleanupIpcHandlers: (() => void) | null = null

function getMainWindow(): BrowserWindow | null {
  const focused = BrowserWindow.getFocusedWindow()
  if (focused && windows.has(focused)) {
    return focused
  }
  if (windows.size > 0) {
    return Array.from(windows)[0]
  }
  return null
}

function registerLocalProtocol(): void {
  protocol.handle('mdow-local', (request) => {
    const url = new URL(request.url)
    const encoded = url.pathname.replace(/^\//, '')
    const filePath = decodeURIComponent(encoded)

    if (!isPathAllowed(filePath)) {
      return new Response('Forbidden', { status: 403 })
    }

    return net.fetch(pathToFileURL(filePath).toString())
  })
}

function setupWebContentsSecurity(win: BrowserWindow): void {
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('http://') || url.startsWith('https://')) {
      void shell.openExternal(url)
    }
    return { action: 'deny' }
  })

  win.webContents.on('will-navigate', (event, url) => {
    const devUrl = process.env['ELECTRON_RENDERER_URL']
    if (is.dev && devUrl && url.startsWith(devUrl)) {
      return
    }
    if (url.startsWith('file://')) {
      return
    }
    event.preventDefault()
    if (url.startsWith('http://') || url.startsWith('https://')) {
      void shell.openExternal(url)
    }
  })
}

function openFile(filePath: string): void {
  void (async () => {
    try {
      const resolved = validateDocumentPath(filePath)
      const content = await readFileContent(resolved)
      addRecent(resolved)
      registerAllowedFile(resolved)
      if (isMac) {
        app.addRecentDocument(resolved)
      }
      setActiveFileWatch(resolved)
      const allWindows = BrowserWindow.getAllWindows()
      for (const win of allWindows) {
        if (!win.isDestroyed()) {
          win.webContents.send('file:opened', { path: resolved, content })
        }
      }
    } catch {
      // Invalid file path
    }
  })()
}

function openFileFromArgv(argv: string[], _win: BrowserWindow): void {
  const filePath = argv.find(isDocumentPath)
  if (filePath) {
    openFile(filePath)
  }
}

function createWindow(targetPath?: string): void {
  const savedBounds = getWindowBounds()

  const win = new BrowserWindow({
    width: savedBounds?.width ?? 1000,
    height: savedBounds?.height ?? 700,
    x: savedBounds?.x,
    y: savedBounds?.y,
    minWidth: 600,
    minHeight: 400,
    show: false,
    ...getWindowChromeOptions(),
    ...(isLinux
      ? {
          icon: is.dev
            ? join(__dirname, '../../resources/icon.png')
            : join(process.resourcesPath, 'icon.png'),
        }
      : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.mjs'),
      sandbox: false,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  windows.add(win)
  if (targetPath) {
    windowPaths.set(win, validatePath(targetPath))
  }

  if (savedBounds?.isMaximized && windows.size === 1) {
    win.maximize()
  }

  setupWebContentsSecurity(win)

  win.on('ready-to-show', () => {
    win.show()
  })

  const saveBounds = () => {
    if (win && !win.isDestroyed()) {
      saveWindowBounds(win.getBounds(), win.isMaximized())
    }
  }
  win.on('resized', saveBounds)
  win.on('moved', saveBounds)
  win.on('maximize', saveBounds)
  win.on('unmaximize', saveBounds)

  const handleNativeThemeUpdate = () => {
    if (win && !win.isDestroyed()) {
      applyWindowChrome(win)
      win.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)
    }
  }
  nativeTheme.on('updated', handleNativeThemeUpdate)

  win.webContents.on('did-finish-load', () => {
    win.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)
    if (!targetPath && windows.size === 1) {
      openFileFromArgv(process.argv, win)
    }
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    const url = process.env['ELECTRON_RENDERER_URL']
    const queryStr = targetPath ? `?openPath=${encodeURIComponent(targetPath)}` : ''
    void win.loadURL(url + queryStr)
  } else {
    const htmlPath = join(__dirname, '../renderer/index.html')
    void win.loadFile(htmlPath, { query: targetPath ? { openPath: targetPath } : undefined })
  }

  win.on('closed', () => {
    nativeTheme.off('updated', handleNativeThemeUpdate)
    const folder = windowPaths.get(win)
    if (folder) unwatchFolder(folder)
    windowPaths.delete(win)
    windows.delete(win)

    if (windows.size === 0) {
      cleanupIpcHandlers?.()
      unwatchAllFiles()
      unwatchFolder()
      clearAllowedPaths()
      if (!isMac) app.quit()
    }
  })
}

function scheduleAutoUpdaterInit(): void {
  setTimeout(() => {
    void import('./updater').then(({ initAutoUpdater }) => initAutoUpdater(getMainWindow))
  }, AUTO_UPDATER_INIT_DELAY_MS)
}

const gotTheLock = app.requestSingleInstanceLock()

if (!gotTheLock) {
  app.quit()
} else {
  app.on('second-instance', (_event, argv) => {
    const targetPath = argv.find((arg) => {
      try {
        const resolved = validatePath(arg)
        return resolved && !arg.startsWith('-')
      } catch {
        return false
      }
    })

    if (targetPath) {
      const resolved = validatePath(targetPath)
      let existingWin: BrowserWindow | null = null
      for (const [win, path] of windowPaths.entries()) {
        if (path === resolved) {
          existingWin = win
          break
        }
      }

      if (existingWin) {
        if (existingWin.isMinimized()) existingWin.restore()
        existingWin.focus()
      } else {
        createWindow(resolved)
      }
    } else {
      const win = getMainWindow()
      if (win) {
        if (win.isMinimized()) win.restore()
        win.focus()
      }
    }
  })

  app.setName('Mdow')

  void app.whenReady().then(() => {
    registerLocalProtocol()

    const appState = getAppState()
    const validThemes: Array<typeof nativeTheme.themeSource> = ['light', 'dark', 'system']
    const theme = validThemes.find((v) => v === appState.theme)
    if (theme && theme !== 'system') {
      nativeTheme.themeSource = theme
    }
    cleanupIpcHandlers = registerIpcHandlers(getMainWindow)
    createMenu(getMainWindow)
    createWindow()
    scheduleAutoUpdaterInit()

    app.on('activate', () => {
      if (BrowserWindow.getAllWindows().length === 0) createWindow()
    })
  })

  app.on('window-all-closed', () => {
    if (!isMac) app.quit()
  })

  app.on('open-file', (event, path) => {
    event.preventDefault()
    openFile(path)
  })

  app.on('before-quit', () => {
    cleanupIpcHandlers?.()
    unwatchAllFiles()
    unwatchFolder()
  })
}
