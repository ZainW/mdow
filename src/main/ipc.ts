import { ipcMain, shell, BrowserWindow } from 'electron'
import { openFileDialog, readFileContent, watchFile } from './file-service'
import { openFolderDialog, scanFolder, watchFolder } from './folder-service'
import { getRecents, addRecent, getAppState, saveAppState, setLastFolder } from './store'

export function registerIpcHandlers(getMainWindow: () => BrowserWindow | null): void {
  ipcMain.handle('file:open-dialog', async () => {
    const win = getMainWindow()
    if (!win) return null
    const result = await openFileDialog(win)
    if (result) {
      addRecent(result.path)
      watchFile(result.path, (content) => {
        win.webContents.send('file:changed', content)
      })
    }
    return result
  })

  ipcMain.handle('file:read', async (_, path: string) => {
    const content = await readFileContent(path)
    const win = getMainWindow()
    if (win) {
      addRecent(path)
      watchFile(path, (newContent) => {
        win.webContents.send('file:changed', newContent)
      })
    }
    return content
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
  ipcMain.handle('store:save-state', (_, state) => saveAppState(state))
  ipcMain.handle('store:add-recent', (_, filePath: string) => addRecent(filePath))

  ipcMain.handle('shell:show-in-folder', (_, filePath: string) => {
    shell.showItemInFolder(filePath)
  })
}
