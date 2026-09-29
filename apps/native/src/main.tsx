import { existsSync, mkdtempSync, readdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { render } from '@gpuix/react'
import { readerKey, runCommand } from './actions'
import { App } from './App'
import { commandForKey, type CommandId } from './lib/commands'
import {
  activateApp,
  handleOpenUrls,
  installMenuBar,
  registerFonts,
  type MenuSpec,
} from './lib/macos'
import { defaultStatePath, loadState, saveState } from './lib/persist'
import { IS_MAC, systemColorScheme } from './lib/platform'
import { APP_VERSION } from './lib/updater'
import { FileWatcher, FolderWatcher } from './lib/watcher'
import {
  createAppStore,
  openFolder,
  openPaths,
  persistedState,
  reloadDocument,
  rescanWorkspace,
  restoreTabs,
  setAppStore,
  setSystemScheme,
} from './store'
import { CHECK_INTERVAL_MS, checkForUpdates, LAUNCH_CHECK_DELAY_MS } from './update-flow'

const SAVE_DEBOUNCE_MS = 400
const APPEARANCE_POLL_MS = 3000

const args = process.argv.slice(2)
const smokeTest = args.includes('--smoke-test')

if (args.includes('--version')) {
  console.log(APP_VERSION)
  process.exit(0)
}

// ---------------------------------------------------------------------------
// Assets
// ---------------------------------------------------------------------------

/** `Contents/Resources` in a packaged app, `apps/native/assets` in development. */
function assetsDir(): string | null {
  const candidates = [
    resolve(dirname(process.execPath), '../Resources/assets'),
    resolve(dirname(process.execPath), 'assets'),
    resolve(import.meta.dir, '../assets'),
  ]
  return candidates.find((dir) => existsSync(join(dir, 'fonts'))) ?? null
}

const assets = assetsDir()
if (args.includes('--verify-assets')) {
  console.log(assets ?? 'missing')
  process.exit(assets ? 0 : 1)
}
if (IS_MAC && assets) {
  const fonts = join(assets, 'fonts')
  registerFonts(
    readdirSync(fonts)
      .filter((name) => name.endsWith('.ttf'))
      .map((name) => join(fonts, name)),
  )
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

const statePath = smokeTest
  ? join(mkdtempSync(join(tmpdir(), 'mdow-smoke-')), 'state.json')
  : defaultStatePath()
const initial = loadState(statePath)
// `bun --hot` re-runs this module; keep the live store across saves.
const store = ((globalThis as { __mdowStore?: ReturnType<typeof createAppStore> }).__mdowStore ??=
  createAppStore(initial, systemColorScheme()))
setAppStore(store)

const firstRun = store.getState().tabs.length === 0
if (firstRun) {
  if (initial.session.lastFolder && existsSync(initial.session.lastFolder)) {
    openFolder(initial.session.lastFolder, false)
  }
  const launchPaths = args.filter((arg) => !arg.startsWith('-'))
  restoreTabs(initial.session.tabs, initial.session.activeTab)
  if (launchPaths.length > 0) openPaths(launchPaths.map((path) => resolve(path)))
}

let saveTimer: ReturnType<typeof setTimeout> | null = null
function saveNow() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = null
  saveState(statePath, persistedState(store.getState()))
}

const fileWatcher = new FileWatcher((path) => reloadDocument(path))
const folderWatcher = new FolderWatcher(() => rescanWorkspace())
let watchedFolder: string | null = null

function syncWatchers() {
  const state = store.getState()
  fileWatcher.sync(state.tabs.map((tab) => tab.path))
  const folder = state.workspace?.folder ?? null
  if (folder !== watchedFolder) {
    watchedFolder = folder
    folderWatcher.watch(folder)
  }
}
syncWatchers()

store.subscribe((state, previous) => {
  if (state.tabs !== previous.tabs || state.workspace !== previous.workspace) syncWatchers()
  if (
    state.prefs !== previous.prefs ||
    state.recents !== previous.recents ||
    state.tabs !== previous.tabs ||
    state.activePath !== previous.activePath ||
    state.workspace?.folder !== previous.workspace?.folder ||
    state.window !== previous.window
  ) {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(saveNow, SAVE_DEBOUNCE_MS)
  }
})

if (firstRun)
  setInterval(() => {
    if (store.getState().prefs.theme === 'system') setSystemScheme(systemColorScheme())
  }, APPEARANCE_POLL_MS).unref()

process.on('exit', () => {
  if (!smokeTest) saveNow()
  fileWatcher.close()
  folderWatcher.close()
})
for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.on(signal, () => process.exit(0))
}

