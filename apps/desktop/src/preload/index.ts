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
  zoomLevel: number
  lastFolder: string | null
  windowBounds: { x: number; y: number; width: number; height: number } | null
  sessionTabs: { path: string }[]
  sessionActiveTabPath: string | null
  contentFont: string
  codeFont: string
  fontSize: number
  lineHeight: number
  theme: string
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
  setTheme: (theme: string) => Promise<void>

  onFileChanged: (callback: (data: { path: string; content: string }) => void) => () => void
  onFileDeleted: (callback: (path: string) => void) => () => void
  onFolderChanged: (callback: (tree: TreeNode[]) => void) => () => void
  onThemeChanged: (callback: (isDark: boolean) => void) => () => void
  onMenuOpenFile: (callback: () => void) => () => void
  onMenuOpenFolder: (callback: () => void) => () => void
  onFileOpened: (callback: (file: FileResult) => void) => () => void
  onMenuZoomIn: (callback: () => void) => () => void
  onMenuZoomOut: (callback: () => void) => () => void
  onMenuZoomReset: (callback: () => void) => () => void
  onMenuShortcuts: (callback: () => void) => () => void
  onMenuSettings: (callback: () => void) => () => void

  checkForUpdates: () => Promise<void>
  downloadUpdate: () => Promise<void>
  installUpdate: () => Promise<void>
  onUpdateAvailable: (
    callback: (info: { version: string; releaseNotes?: string }) => void,
  ) => () => void
  onUpdateUpToDate: (callback: () => void) => () => void
  onUpdateDownloadProgress: (callback: (progress: { percent: number }) => void) => () => void
  onUpdateDownloaded: (callback: () => void) => () => void
  onUpdateError: (callback: (message: string) => void) => () => void
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
  setTheme: (theme) => ipcRenderer.invoke('theme:set', theme),

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
  onMenuZoomIn: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:zoom-in', handler)
    return () => ipcRenderer.removeListener('menu:zoom-in', handler)
  },
  onMenuZoomOut: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:zoom-out', handler)
    return () => ipcRenderer.removeListener('menu:zoom-out', handler)
  },
  onMenuZoomReset: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:zoom-reset', handler)
    return () => ipcRenderer.removeListener('menu:zoom-reset', handler)
  },
  onMenuShortcuts: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:shortcuts', handler)
    return () => ipcRenderer.removeListener('menu:shortcuts', handler)
  },
  onMenuSettings: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:settings', handler)
    return () => ipcRenderer.removeListener('menu:settings', handler)
  },

  checkForUpdates: () => ipcRenderer.invoke('updater:check'),
  downloadUpdate: () => ipcRenderer.invoke('updater:download'),
  installUpdate: () => ipcRenderer.invoke('updater:install'),
  onUpdateAvailable: (callback) => {
    const handler = (
      _: Electron.IpcRendererEvent,
      info: { version: string; releaseNotes?: string },
    ) => callback(info)
    ipcRenderer.on('updater:update-available', handler)
    return () => ipcRenderer.removeListener('updater:update-available', handler)
  },
  onUpdateUpToDate: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('updater:up-to-date', handler)
    return () => ipcRenderer.removeListener('updater:up-to-date', handler)
  },
  onUpdateDownloadProgress: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, progress: { percent: number }) =>
      callback(progress)
    ipcRenderer.on('updater:download-progress', handler)
    return () => ipcRenderer.removeListener('updater:download-progress', handler)
  },
  onUpdateDownloaded: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('updater:update-downloaded', handler)
    return () => ipcRenderer.removeListener('updater:update-downloaded', handler)
  },
  onUpdateError: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, message: string) => callback(message)
    ipcRenderer.on('updater:error', handler)
    return () => ipcRenderer.removeListener('updater:error', handler)
  },
}

contextBridge.exposeInMainWorld('api', api)
