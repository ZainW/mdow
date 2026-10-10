import { existsSync, statSync } from 'node:fs'
import { dirname, isAbsolute, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { NativeRenderer } from '@gpuix/react'
import type { CommandId } from './lib/commands'
import { isSupportedDocument } from './lib/documents'
import { openExternal } from './lib/platform'
import {
  appStore,
  closeTab,
  cycleTab,
  openFolder,
  openDocumentAsync,
  openPathsAsync,
  setOverlay,
  setPrefs,
  showSidebar,
  toggleOverlay,
  toggleSidebar,
  zoomBy,
} from './store'
import { sendReader } from './ui/reader-bus'
import { checkForUpdates } from './update-flow'

let renderer: NativeRenderer | null = null

export function setRenderer(next: NativeRenderer | null) {
  renderer = next
}

export async function promptOpenFile() {
  const paths = await renderer?.promptForPaths?.({ files: true, multiple: true, prompt: 'Open' })
  if (paths) openPathsAsync(paths)
}

export async function promptOpenFolder() {
  const paths = await renderer?.promptForPaths?.({
    files: false,
    directories: true,
    prompt: 'Open Folder',
  })
  if (paths?.[0]) openFolder(paths[0])
}

export function runCommand(command: CommandId) {
  const state = appStore().getState()
  switch (command) {
    case 'open-file':
      return void promptOpenFile()
    case 'open-folder':
      return void promptOpenFolder()
    case 'close-tab':
      return closeTab()
    case 'next-tab':
      return cycleTab(1)
    case 'previous-tab':
      return cycleTab(-1)
    case 'toggle-sidebar':
      return toggleSidebar()
    case 'sidebar-recents':
      return showSidebar('recents')
    case 'sidebar-folder':
      return showSidebar('folder')
    case 'sidebar-outline':
      return showSidebar('outline')
    case 'toggle-wide-mode':
      return setPrefs({ wideMode: !state.prefs.wideMode })
    case 'column-standard':
      return setPrefs({ readingWidth: 'standard', wideMode: false })
    case 'column-comfortable':
      return setPrefs({ readingWidth: 'comfortable', wideMode: false })
    case 'column-wide':
      return setPrefs({ readingWidth: 'wide', wideMode: false })
    case 'theme-system':
      return setPrefs({ theme: 'system' })
    case 'theme-light':
      return setPrefs({ theme: 'light' })
    case 'theme-dark':
      return setPrefs({ theme: 'dark' })
    case 'zoom-in':
      return zoomBy(1)
    case 'zoom-out':
      return zoomBy(-1)
    case 'zoom-reset':
      return setPrefs({ zoomLevel: 100 })
    case 'find':
      return state.activePath ? setOverlay('find') : undefined
    case 'find-next':
      if (state.overlay !== 'find') return state.activePath ? setOverlay('find') : undefined
      return sendReader({ type: 'find', direction: 1 })
    case 'find-previous':
      if (state.overlay !== 'find') return state.activePath ? setOverlay('find') : undefined
      return sendReader({ type: 'find', direction: -1 })
    case 'command-palette':
      return toggleOverlay('palette')
    case 'settings':
      return toggleOverlay('settings')
    case 'shortcuts':
      return toggleOverlay('shortcuts')
    case 'check-for-updates':
      return void checkForUpdates({ manual: true })
    case 'dismiss':
      return setOverlay(null)
  }
}

/** Reader navigation keys, handled only when no text field owns the keyboard. */
export function readerKey(key: string, shift: boolean): boolean {
  switch (key) {
    case 'home':
      sendReader({ type: 'scroll', to: 'top' })
      return true
    case 'end':
      sendReader({ type: 'scroll', to: 'bottom' })
      return true
    case 'pageup':
      sendReader({ type: 'page', direction: -1 })
      return true
    case 'pagedown':
      sendReader({ type: 'page', direction: 1 })
      return true
    case 'space':
      sendReader({ type: 'page', direction: shift ? -1 : 1 })
      return true
    case 'up':
      sendReader({ type: 'line', direction: -1 })
      return true
    case 'down':
      sendReader({ type: 'line', direction: 1 })
      return true
    default:
      return false
  }
}

/**
 * Route a link from the reader: `#anchors` scroll, web links open in the browser, supported
 * documents open as tabs, and any other local file opens in its default app.
 */
export function followLink(href: string, documentPath: string) {
  const target = href.trim()
  if (!target) return
  if (target.startsWith('#')) {
    sendReader({ type: 'slug', slug: decodeFragment(target.slice(1)) })
    return
  }
  if (/^[a-z][a-z0-9+.-]*:/i.test(target) && !target.startsWith('file:')) {
    void openExternal(target)
    return
  }
  const [rawPath, fragment] = splitFragment(target)
  let path: string
  try {
    path = rawPath.startsWith('file:') ? fileURLToPath(rawPath) : decodeURI(rawPath)
  } catch {
    path = rawPath
  }
  const absolute = isAbsolute(path) ? path : resolve(dirname(documentPath), path)
  if (!existsSync(absolute)) return
  let isDirectory = false
  try {
    isDirectory = statSync(absolute).isDirectory()
  } catch {}
  if (!isDirectory && isSupportedDocument(absolute)) {
    void openDocumentAsync(absolute).then((document) => {
      if (fragment && document.ok)
        setTimeout(() => sendReader({ type: 'slug', slug: decodeFragment(fragment) }), 50)
    })
  } else {
    void openExternal(absolute)
  }
}

function splitFragment(target: string): [string, string | null] {
  const index = target.indexOf('#')
  return index === -1 ? [target, null] : [target.slice(0, index), target.slice(index + 1)]
}

function decodeFragment(fragment: string) {
  try {
    return decodeURIComponent(fragment).toLowerCase()
  } catch {
    return fragment.toLowerCase()
  }
}
