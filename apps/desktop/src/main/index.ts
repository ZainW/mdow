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
import { initAutoUpdater } from './updater'
import { addRecent, getWindowBounds, saveWindowBounds, getAppState } from './store'
import { unwatchFolder } from './folder-service'
import { readFileContent, unwatchAllFiles, attachFileWatcher } from './file-service'
import { isMac, isLinux } from './platform'
import { applyWindowChrome, getWindowChromeOptions } from './window-chrome'
import { isMarkdownPath, validateMarkdownPath } from './path-validation'
import { registerAllowedFile, isPathAllowed, clearAllowedPaths } from './allowed-paths'

let mainWindow: BrowserWindow | null = null
let windowReady = false
let pendingOpenPath: string | null = null

function getMainWindow(): BrowserWindow | null {
  return mainWindow
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
  if (!mainWindow || !windowReady) {
    pendingOpenPath = filePath
    return
  }

  void (async () => {
    try {
      const resolved = validateMarkdownPath(filePath)
      const content = await readFileContent(resolved)
      addRecent(resolved)
      registerAllowedFile(resolved)
      if (isMac) {
        app.addRecentDocument(resolved)
      }
      attachFileWatcher(getMainWindow, resolved)
      mainWindow?.webContents.send('file:opened', { path: resolved, content })
    } catch {
      // Invalid file path
    }
  })()
}

function flushPendingOpen(): void {
  if (pendingOpenPath) {
    const path = pendingOpenPath
    pendingOpenPath = null
    openFile(path)
  }
}

function openFileFromArgv(argv: string[]): void {
  const filePath = argv.find(isMarkdownPath)
  if (filePath) {
    openFile(filePath)
  }
}

function createWindow(): void {
  const savedBounds = getWindowBounds()

  mainWindow = new BrowserWindow({
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

  if (savedBounds?.isMaximized) {
    mainWindow.maximize()
  }

  setupWebContentsSecurity(mainWindow)

  mainWindow.on('ready-to-show', () => {
    windowReady = true
    mainWindow!.show()
    flushPendingOpen()
  })

  const saveBounds = () => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      saveWindowBounds(mainWindow.getBounds(), mainWindow.isMaximized())
    }
  }
  mainWindow.on('resized', saveBounds)
  mainWindow.on('moved', saveBounds)
  mainWindow.on('maximize', saveBounds)
  mainWindow.on('unmaximize', saveBounds)

  const handleNativeThemeUpdate = () => {
    if (mainWindow) applyWindowChrome(mainWindow)
    mainWindow?.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)
  }
  nativeTheme.on('updated', handleNativeThemeUpdate)

  mainWindow.webContents.on('did-finish-load', () => {
    mainWindow?.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)
    openFileFromArgv(process.argv)
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    void mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    void mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }

  mainWindow.on('closed', () => {
    nativeTheme.off('updated', handleNativeThemeUpdate)
    unwatchAllFiles()
    unwatchFolder()
    clearAllowedPaths()
    mainWindow = null
    windowReady = false
  })
}

const gotTheLock = app.requestSingleInstanceLock()

if (!gotTheLock) {
  app.quit()
} else {
  app.on('second-instance', (_event, argv) => {
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore()
      mainWindow.focus()
      openFileFromArgv(argv)
    }
  })

  app.setName('Mdow')

  if (isMac && is.dev) {
    // Each Electron version bump re-prompts for Keychain access because the
    // dev binary is ad-hoc signed. Mock the keychain in dev — prod builds
    // with a real Developer ID signature don't have this problem.
    app.commandLine.appendSwitch('use-mock-keychain')
  }

  void app.whenReady().then(() => {
    registerLocalProtocol()

    const appState = getAppState()
    const validThemes: Array<typeof nativeTheme.themeSource> = ['light', 'dark', 'system']
    const theme = validThemes.find((v) => v === appState.theme)
    if (theme && theme !== 'system') {
      nativeTheme.themeSource = theme
    }
    registerIpcHandlers(getMainWindow)
    createMenu(getMainWindow)
    createWindow()
    initAutoUpdater(getMainWindow)

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
    unwatchAllFiles()
    unwatchFolder()
  })
}
