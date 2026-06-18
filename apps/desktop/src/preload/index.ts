import { contextBridge, ipcRenderer, webUtils } from 'electron'
import {
  IPC,
  type AppState,
  type FileResult,
  type FolderOpenResult,
  type ScanResult,
  type TreeNode,
} from '../shared/types'

export type {
  AppState,
  ErrorType,
  FileError,
  FileResult,
  ScanResult,
  TreeNode,
} from '../shared/types'
export {
  DOCUMENT_EXTENSIONS,
  HTML_EXTENSIONS,
  MD_EXTENSIONS,
  isDocumentPath,
  isHtmlPath,
  isMarkdownPath,
} from '../shared/types'

type Unsubscribe = () => void

function createIpcListener(channel: string, callback: () => void): Unsubscribe
function createIpcListener<T>(channel: string, callback: (payload: T) => void): Unsubscribe
function createIpcListener<T>(
  channel: string,
  callback: ((payload: T) => void) | (() => void),
): Unsubscribe {
  const handler = (_: Electron.IpcRendererEvent, payload?: T) => {
    if (payload === undefined) {
      ;(callback as () => void)()
    } else {
      ;(callback as (payload: T) => void)(payload)
    }
  }
  ipcRenderer.on(channel, handler)
  return () => ipcRenderer.removeListener(channel, handler)
}

export interface ElectronAPI {
  platform: NodeJS.Platform
  openFileDialog: () => Promise<FileResult | null>
  readFile: (path: string) => Promise<string>
  statFile: (path: string) => Promise<{ exists: boolean; isFile: boolean; isDirectory: boolean }>
  unwatchFile: (path: string) => Promise<void>
  setActiveFileWatch: (path: string | null) => Promise<void>
  openFolderDialog: () => Promise<{ path: string; tree: TreeNode[]; truncated: boolean } | null>
  openFolderPath: (path: string) => Promise<FolderOpenResult>
  readFolderTree: (folderPath: string) => Promise<ScanResult>
  getRecents: () => Promise<string[]>
  getAppState: () => Promise<AppState>
  saveAppState: (state: Partial<AppState>) => Promise<void>
  showInFolder: (filePath: string) => Promise<void>
  openExternal: (url: string) => Promise<void>
  resolveLocalUrl: (path: string) => Promise<string>
  setWindowTitle: (title: string, filePath?: string) => Promise<void>
  closeWindow: () => Promise<void>
  setTheme: (theme: string) => Promise<void>
  getPathForFile: (file: File) => string

  onFileChanged: (callback: (data: { path: string; content: string }) => void) => Unsubscribe
  onFileDeleted: (callback: (path: string) => void) => Unsubscribe
  onFolderChanged: (callback: (scan: ScanResult) => void) => Unsubscribe
  onThemeChanged: (callback: (isDark: boolean) => void) => Unsubscribe
  onMenuOpenFile: (callback: () => void) => Unsubscribe
  onMenuOpenFolder: (callback: () => void) => Unsubscribe
  onMenuFind: (callback: () => void) => Unsubscribe
  onMenuToggleSidebar: (callback: () => void) => Unsubscribe
  onFileOpened: (callback: (file: FileResult) => void) => Unsubscribe
  onMenuZoomIn: (callback: () => void) => Unsubscribe
  onMenuZoomOut: (callback: () => void) => Unsubscribe
  onMenuZoomReset: (callback: () => void) => Unsubscribe
  onMenuShortcuts: (callback: () => void) => Unsubscribe
  onMenuSettings: (callback: () => void) => Unsubscribe
  onMenuCloseTab: (callback: () => void) => Unsubscribe

  checkForUpdates: (opts?: { manual?: boolean }) => Promise<void>
  downloadUpdate: () => Promise<void>
  installUpdate: () => Promise<void>
  setAutoUpdateScheduling: (enabled: boolean) => Promise<void>
  onUpdateAvailable: (
    callback: (info: { version: string; releaseNotes?: string }) => void,
  ) => Unsubscribe
  onUpdateUpToDate: (callback: (info: { wasManual: boolean }) => void) => Unsubscribe
  onUpdateDownloadProgress: (callback: (progress: { percent: number }) => void) => Unsubscribe
  onUpdateDownloaded: (callback: () => void) => Unsubscribe
  onUpdateError: (callback: (message: string) => void) => Unsubscribe
  onMenuCheckForUpdates: (callback: () => void) => Unsubscribe
}

