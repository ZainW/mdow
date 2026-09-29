import { mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import {
  CODE_FONTS,
  COLUMN_WIDTHS,
  CONTENT_FONTS,
  DEFAULT_PREFS,
  INTERFACE_SCALES,
  SIDEBAR_MODES,
  THEME_MODES,
  clampZoom,
  type Prefs,
} from './prefs'

export const MAX_RECENTS = 20

export interface WindowBounds {
  width: number
  height: number
}

export interface Session {
  recents: string[]
  lastFolder: string | null
  tabs: string[]
  activeTab: string | null
  window: WindowBounds | null
}

export interface PersistedState {
  prefs: Prefs
  session: Session
}

export const EMPTY_SESSION: Session = {
  recents: [],
  lastFolder: null,
  tabs: [],
  activeTab: null,
  window: null,
}

/**
 * Same file and camelCase wire format the Rust build wrote, so upgrading keeps settings,
 * recents and open tabs. Only width/height of `windowBounds` are honored: gpuix cannot place
 * a window yet.
 */
export function defaultStatePath() {
  if (process.env.MDOW_STATE_PATH) return process.env.MDOW_STATE_PATH
  if (process.platform === 'darwin') {
    return join(homedir(), 'Library/Application Support/Mdow Native/state.json')
  }
  const config = process.env.XDG_CONFIG_HOME || join(homedir(), '.config')
  return join(config, 'mdow-native', 'state.json')
}

export function loadState(path: string): PersistedState {
  let raw: unknown
  try {
    raw = JSON.parse(readFileSync(path, 'utf8'))
  } catch {
    return { prefs: { ...DEFAULT_PREFS }, session: { ...EMPTY_SESSION } }
  }
  return decodeState(raw)
}

function pick<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
  return typeof value === 'string' && (allowed as readonly string[]).includes(value)
    ? (value as T)
    : fallback
}

function strings(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string')
    : []
}

function string(value: unknown): string | null {
  return typeof value === 'string' && value ? value : null
}

/** Each field decodes on its own, so one bad value never discards the rest. */
export function decodeState(raw: unknown): PersistedState {
  const wire = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>
  const prefs: Prefs = {
    theme: pick(wire.theme, THEME_MODES, DEFAULT_PREFS.theme),
    contentFont: pick(wire.contentFont, CONTENT_FONTS, DEFAULT_PREFS.contentFont),
    codeFont: pick(wire.codeFont, CODE_FONTS, DEFAULT_PREFS.codeFont),
    interfaceScale: pick(wire.interfaceScale, INTERFACE_SCALES, DEFAULT_PREFS.interfaceScale),
    readingWidth: pick(wire.readingWidth, COLUMN_WIDTHS, DEFAULT_PREFS.readingWidth),
    wideMode: typeof wire.wideMode === 'boolean' ? wire.wideMode : DEFAULT_PREFS.wideMode,
    zoomLevel:
      typeof wire.zoomLevel === 'number' && Number.isFinite(wire.zoomLevel)
        ? clampZoom(wire.zoomLevel)
        : DEFAULT_PREFS.zoomLevel,
    sidebarMode: pick(wire.sidebarMode, SIDEBAR_MODES, DEFAULT_PREFS.sidebarMode),
  }

  const tabs = Array.isArray(wire.sessionTabs)
    ? wire.sessionTabs
        .map((tab) =>
          tab && typeof tab === 'object' ? string((tab as { path?: unknown }).path) : null,
        )
        .filter((path): path is string => path !== null)
    : []
  const active = string(wire.sessionActiveTabPath)
  const bounds = wire.windowBounds as Record<string, unknown> | null | undefined
  const window =
    bounds &&
    typeof bounds.width === 'number' &&
    typeof bounds.height === 'number' &&
    bounds.width >= 320 &&
    bounds.height >= 240
      ? { width: Math.round(bounds.width), height: Math.round(bounds.height) }
      : null

  return {
    prefs,
    session: {
      recents: [...new Set(strings(wire.recents))].slice(0, MAX_RECENTS),
      lastFolder: string(wire.lastFolder),
      tabs,
      activeTab: active && tabs.includes(active) ? active : (tabs[0] ?? null),
      window,
    },
  }
}

export function encodeState({ prefs, session }: PersistedState) {
  return {
    theme: prefs.theme,
    contentFont: prefs.contentFont,
    codeFont: prefs.codeFont,
    interfaceScale: prefs.interfaceScale,
    readingWidth: prefs.readingWidth,
    wideMode: prefs.wideMode,
    zoomLevel: prefs.zoomLevel,
    sidebarMode: prefs.sidebarMode,
    recents: session.recents,
    lastFolder: session.lastFolder,
    sessionTabs: session.tabs.map((path) => ({ path })),
    sessionActiveTabPath: session.activeTab,
    windowBounds: session.window ? { x: 0, y: 0, ...session.window } : null,
  }
}

export function saveState(path: string, state: PersistedState) {
  try {
    mkdirSync(dirname(path), { recursive: true })
    const tmp = path.replace(/\.json$/, '') + '.json.tmp'
    writeFileSync(tmp, JSON.stringify(encodeState(state), null, 2))
    renameSync(tmp, path)
  } catch (error) {
    console.error('mdow: could not save state', error)
  }
}

export function pushRecent(recents: string[], path: string) {
  return [path, ...recents.filter((item) => item !== path)].slice(0, MAX_RECENTS)
}
