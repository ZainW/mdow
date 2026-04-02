import Store from 'electron-store'

interface SessionTab {
  path: string
}

interface StoreSchema {
  recents: string[]
  lastFolder: string | null
  sidebarWidth: number
  zoomLevel: number
  windowBounds: { x: number; y: number; width: number; height: number } | null
  sessionTabs: SessionTab[]
  sessionActiveTabPath: string | null
  contentFont: string
  codeFont: string
  fontSize: number
  lineHeight: number
  theme: string
}

const store = new Store<StoreSchema>({
  defaults: {
    recents: [],
    lastFolder: null,
    sidebarWidth: 260,
    zoomLevel: 100,
    windowBounds: null,
    sessionTabs: [],
    sessionActiveTabPath: null,
    contentFont: 'inter',
    codeFont: 'geist-mono',
    fontSize: 15.5,
    lineHeight: 1.65,
    theme: 'system',
  },
})

const MAX_RECENTS = 20

export function getRecents(): string[] {
  return store.get('recents')
}

export function addRecent(filePath: string): void {
  const recents = store.get('recents').filter((r) => r !== filePath)
  recents.unshift(filePath)
  store.set('recents', recents.slice(0, MAX_RECENTS))
}

export function getAppState() {
  return {
    sidebarWidth: store.get('sidebarWidth'),
    zoomLevel: store.get('zoomLevel'),
    lastFolder: store.get('lastFolder'),
    windowBounds: store.get('windowBounds'),
    sessionTabs: store.get('sessionTabs'),
    sessionActiveTabPath: store.get('sessionActiveTabPath'),
    contentFont: store.get('contentFont'),
    codeFont: store.get('codeFont'),
    fontSize: store.get('fontSize'),
    lineHeight: store.get('lineHeight'),
    theme: store.get('theme'),
  }
}

export function saveAppState(state: Partial<StoreSchema>): void {
  if (state.sidebarWidth !== undefined) store.set('sidebarWidth', state.sidebarWidth)
  if (state.zoomLevel !== undefined) store.set('zoomLevel', state.zoomLevel)
  if (state.lastFolder !== undefined) store.set('lastFolder', state.lastFolder)
  if (state.windowBounds !== undefined) store.set('windowBounds', state.windowBounds)
  if (state.recents !== undefined) store.set('recents', state.recents)
  if (state.sessionTabs !== undefined) store.set('sessionTabs', state.sessionTabs)
  if (state.sessionActiveTabPath !== undefined)
    store.set('sessionActiveTabPath', state.sessionActiveTabPath)
  if (state.contentFont !== undefined) store.set('contentFont', state.contentFont)
  if (state.codeFont !== undefined) store.set('codeFont', state.codeFont)
  if (state.fontSize !== undefined) store.set('fontSize', state.fontSize)
  if (state.lineHeight !== undefined) store.set('lineHeight', state.lineHeight)
  if (state.theme !== undefined) store.set('theme', state.theme)
}

export function getWindowBounds() {
  return store.get('windowBounds')
}

export function saveWindowBounds(bounds: { x: number; y: number; width: number; height: number }) {
  store.set('windowBounds', bounds)
}

export function getLastFolder(): string | null {
  return store.get('lastFolder')
}

export function setLastFolder(folder: string | null): void {
  store.set('lastFolder', folder)
}
