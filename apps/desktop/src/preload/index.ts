import { contextBridge, ipcRenderer, webUtils } from 'electron'
import type { ElectronAPI, FileResult, ScanResult } from '../shared/api-types'

export type { AppState, ElectronAPI, FileResult, ScanResult, TreeNode } from '../shared/api-types'

const api: ElectronAPI = {
  platform: process.platform,
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
  closeWindow: () => ipcRenderer.invoke('window:close'),
  setTheme: (theme) => ipcRenderer.invoke('theme:set', theme),
  getPathForFile: (file) => webUtils.getPathForFile(file),

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
    const handler = (_: Electron.IpcRendererEvent, scan: ScanResult) => callback(scan)
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
  onMenuFind: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:find', handler)
    return () => ipcRenderer.removeListener('menu:find', handler)
  },
  onMenuToggleSidebar: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:toggle-sidebar', handler)
    return () => ipcRenderer.removeListener('menu:toggle-sidebar', handler)
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
  onMenuCheckForUpdates: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:check-for-updates', handler)
    return () => ipcRenderer.removeListener('menu:check-for-updates', handler)
  },
  onMenuCloseTab: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:close-tab', handler)
    return () => ipcRenderer.removeListener('menu:close-tab', handler)
  },

  checkForUpdates: (opts?: { manual?: boolean }) => ipcRenderer.invoke('updater:check', opts),
  downloadUpdate: () => ipcRenderer.invoke('updater:download'),
  installUpdate: () => ipcRenderer.invoke('updater:install'),
  setAutoUpdateScheduling: (enabled: boolean) =>
    ipcRenderer.invoke('updater:set-scheduling', enabled),
  onUpdateAvailable: (callback) => {
    const handler = (
      _: Electron.IpcRendererEvent,
      info: { version: string; releaseNotes?: string },
    ) => callback(info)
    ipcRenderer.on('updater:update-available', handler)
    return () => ipcRenderer.removeListener('updater:update-available', handler)
  },
  onUpdateUpToDate: (callback: (info: { wasManual: boolean }) => void) => {
    const handler = (_: Electron.IpcRendererEvent, info: { wasManual: boolean }) => callback(info)
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
