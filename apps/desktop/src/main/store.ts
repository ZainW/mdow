import Store from 'electron-store'
import { existsSync } from 'fs'
import type { CompanionProviderId } from '../shared/types'

interface SessionTab {
  path: string
}

interface WindowBounds {
  x: number
  y: number
  width: number
  height: number
  isMaximized?: boolean
}

type SidebarMode = 'recents' | 'folder' | 'outline'
type InterfaceScale = 'compact' | 'comfortable' | 'large'
type ReadingWidth = 'standard' | 'comfortable' | 'wide'

interface StoreSchema {
  recents: string[]
  lastFolder: string | null
  zoomLevel: number
  windowBounds: WindowBounds | null
  sessionTabs: SessionTab[]
  sessionActiveTabPath: string | null
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

const store = new Store<StoreSchema>({
  defaults: {
    recents: [],
    lastFolder: null,
    zoomLevel: 100,
    windowBounds: null,
    sessionTabs: [],
    sessionActiveTabPath: null,
    contentFont: 'inter',
    codeFont: 'geist-mono',
    theme: 'system',
    autoUpdateEnabled: true,
    wideMode: false,
    interfaceScale: 'compact',
    readingWidth: 'standard',
    sidebarMode: 'recents',
    companionProvider: 'auto',
    companionCustomCommand: '',
  },
})

const MAX_RECENTS = 20

function filterExistingRecents(recents: string[]): string[] {
  return recents.filter((path) => existsSync(path))
}

function pruneRecentsList(): string[] {
  const recents = store.get('recents')
  const existing = filterExistingRecents(recents)
  if (existing.length !== recents.length) {
    store.set('recents', existing)
  }
  return existing
}

export function getRecents(): string[] {
  return pruneRecentsList()
}

export function addRecent(filePath: string): void {
  const recents = store.get('recents').filter((r) => r !== filePath)
  recents.unshift(filePath)
  store.set('recents', recents.slice(0, MAX_RECENTS))
}

export function getAppState() {
  return {
    zoomLevel: store.get('zoomLevel'),
    lastFolder: store.get('lastFolder'),
    windowBounds: store.get('windowBounds'),
    sessionTabs: store.get('sessionTabs'),
    sessionActiveTabPath: store.get('sessionActiveTabPath'),
    contentFont: store.get('contentFont'),
    codeFont: store.get('codeFont'),
    theme: store.get('theme'),
    autoUpdateEnabled: store.get('autoUpdateEnabled'),
    wideMode: store.get('wideMode'),
    interfaceScale: store.get('interfaceScale'),
    readingWidth: store.get('readingWidth'),
    sidebarMode: store.get('sidebarMode'),
    companionProvider: store.get('companionProvider'),
    companionCustomCommand: store.get('companionCustomCommand'),
  }
}

export function saveAppState(state: Partial<StoreSchema>): void {
  if (state.zoomLevel !== undefined) store.set('zoomLevel', state.zoomLevel)
  if (state.lastFolder !== undefined) store.set('lastFolder', state.lastFolder)
  if (state.windowBounds !== undefined) store.set('windowBounds', state.windowBounds)
  if (state.recents !== undefined) store.set('recents', state.recents)
  if (state.sessionTabs !== undefined) store.set('sessionTabs', state.sessionTabs)
  if (state.sessionActiveTabPath !== undefined)
    store.set('sessionActiveTabPath', state.sessionActiveTabPath)
  if (state.contentFont !== undefined) store.set('contentFont', state.contentFont)
  if (state.codeFont !== undefined) store.set('codeFont', state.codeFont)
  if (state.theme !== undefined) store.set('theme', state.theme)
  if (state.autoUpdateEnabled !== undefined) store.set('autoUpdateEnabled', state.autoUpdateEnabled)
  if (state.wideMode !== undefined) store.set('wideMode', state.wideMode)
  if (state.interfaceScale !== undefined) store.set('interfaceScale', state.interfaceScale)
  if (state.readingWidth !== undefined) store.set('readingWidth', state.readingWidth)
  if (state.sidebarMode !== undefined) store.set('sidebarMode', state.sidebarMode)
  if (state.companionProvider !== undefined) store.set('companionProvider', state.companionProvider)
  if (state.companionCustomCommand !== undefined)
    store.set('companionCustomCommand', state.companionCustomCommand)
}

export function getWindowBounds(): WindowBounds | null {
  return store.get('windowBounds')
}

export function saveWindowBounds(
  bounds: { x: number; y: number; width: number; height: number },
  isMaximized?: boolean,
) {
  store.set('windowBounds', { ...bounds, isMaximized: isMaximized ?? false })
}

export function getLastFolder(): string | null {
  return store.get('lastFolder')
}

export function setLastFolder(folder: string | null): void {
  store.set('lastFolder', folder)
}

export function isAutoUpdateEnabled(): boolean {
  return store.get('autoUpdateEnabled')
}