function quit() {
  process.exit(0)
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

if (IS_MAC && firstRun) {
  handleOpenUrls((paths) => {
    openPaths(paths)
    activateApp()
  })
}

const window = initial.session.window
render(<App saveNow={saveNow} />, {
  title: 'Mdow',
  appName: 'Mdow',
  appId: 'mdow-native',
  width: window?.width ?? 1120,
  height: window?.height ?? 760,
  minWidth: 480,
  minHeight: 360,
  titlebarTransparent: true,
  trafficLightX: 14,
  trafficLightY: 14,
  focus: process.env.GPUIX_BACKGROUND !== '1',
  onKeyDown(event) {
    const command = commandForKey(event)
    if (command) {
      runCommand(command)
      return
    }
    const overlay = store.getState().overlay
    if (overlay === null && event.key && !hasPrimaryModifier(event.modifiers)) {
      readerKey(event.key, !!event.modifiers?.shift)
    }
  },
})

type MenuModifiers = ('cmd' | 'shift' | 'alt' | 'ctrl')[]

function item(title: string, command: CommandId, key?: string, modifiers?: MenuModifiers) {
  return {
    title,
    run: () => runCommand(command),
    key,
    modifiers: key ? (modifiers ?? ['cmd']) : undefined,
  }
}

function hasPrimaryModifier(modifiers: { cmd: boolean; ctrl: boolean; alt: boolean } | undefined) {
  return !!modifiers && (modifiers.cmd || modifiers.ctrl || modifiers.alt)
}

if (IS_MAC && firstRun) {
  const separator = { title: '', separator: true }
  const menus: MenuSpec[] = [
    {
      title: 'Mdow',
      items: [
        item('Check for Updates…', 'check-for-updates'),
        separator,
        item('Settings…', 'settings', ','),
        separator,
        { title: 'Services', services: true },
        separator,
        { title: 'Hide Mdow', selector: 'hide:', key: 'h', modifiers: ['cmd'] },
        {
          title: 'Hide Others',
          selector: 'hideOtherApplications:',
          key: 'h',
          modifiers: ['cmd', 'alt'],
        },
        { title: 'Show All', selector: 'unhideAllApplications:' },
        separator,
        { title: 'Quit Mdow', run: quit, key: 'q', modifiers: ['cmd'] },
      ],
    },
    {
      title: 'File',
      items: [
        item('Open File…', 'open-file', 'o'),
        item('Open Folder…', 'open-folder', 'o', ['cmd', 'shift']),
        separator,
        item('Close Tab', 'close-tab', 'w'),
      ],
    },
    {
      title: 'Edit',
      // No Copy/Paste items: a menu key equivalent would take ⌘C from text selection and
      // the find field before GPUI sees it.
      items: [
        item('Find…', 'find', 'f'),
        item('Find Next', 'find-next', 'g'),
        item('Find Previous', 'find-previous', 'g', ['cmd', 'shift']),
      ],
    },
    {
      title: 'View',
      items: [
        item('Toggle Sidebar', 'toggle-sidebar', 'b'),
        item('Recents', 'sidebar-recents', '1', ['ctrl']),
        item('Folder', 'sidebar-folder', '2', ['ctrl']),
        item('Outline', 'sidebar-outline', '3', ['ctrl']),
        separator,
        item('Wide Mode', 'toggle-wide-mode', 'w', ['cmd', 'shift']),
        separator,
        item('Zoom In', 'zoom-in', '='),
        item('Zoom Out', 'zoom-out', '-'),
        item('Actual Size', 'zoom-reset', '0'),
        separator,
        item('Command Palette', 'command-palette', 'k'),
        item('Keyboard Shortcuts', 'shortcuts', '/'),
      ],
    },
    {
      title: 'Window',
      windowsMenu: true,
      items: [
        { title: 'Minimize', selector: 'performMiniaturize:', key: 'm', modifiers: ['cmd'] },
        { title: 'Zoom', selector: 'performZoom:' },
        separator,
        item('Show Previous Tab', 'previous-tab', '[', ['cmd', 'shift']),
        item('Show Next Tab', 'next-tab', ']', ['cmd', 'shift']),
      ],
    },
  ]
  installMenuBar(menus)
}

// ---------------------------------------------------------------------------
// Updates and smoke test
// ---------------------------------------------------------------------------

if (smokeTest) {
  setTimeout(() => {
    console.log('MDOW_SMOKE_OK')
    process.exit(0)
  }, 3000)
} else if (firstRun) {
  setTimeout(() => void checkForUpdates({ manual: false }), LAUNCH_CHECK_DELAY_MS)
  setInterval(() => void checkForUpdates({ manual: false }), CHECK_INTERVAL_MS).unref()
}
