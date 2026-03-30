import Store from 'electron-store'

interface StoreSchema {
  recents: string[]
  lastFolder: string | null
  sidebarWidth: number
  windowBounds: { x: number; y: number; width: number; height: number } | null
}

const store = new Store<StoreSchema>({
  defaults: {
    recents: [],
    lastFolder: null,
    sidebarWidth: 260,
    windowBounds: null,
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
    lastFolder: store.get('lastFolder'),
    windowBounds: store.get('windowBounds'),
  }
}

export function saveAppState(state: Partial<StoreSchema>): void {
  if (state.sidebarWidth !== undefined) store.set('sidebarWidth', state.sidebarWidth)
  if (state.lastFolder !== undefined) store.set('lastFolder', state.lastFolder)
  if (state.windowBounds !== undefined) store.set('windowBounds', state.windowBounds)
  if (state.recents !== undefined) store.set('recents', state.recents)
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
