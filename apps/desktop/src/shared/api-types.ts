export type Platform =
  | 'aix'
  | 'android'
  | 'darwin'
  | 'freebsd'
  | 'haiku'
  | 'linux'
  | 'openbsd'
  | 'sunos'
  | 'win32'
  | 'cygwin'
  | 'netbsd'

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

export interface ScanResult {
  tree: TreeNode[]
  truncated: boolean
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
  autoUpdateEnabled: boolean
}

export interface ElectronAPI {
  platform: Platform
  openFileDialog: () => Promise<FileResult | null>
  readFile: (path: string) => Promise<string>
  unwatchFile: (path: string) => Promise<void>
  openFolderDialog: () => Promise<{ path: string; tree: TreeNode[]; truncated: boolean } | null>
  readFolderTree: (folderPath: string) => Promise<ScanResult>
  getRecents: () => Promise<string[]>
  getAppState: () => Promise<AppState>
  saveAppState: (state: Partial<AppState>) => Promise<void>
  addRecent: (filePath: string) => Promise<void>
  showInFolder: (filePath: string) => Promise<void>
  setWindowTitle: (title: string, filePath?: string) => Promise<void>
  closeWindow: () => Promise<void>
  setTheme: (theme: string) => Promise<void>
  getPathForFile: (file: File) => string

  onFileChanged: (callback: (data: { path: string; content: string }) => void) => () => void
  onFileDeleted: (callback: (path: string) => void) => () => void
  onFolderChanged: (callback: (scan: ScanResult) => void) => () => void
  onThemeChanged: (callback: (isDark: boolean) => void) => () => void
  onMenuOpenFile: (callback: () => void) => () => void
  onMenuOpenFolder: (callback: () => void) => () => void
  onMenuFind: (callback: () => void) => () => void
  onMenuToggleSidebar: (callback: () => void) => () => void
  onFileOpened: (callback: (file: FileResult) => void) => () => void
  onMenuZoomIn: (callback: () => void) => () => void
  onMenuZoomOut: (callback: () => void) => () => void
  onMenuZoomReset: (callback: () => void) => () => void
  onMenuShortcuts: (callback: () => void) => () => void
  onMenuSettings: (callback: () => void) => () => void
  onMenuCloseTab: (callback: () => void) => () => void

  checkForUpdates: (opts?: { manual?: boolean }) => Promise<void>
  downloadUpdate: () => Promise<void>
  installUpdate: () => Promise<void>
  setAutoUpdateScheduling: (enabled: boolean) => Promise<void>
  onUpdateAvailable: (
    callback: (info: { version: string; releaseNotes?: string }) => void,
  ) => () => void
  onUpdateUpToDate: (callback: (info: { wasManual: boolean }) => void) => () => void
  onUpdateDownloadProgress: (callback: (progress: { percent: number }) => void) => () => void
  onUpdateDownloaded: (callback: () => void) => () => void
  onUpdateError: (callback: (message: string) => void) => () => void
  onMenuCheckForUpdates: (callback: () => void) => () => void
}
