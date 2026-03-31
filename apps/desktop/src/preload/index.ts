import { contextBridge, ipcRenderer } from 'electron'

export interface FileResult {
  path: string
  content: string
}

export interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

export interface AppState {
  sidebarWidth: number
  lastFolder: string | null
  windowBounds: { x: number; y: number; width: number; height: number } | null
}

export interface ElectronAPI {
  openFileDialog: () => Promise<FileResult | null>
  readFile: (path: string) => Promise<string>
  unwatchFile: (path: string) => Promise<void>
  openFolderDialog: () => Promise<{ path: string; tree: TreeNode[] } | null>
  readFolderTree: (folderPath: string) => Promise<TreeNode[]>
  getRecents: () => Promise<string[]>
  getAppState: () => Promise<AppState>
  saveAppState: (state: Partial<AppState>) => Promise<void>
  addRecent: (filePath: string) => Promise<void>
  showInFolder: (filePath: string) => Promise<void>
  setWindowTitle: (title: string, filePath?: string) => Promise<void>

  onFileChanged: (callback: (data: { path: string; content: string }) => void) => () => void
  onFileDeleted: (callback: (path: string) => void) => () => void
  onFolderChanged: (callback: (tree: TreeNode[]) => void) => () => void
  onThemeChanged: (callback: (isDark: boolean) => void) => () => void
  onMenuOpenFile: (callback: () => void) => () => void
  onMenuOpenFolder: (callback: () => void) => () => void
  onFileOpened: (callback: (file: FileResult) => void) => () => void
}

const api: ElectronAPI = {
  openFileDialog: () => ipcRenderer.invoke('file:open-dialog'),
  readFile: (path) => ipcRenderer.invoke('file:read', path),
  unwatchFile: (path) => ipcRenderer.invoke('file:unwatch', path),
  openFolderDialog: () => ipcRenderer.invoke('folder:open-dialog'),
  readFolderTree: (folderPath) => ipcRenderer.invoke('folder:read-tree', folderPath),
  getRecents: () => ipcRenderer.invoke('store:get-recents'),
  getAppState: () => ipcRenderer.invoke('store:get-state'),
  saveAppState: (state) => ipcRenderer.invoke('store:save-state', state),
  addRecent: (filePath) => ipcRenderer.invoke('store:add-recent', filePath),
  showInFolder: (filePath) => ipcRenderer.invoke('shell:show-in-folder', filePath),
  setWindowTitle: (title, filePath) => ipcRenderer.invoke('window:set-title', title, filePath),

  onFileChanged: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, data: { path: string; content: string }) =>
      callback(data)
    ipcRenderer.on('file:changed', handler)
    return () => ipcRenderer.removeListener('file:changed', handler)
  },
  onFileDeleted: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, path: string) => callback(path)
    ipcRenderer.on('file:deleted', handler)
    return () => ipcRenderer.removeListener('file:deleted', handler)
  },
  onFolderChanged: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, tree: TreeNode[]) => callback(tree)
    ipcRenderer.on('folder:changed', handler)
    return () => ipcRenderer.removeListener('folder:changed', handler)
  },
  onThemeChanged: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, isDark: boolean) => callback(isDark)
    ipcRenderer.on('theme:changed', handler)
    return () => ipcRenderer.removeListener('theme:changed', handler)
  },
  onMenuOpenFile: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:open-file', handler)
    return () => ipcRenderer.removeListener('menu:open-file', handler)
  },
  onMenuOpenFolder: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:open-folder', handler)
    return () => ipcRenderer.removeListener('menu:open-folder', handler)
  },
  onFileOpened: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, file: FileResult) => callback(file)
    ipcRenderer.on('file:opened', handler)
    return () => ipcRenderer.removeListener('file:opened', handler)
  },
}

contextBridge.exposeInMainWorld('api', api)
