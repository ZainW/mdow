import { app, BrowserWindow, nativeTheme } from 'electron'
import { join } from 'path'
import { is } from '@electron-toolkit/utils'
import { registerIpcHandlers } from './ipc'
import { createMenu } from './menu'
import { getWindowBounds, saveWindowBounds, getLastFolder } from './store'
import { scanFolder, watchFolder } from './folder-service'
import { readFileContent } from './file-service'

let mainWindow: BrowserWindow | null = null

function getMainWindow(): BrowserWindow | null {
  return mainWindow
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
      preload: join(__dirname, '../preload/index.js'),
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

  mainWindow.webContents.on('did-finish-load', async () => {
    mainWindow?.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)

    const lastFolder = getLastFolder()
    if (lastFolder) {
      try {
        const tree = await scanFolder(lastFolder)
        mainWindow?.webContents.send('folder:changed', tree)
        watchFolder(lastFolder, (newTree) => {
          mainWindow?.webContents.send('folder:changed', newTree)
        })
      } catch {
        // Folder no longer exists
      }
    }

    const filePath = process.argv.find((arg) =>
      arg.endsWith('.md') || arg.endsWith('.markdown') || arg.endsWith('.mdx')
    )
    if (filePath) {
      try {
        const content = await readFileContent(filePath)
        mainWindow?.webContents.send('file:opened', { path: filePath, content })
      } catch {
        // Invalid file path
      }
    }
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }

  mainWindow.on('closed', () => {
    mainWindow = null
  })
}

app.whenReady().then(() => {
  registerIpcHandlers(getMainWindow)
  createMenu(getMainWindow)
  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

app.on('open-file', async (event, path) => {
  event.preventDefault()
  if (mainWindow) {
    try {
      const content = await readFileContent(path)
      mainWindow.webContents.send('file:opened', { path, content })
    } catch {
      // Invalid file
    }
  }
})
