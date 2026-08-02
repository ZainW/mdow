import Store from 'electron-store'
import { existsSync } from 'fs'

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
type CompanionProviderId = 'opencode' | 'codex-acp' | 'custom'

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
  companionPreferredProvider: CompanionProviderId | null
  companionCustomCommand: string
  companionLastModel: string | null
}

const storeDefaults: StoreSchema = {
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
  companionPreferredProvider: null,
  companionCustomCommand: '',
  companionLastModel: null,
}

let store: Store<StoreSchema> | null = null

function getStore(): Store<StoreSchema> {
  store ??= new Store<StoreSchema>({ defaults: storeDefaults })
  return store
}

const MAX_RECENTS = 20

function filterExistingRecents(recents: string[]): string[] {
  return recents.filter((path) => existsSync(path))
}

function pruneRecentsList(): string[] {
  const recents = getStore().get('recents')
  const existing = filterExistingRecents(recents)
  if (existing.length !== recents.length) {
    getStore().set('recents', existing)
  }
  return existing
}

export function getRecents(): string[] {
  return pruneRecentsList()
}

export function addRecent(filePath: string): void {
  const recents = getStore()
    .get('recents')
    .filter((r) => r !== filePath)
  recents.unshift(filePath)
  getStore().set('recents', recents.slice(0, MAX_RECENTS))
}

export function getAppState() {
  const appStore = getStore()
  return {
    zoomLevel: appStore.get('zoomLevel'),
    lastFolder: appStore.get('lastFolder'),
    windowBounds: appStore.get('windowBounds'),
    sessionTabs: appStore.get('sessionTabs'),
    sessionActiveTabPath: appStore.get('sessionActiveTabPath'),
    contentFont: appStore.get('contentFont'),
    codeFont: appStore.get('codeFont'),
    theme: appStore.get('theme'),
    autoUpdateEnabled: appStore.get('autoUpdateEnabled'),
    wideMode: appStore.get('wideMode'),
    interfaceScale: appStore.get('interfaceScale'),
    readingWidth: appStore.get('readingWidth'),
    sidebarMode: appStore.get('sidebarMode'),
    companionPreferredProvider: appStore.get('companionPreferredProvider'),
    companionCustomCommand: appStore.get('companionCustomCommand'),
    companionLastModel: appStore.get('companionLastModel'),
  }
}

export function saveAppState(state: Partial<StoreSchema>): void {
  const appStore = getStore()
  if (state.zoomLevel !== undefined) appStore.set('zoomLevel', state.zoomLevel)
  if (state.lastFolder !== undefined) appStore.set('lastFolder', state.lastFolder)
  if (state.windowBounds !== undefined) appStore.set('windowBounds', state.windowBounds)
  if (state.recents !== undefined) appStore.set('recents', state.recents)
  if (state.sessionTabs !== undefined) appStore.set('sessionTabs', state.sessionTabs)
  if (state.sessionActiveTabPath !== undefined)
    appStore.set('sessionActiveTabPath', state.sessionActiveTabPath)
  if (state.contentFont !== undefined) appStore.set('contentFont', state.contentFont)
  if (state.codeFont !== undefined) appStore.set('codeFont', state.codeFont)
  if (state.theme !== undefined) appStore.set('theme', state.theme)
  if (state.autoUpdateEnabled !== undefined)
    appStore.set('autoUpdateEnabled', state.autoUpdateEnabled)
  if (state.wideMode !== undefined) appStore.set('wideMode', state.wideMode)
  if (state.interfaceScale !== undefined) appStore.set('interfaceScale', state.interfaceScale)
  if (state.readingWidth !== undefined) appStore.set('readingWidth', state.readingWidth)
  if (state.sidebarMode !== undefined) appStore.set('sidebarMode', state.sidebarMode)
}

export function getCompanionSettings(): {
  preferredProvider: CompanionProviderId | null
  customCommand: string
  lastModel: string | null
} {
  const appStore = getStore()
  return {
    preferredProvider: appStore.get('companionPreferredProvider'),
    customCommand: appStore.get('companionCustomCommand'),
    lastModel: appStore.get('companionLastModel'),
  }
}

export function saveCompanionSettings(settings: {
  preferredProvider?: CompanionProviderId | null
  customCommand?: string
  lastModel?: string | null
}): void {
  const appStore = getStore()
  if (settings.preferredProvider !== undefined) {
    appStore.set('companionPreferredProvider', settings.preferredProvider)
  }
  if (settings.customCommand !== undefined) {
    appStore.set('companionCustomCommand', settings.customCommand)
  }
  if (settings.lastModel !== undefined) {
    appStore.set('companionLastModel', settings.lastModel)
  }
}

export function getWindowBounds(): WindowBounds | null {
  return getStore().get('windowBounds')
}

export function saveWindowBounds(
  bounds: { x: number; y: number; width: number; height: number },
  isMaximized?: boolean,
) {
  getStore().set('windowBounds', { ...bounds, isMaximized: isMaximized ?? false })
}

export function getLastFolder(): string | null {
  return getStore().get('lastFolder')
}

export function setLastFolder(folder: string | null): void {
  getStore().set('lastFolder', folder)
}

export function isAutoUpdateEnabled(): boolean {
  return getStore().get('autoUpdateEnabled')
}
