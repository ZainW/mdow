import { app, BrowserWindow, nativeTheme } from 'electron'
import { join } from 'path'
import { is } from '@electron-toolkit/utils'
import { registerIpcHandlers } from './ipc'
import { createMenu } from './menu'
import { initAutoUpdater } from './updater'
import { addRecent, getWindowBounds, saveWindowBounds, getAppState } from './store'
import { unwatchFolder } from './folder-service'
import { readFileContent, unwatchAllFiles, watchFile } from './file-service'
import { isMac, isLinux } from './platform'
import { applyWindowChrome, getWindowChromeOptions } from './window-chrome'

let mainWindow: BrowserWindow | null = null

const markdownExtensions = ['.md', '.markdown', '.mdx']

function getMainWindow(): BrowserWindow | null {
  return mainWindow
}

function isMarkdownPath(path: string): boolean {
  return markdownExtensions.some((extension) => path.toLowerCase().endsWith(extension))
}

function watchOpenedFile(filePath: string): void {
  watchFile(filePath, (event) => {
    const win = getMainWindow()
    if (!win || win.isDestroyed()) return
    if (event.type === 'changed') {
      win.webContents.send('file:changed', { path: filePath, content: event.content })
    } else {
      win.webContents.send('file:deleted', filePath)
    }
  })
}

function openFile(filePath: string): void {
  void readFileContent(filePath)
    .then((content) => {
      addRecent(filePath)
      watchOpenedFile(filePath)
      mainWindow?.webContents.send('file:opened', { path: filePath, content })
    })
    .catch(() => {
      // Invalid file path
    })
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
    },
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow!.show()
  })

  const saveBounds = () => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      saveWindowBounds(mainWindow.getBounds())
    }
  }
  mainWindow.on('resized', saveBounds)
  mainWindow.on('moved', saveBounds)

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
    mainWindow = null
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
    if (mainWindow) {
      openFile(path)
    }
  })

  app.on('before-quit', () => {
    unwatchAllFiles()
    unwatchFolder()
  })
}
