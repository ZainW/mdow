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

export interface FileResult {
  path: string
  content: string
}

export type ErrorType = 'not-found' | 'permission-denied' | 'deleted' | 'read-error'

export interface FileError {
  type: ErrorType
  path: string
}

export type SidebarMode = 'recents' | 'folder' | 'outline'
export type InterfaceScale = 'compact' | 'comfortable' | 'large'
export type ReadingWidth = 'standard' | 'comfortable' | 'wide'
export type PaneId = 'primary' | 'secondary'

export type CompanionProviderId = 'auto' | 'opencode' | 'codex' | 'custom'

export type CompanionProviderStatusState = 'available' | 'missing' | 'failed'

export interface CompanionProviderStatus {
  id: Exclude<CompanionProviderId, 'auto'>
  label: string
  command: string
  status: CompanionProviderStatusState
  installHint?: string
  error?: string
}

export interface CompanionSettings {
  provider: CompanionProviderId
  customCommand: string
}

export type CompanionMessageRole = 'user' | 'assistant' | 'system'
export type CompanionMessageStatus = 'complete' | 'streaming' | 'error'

export interface CompanionCitation {
  sourceId: string
  title: string
  path: string
  heading?: string
}

export interface CompanionContextSource {
  id: string
  title: string
  path: string
  heading?: string
}

export interface CompanionContextWarning {
  type: 'missing-file' | 'permission-denied' | 'truncated' | 'no-context'
  message: string
}

export interface CompanionContextSummary {
  activePath: string | null
  folderPath: string | null
  sourceCount: number
  truncated: boolean
  warnings: CompanionContextWarning[]
  sources: CompanionContextSource[]
}

export interface CompanionMessage {
  id: string
  role: CompanionMessageRole
  content: string
  status: CompanionMessageStatus
  citations: CompanionCitation[]
  createdAt: number
}

export type CompanionUpdate =
  | { type: 'status'; status: 'starting' | 'ready' | 'streaming' | 'complete' | 'cancelled' }
  | { type: 'assistant-delta'; messageId: string; text: string }
  | { type: 'context'; summary: CompanionContextSummary }
  | { type: 'warning'; warning: CompanionContextWarning }
  | { type: 'tool-refused'; title: string }
  | { type: 'error'; message: string }

export interface CompanionSendRequest {
  messageId: string
  text: string
  provider: CompanionProviderId
  activePath: string | null
  openFolderPath: string | null
}

export interface AppState {
  recents?: string[]
  zoomLevel: number
  lastFolder: string | null
  windowBounds: { x: number; y: number; width: number; height: number } | null
  sessionTabs: { path: string }[]
  sessionActiveTabPath: string | null
  sessionSplitView?: boolean
  sessionPrimaryPanePath?: string | null
  sessionSecondaryPanePath?: string | null
  sessionActivePane?: PaneId
  contentFont: string
  codeFont: string
  theme: string
  autoUpdateEnabled: boolean
  wideMode: boolean
  interfaceScale: InterfaceScale
  readingWidth: ReadingWidth
  sidebarMode: SidebarMode
  companionProvider: CompanionProviderId
  companionCustomCommand: string
}

export interface FolderOpenResult extends ScanResult {
  path: string
}

export const MD_EXTENSIONS = new Set(['.md', '.markdown', '.mdx'])
export const HTML_EXTENSIONS = new Set(['.html', '.htm'])
export const DOCUMENT_EXTENSIONS = new Set([...MD_EXTENSIONS, ...HTML_EXTENSIONS])

export function isMarkdownPath(path: string): boolean {
  const lower = path.toLowerCase()
  for (const ext of MD_EXTENSIONS) {
    if (lower.endsWith(ext)) return true
  }
  return false
}

export function isHtmlPath(path: string): boolean {
  const lower = path.toLowerCase()
  for (const ext of HTML_EXTENSIONS) {
    if (lower.endsWith(ext)) return true
  }
  return false
}

export function isDocumentPath(path: string): boolean {
  const lower = path.toLowerCase()
  for (const ext of DOCUMENT_EXTENSIONS) {
    if (lower.endsWith(ext)) return true
  }
  return false
}

export const IPC = {
  FILE_OPEN_DIALOG: 'file:open-dialog',
  FILE_READ: 'file:read',
  FILE_UNWATCH: 'file:unwatch',
  FILE_SET_ACTIVE_WATCH: 'file:set-active-watch',
  FILE_STAT: 'file:stat',
  FILE_CHANGED: 'file:changed',
  FILE_DELETED: 'file:deleted',
  FILE_OPENED: 'file:opened',
  FOLDER_OPEN_DIALOG: 'folder:open-dialog',
  FOLDER_OPEN_PATH: 'folder:open-path',
  FOLDER_READ_TREE: 'folder:read-tree',
  FOLDER_CHANGED: 'folder:changed',
  STORE_GET_RECENTS: 'store:get-recents',
  STORE_GET_STATE: 'store:get-state',
  STORE_SAVE_STATE: 'store:save-state',
  SHELL_SHOW_IN_FOLDER: 'shell:show-in-folder',
  SHELL_OPEN_EXTERNAL: 'shell:open-external',
  URL_RESOLVE_LOCAL: 'url:resolve-local',
  WINDOW_SET_TITLE: 'window:set-title',
  WINDOW_CLOSE: 'window:close',
  THEME_SET: 'theme:set',
  THEME_CHANGED: 'theme:changed',
  MENU_OPEN_FILE: 'menu:open-file',
  MENU_OPEN_FOLDER: 'menu:open-folder',
  MENU_OPEN_RECENT: 'menu:open-recent',
  MENU_FIND: 'menu:find',
  MENU_TOGGLE_SIDEBAR: 'menu:toggle-sidebar',
  MENU_ZOOM_IN: 'menu:zoom-in',
  MENU_ZOOM_OUT: 'menu:zoom-out',
  MENU_ZOOM_RESET: 'menu:zoom-reset',
  MENU_SHORTCUTS: 'menu:shortcuts',
  MENU_SETTINGS: 'menu:settings',
  MENU_CLOSE_TAB: 'menu:close-tab',
  MENU_CHECK_FOR_UPDATES: 'menu:check-for-updates',
  UPDATER_CHECK: 'updater:check',
  UPDATER_SET_SCHEDULING: 'updater:set-scheduling',
  UPDATER_DOWNLOAD: 'updater:download',
  UPDATER_INSTALL: 'updater:install',
  UPDATER_UPDATE_AVAILABLE: 'updater:update-available',
  UPDATER_UP_TO_DATE: 'updater:up-to-date',
  UPDATER_DOWNLOAD_PROGRESS: 'updater:download-progress',
  UPDATER_UPDATE_DOWNLOADED: 'updater:update-downloaded',
  UPDATER_ERROR: 'updater:error',
  COMPANION_DETECT_PROVIDERS: 'companion:detect-providers',
  COMPANION_GET_SETTINGS: 'companion:get-settings',
  COMPANION_SAVE_SETTINGS: 'companion:save-settings',
  COMPANION_START_SESSION: 'companion:start-session',
  COMPANION_SEND: 'companion:send',
  COMPANION_CANCEL: 'companion:cancel',
  COMPANION_SHUTDOWN: 'companion:shutdown',
  COMPANION_UPDATE: 'companion:update',
} as const
