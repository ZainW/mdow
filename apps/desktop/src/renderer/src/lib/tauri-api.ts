import type { ElectronAPI, FileResult, Platform, ScanResult } from '../../../shared/api-types'

const MD_EXTENSIONS = ['.md', '.markdown', '.mdx']

function detectPlatformFallback(): Platform {
  if (typeof navigator === 'undefined') return 'linux'

  const platform = navigator.platform?.toLowerCase() ?? ''
  const ua = navigator.userAgent.toLowerCase()

  if (platform.includes('mac') || ua.includes('mac')) return 'darwin'
  if (platform.includes('win') || ua.includes('win')) return 'win32'
  if (platform.includes('linux') || ua.includes('linux')) return 'linux'
  if (platform.includes('freebsd')) return 'freebsd'
  if (platform.includes('openbsd')) return 'openbsd'
  if (platform.includes('netbsd')) return 'netbsd'

  return 'linux'
}

function isMarkdownPath(path: string): boolean {
  const lower = path.toLowerCase()
  return MD_EXTENSIONS.some((ext) => lower.endsWith(ext))
}

const PLATFORMS = new Set<Platform>([
  'aix',
  'android',
  'darwin',
  'freebsd',
  'haiku',
  'linux',
  'openbsd',
  'sunos',
  'win32',
  'cygwin',
  'netbsd',
])

function isPlatform(value: string): value is Platform {
  for (const platform of PLATFORMS) {
    if (platform === value) return true
  }
  return false
}

type TauriEventPayloads = {
  'file:changed': { path: string; content: string }
  'file:deleted': string
  'folder:changed': ScanResult
  'theme:changed': boolean
  'file:opened': FileResult
}

function subscribe<E extends keyof TauriEventPayloads>(
  listen: typeof import('@tauri-apps/api/event').listen,
  event: E,
  callback: (payload: TauriEventPayloads[E]) => void,
): () => void {
  let unlisten: (() => void) | null = null
  let cancelled = false

  void listen(event, (eventPayload: { payload: TauriEventPayloads[E] }) => {
    callback(eventPayload.payload)
  }).then((fn) => {
    if (cancelled) {
      fn()
    } else {
      unlisten = fn
    }
  })

  return () => {
    cancelled = true
    unlisten?.()
  }
}

const noopUnsub = (): void => {}

async function installDragDropFallback(
  invoke: typeof import('@tauri-apps/api/core').invoke,
  notifyFileOpened: (file: FileResult) => void,
): Promise<void> {
  if (typeof window === 'undefined') return

  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const appWindow = getCurrentWindow()

    await appWindow.onDragDropEvent((event) => {
      if (event.payload.type !== 'drop') return

      for (const path of event.payload.paths) {
        if (!isMarkdownPath(path)) continue

        void invoke<string>('read_file', { path }).then(
          (content) => notifyFileOpened({ path, content }),
          () => {},
        )
      }
    })
  } catch {
    // Drag-drop is optional; Rust may emit file:opened instead.
  }
}

export async function createTauriApi(): Promise<ElectronAPI> {
  const { invoke } = await import('@tauri-apps/api/core')
  const { listen } = await import('@tauri-apps/api/event')

  let platform: Platform = detectPlatformFallback()
  try {
    const platformResult = await invoke<string>('platform')
    if (isPlatform(platformResult)) {
      platform = platformResult
    }
  } catch {
    // Keep browser-derived fallback when platform command is unavailable.
  }

  const fileOpenedCallbacks = new Set<(file: FileResult) => void>()

  const notifyFileOpened = (file: FileResult) => {
    fileOpenedCallbacks.forEach((cb) => cb(file))
  }

  const api: ElectronAPI = {
    platform,

    openFileDialog: () => invoke('open_file_dialog'),
    readFile: (path) => invoke('read_file', { path }),
    unwatchFile: (path) => invoke('unwatch_file', { path }),
    openFolderDialog: () => invoke('open_folder_dialog'),
    readFolderTree: (folderPath) => invoke('read_folder_tree', { folderPath }),
    getRecents: () => invoke('store_get_recents'),
    getAppState: () => invoke('store_get_state'),
    saveAppState: (state) => invoke('store_save_state', { patch: state }),
    addRecent: (filePath) => invoke('store_add_recent', { filePath }),
    showInFolder: (filePath) => invoke('show_in_folder', { filePath }),
    setWindowTitle: (title) => invoke('set_window_title', { title }),
    closeWindow: () => invoke('close_window'),
    setTheme: (theme) => invoke('set_theme', { theme }),

    getPathForFile: (file) => {
      const withPath = file as File & { path?: string }
      return withPath.path ?? file.name
    },

    onFileChanged: (cb) => subscribe(listen, 'file:changed', cb),
    onFileDeleted: (cb) => subscribe(listen, 'file:deleted', cb),
    onFolderChanged: (cb) => subscribe(listen, 'folder:changed', cb),
    onThemeChanged: (cb) => subscribe(listen, 'theme:changed', cb),
    onFileOpened: (cb) => {
      fileOpenedCallbacks.add(cb)
      const unsub = subscribe(listen, 'file:opened', cb)
      return () => {
        fileOpenedCallbacks.delete(cb)
        unsub()
      }
    },

    onMenuOpenFile: () => noopUnsub,
    onMenuOpenFolder: () => noopUnsub,
    onMenuFind: () => noopUnsub,
    onMenuToggleSidebar: () => noopUnsub,
    onMenuZoomIn: () => noopUnsub,
    onMenuZoomOut: () => noopUnsub,
    onMenuZoomReset: () => noopUnsub,
    onMenuShortcuts: () => noopUnsub,
    onMenuSettings: () => noopUnsub,
    onMenuCloseTab: () => noopUnsub,
    onMenuCheckForUpdates: () => noopUnsub,

    checkForUpdates: async () => {
      await invoke('check_for_updates')
    },
    downloadUpdate: async () => {
      await invoke('download_update')
    },
    installUpdate: async () => {
      await invoke('install_update')
    },
    setAutoUpdateScheduling: async (enabled) => {
      await invoke('set_auto_update_scheduling', { enabled })
    },
    onUpdateAvailable: () => noopUnsub,
    onUpdateUpToDate: () => noopUnsub,
    onUpdateDownloadProgress: () => noopUnsub,
    onUpdateDownloaded: () => noopUnsub,
    onUpdateError: () => noopUnsub,
  }

  void installDragDropFallback(invoke, notifyFileOpened)

  return api
}
