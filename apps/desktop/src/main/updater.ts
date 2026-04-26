import pkg from 'electron-updater'
import { BrowserWindow } from 'electron'
import log from 'electron-log'
import { isAutoUpdateEnabled } from './store'
import { isMac } from './platform'

const { autoUpdater } = pkg

autoUpdater.logger = log
autoUpdater.autoDownload = false
autoUpdater.autoInstallOnAppQuit = true

const STARTUP_DELAY_MS = 30_000
const RECHECK_INTERVAL_MS = 4 * 60 * 60 * 1000

let manualCheckPending = false
let intervalHandle: NodeJS.Timeout | null = null
let startupHandle: NodeJS.Timeout | null = null

export function initAutoUpdater(getMainWindow: () => BrowserWindow | null): void {
  if (isMac) {
    // Squirrel.Mac requires a signed build to apply updates. Until we have an
    // Apple Developer ID, a half-working updater is worse than an honest link
    // out to the releases page (handled by the menu).
    return
  }

  const send = (channel: string, ...args: unknown[]) => {
    const win = getMainWindow()
    if (win && !win.isDestroyed()) {
      win.webContents.send(channel, ...args)
    }
  }

  autoUpdater.on('update-available', (info) => {
    manualCheckPending = false
    send('updater:update-available', {
      version: info.version,
      releaseNotes: info.releaseNotes,
    })
  })

  autoUpdater.on('update-not-available', () => {
    const wasManual = manualCheckPending
    manualCheckPending = false
    send('updater:up-to-date', { wasManual })
  })

  autoUpdater.on('download-progress', (progress) => {
    send('updater:download-progress', { percent: Math.round(progress.percent) })
  })

  autoUpdater.on('update-downloaded', () => {
    send('updater:update-downloaded')
  })

  autoUpdater.on('error', (err) => {
    log.error('Auto-updater error:', err)
    // Background-check errors stay silent. Manual checks get UI feedback so
    // the user isn't left wondering whether their click did anything.
    if (manualCheckPending) {
      manualCheckPending = false
      send('updater:error', err.message)
    }
  })

  scheduleAutoChecks()
}

function scheduleAutoChecks(): void {
  if (intervalHandle) return
  if (!isAutoUpdateEnabled()) return
  startupHandle = setTimeout(() => {
    startupHandle = null
    void autoUpdater.checkForUpdates().catch(() => {})
  }, STARTUP_DELAY_MS)
  intervalHandle = setInterval(() => {
    void autoUpdater.checkForUpdates().catch(() => {})
  }, RECHECK_INTERVAL_MS)
}

export function checkForUpdates(opts?: { manual?: boolean }): void {
  if (isMac) return
  if (opts?.manual) manualCheckPending = true
  void autoUpdater.checkForUpdates().catch(() => {})
}

export function downloadUpdate(): void {
  if (isMac) return
  void autoUpdater.downloadUpdate().catch(() => {})
}

export function installUpdate(): void {
  if (isMac) return
  autoUpdater.quitAndInstall()
}

export function setAutoUpdateScheduling(enabled: boolean): void {
  if (isMac) return
  if (enabled && !intervalHandle) {
    scheduleAutoChecks()
  } else if (!enabled) {
    if (intervalHandle) {
      clearInterval(intervalHandle)
      intervalHandle = null
    }
    if (startupHandle) {
      clearTimeout(startupHandle)
      startupHandle = null
    }
  }
}