const api: ElectronAPI = {
  platform: process.platform,
  openFileDialog: () => ipcRenderer.invoke(IPC.FILE_OPEN_DIALOG),
  readFile: (path) => ipcRenderer.invoke(IPC.FILE_READ, path),
  statFile: (path) => ipcRenderer.invoke(IPC.FILE_STAT, path),
  unwatchFile: (path) => ipcRenderer.invoke(IPC.FILE_UNWATCH, path),
  setActiveFileWatch: (path) => ipcRenderer.invoke(IPC.FILE_SET_ACTIVE_WATCH, path),
  openFolderDialog: () => ipcRenderer.invoke(IPC.FOLDER_OPEN_DIALOG),
  openFolderPath: (path) => ipcRenderer.invoke(IPC.FOLDER_OPEN_PATH, path),
  readFolderTree: (folderPath) => ipcRenderer.invoke(IPC.FOLDER_READ_TREE, folderPath),
  getRecents: () => ipcRenderer.invoke(IPC.STORE_GET_RECENTS),
  getAppState: () => ipcRenderer.invoke(IPC.STORE_GET_STATE),
  saveAppState: (state) => ipcRenderer.invoke(IPC.STORE_SAVE_STATE, state),
  showInFolder: (filePath) => ipcRenderer.invoke(IPC.SHELL_SHOW_IN_FOLDER, filePath),
  openExternal: (url) => ipcRenderer.invoke(IPC.SHELL_OPEN_EXTERNAL, url),
  resolveLocalUrl: (path) => ipcRenderer.invoke(IPC.URL_RESOLVE_LOCAL, path),
  setWindowTitle: (title, filePath) => ipcRenderer.invoke(IPC.WINDOW_SET_TITLE, title, filePath),
  closeWindow: () => ipcRenderer.invoke(IPC.WINDOW_CLOSE),
  setTheme: (theme) => ipcRenderer.invoke(IPC.THEME_SET, theme),
  getPathForFile: (file) => webUtils.getPathForFile(file),

  onFileChanged: (callback) => createIpcListener(IPC.FILE_CHANGED, callback),
  onFileDeleted: (callback) => createIpcListener(IPC.FILE_DELETED, callback),
  onFolderChanged: (callback) => createIpcListener(IPC.FOLDER_CHANGED, callback),
  onThemeChanged: (callback) => createIpcListener(IPC.THEME_CHANGED, callback),
  onMenuOpenFile: (callback) => createIpcListener(IPC.MENU_OPEN_FILE, callback),
  onMenuOpenFolder: (callback) => createIpcListener(IPC.MENU_OPEN_FOLDER, callback),
  onMenuFind: (callback) => createIpcListener(IPC.MENU_FIND, callback),
  onMenuToggleSidebar: (callback) => createIpcListener(IPC.MENU_TOGGLE_SIDEBAR, callback),
  onFileOpened: (callback) => createIpcListener(IPC.FILE_OPENED, callback),
  onMenuZoomIn: (callback) => createIpcListener(IPC.MENU_ZOOM_IN, callback),
  onMenuZoomOut: (callback) => createIpcListener(IPC.MENU_ZOOM_OUT, callback),
  onMenuZoomReset: (callback) => createIpcListener(IPC.MENU_ZOOM_RESET, callback),
  onMenuShortcuts: (callback) => createIpcListener(IPC.MENU_SHORTCUTS, callback),
  onMenuSettings: (callback) => createIpcListener(IPC.MENU_SETTINGS, callback),
  onMenuCloseTab: (callback) => createIpcListener(IPC.MENU_CLOSE_TAB, callback),
  onMenuCheckForUpdates: (callback) => createIpcListener(IPC.MENU_CHECK_FOR_UPDATES, callback),

  checkForUpdates: (opts?: { manual?: boolean }) => ipcRenderer.invoke(IPC.UPDATER_CHECK, opts),
  downloadUpdate: () => ipcRenderer.invoke(IPC.UPDATER_DOWNLOAD),
  installUpdate: () => ipcRenderer.invoke(IPC.UPDATER_INSTALL),
  setAutoUpdateScheduling: (enabled: boolean) =>
    ipcRenderer.invoke(IPC.UPDATER_SET_SCHEDULING, enabled),
  onUpdateAvailable: (callback) => createIpcListener(IPC.UPDATER_UPDATE_AVAILABLE, callback),
  onUpdateUpToDate: (callback) => createIpcListener(IPC.UPDATER_UP_TO_DATE, callback),
  onUpdateDownloadProgress: (callback) =>
    createIpcListener(IPC.UPDATER_DOWNLOAD_PROGRESS, callback),
  onUpdateDownloaded: (callback) => createIpcListener(IPC.UPDATER_UPDATE_DOWNLOADED, callback),
  onUpdateError: (callback) => createIpcListener(IPC.UPDATER_ERROR, callback),
}

contextBridge.exposeInMainWorld('api', api)
