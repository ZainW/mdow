import { app, BrowserWindow, nativeTheme } from 'electron'
import { join } from 'path'
import { is } from '@electron-toolkit/utils'
import { registerIpcHandlers } from './ipc'
import { createMenu } from './menu'
import { initAutoUpdater } from './updater'
import { getWindowBounds, saveWindowBounds, getLastFolder, getAppState } from './store'
import { scanFolder, watchFolder } from './folder-service'
import { readFileContent } from './file-service'

let mainWindow: BrowserWindow | null = null

function getMainWindow(): BrowserWindow | null {
  return mainWindow
}

function openFileFromArgv(argv: string[]): void {
  const filePath = argv.find(
    (arg) => arg.endsWith('.md') || arg.endsWith('.markdown') || arg.endsWith('.mdx'),
  )
  if (filePath) {
    void readFileContent(filePath)
      .then((content) => {
        mainWindow?.webContents.send('file:opened', { path: filePath, content })
      })
      .catch(() => {
        // Invalid file path
      })
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

  nativeTheme.on('updated', () => {
    mainWindow?.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)
  })

  mainWindow.webContents.on('did-finish-load', () => {
    mainWindow?.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)

    const lastFolder = getLastFolder()
    if (lastFolder) {
      void scanFolder(lastFolder)
        .then((tree) => {
          mainWindow?.webContents.send('folder:changed', tree)
          watchFolder(lastFolder, (newTree) => {
            mainWindow?.webContents.send('folder:changed', newTree)
          })
        })
        .catch(() => {
          // Folder no longer exists
        })
    }

    openFileFromArgv(process.argv)
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    void mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    void mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }

  mainWindow.on('closed', () => {
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

  void app.whenReady().then(() => {
    const appState = getAppState()
    if (appState.theme && appState.theme !== 'system') {
      nativeTheme.themeSource = appState.theme as typeof nativeTheme.themeSource
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
    if (process.platform !== 'darwin') app.quit()
  })

  app.on('open-file', (event, path) => {
    event.preventDefault()
    if (mainWindow) {
      void readFileContent(path)
        .then((content) => {
          mainWindow?.webContents.send('file:opened', { path, content })
        })
        .catch(() => {
          // Invalid file
        })
    }
  })
}
