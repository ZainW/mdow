import pkg from 'electron-updater'
import { BrowserWindow } from 'electron'
import log from 'electron-log'

const { autoUpdater } = pkg

autoUpdater.logger = log
autoUpdater.autoDownload = false
autoUpdater.autoInstallOnAppQuit = true

export function initAutoUpdater(getMainWindow: () => BrowserWindow | null): void {
  const send = (channel: string, ...args: unknown[]) => {
    const win = getMainWindow()
    if (win && !win.isDestroyed()) {
      win.webContents.send(channel, ...args)
    }
  }

  autoUpdater.on('update-available', (info) => {
    send('updater:update-available', {
      version: info.version,
      releaseNotes: info.releaseNotes,
    })
  })

  autoUpdater.on('update-not-available', () => {
    send('updater:up-to-date')
  })

  autoUpdater.on('download-progress', (progress) => {
    send('updater:download-progress', {
      percent: Math.round(progress.percent),
    })
  })

  autoUpdater.on('update-downloaded', () => {
    send('updater:update-downloaded')
  })

  autoUpdater.on('error', (err) => {
    log.error('Auto-updater error:', err)
    send('updater:error', err.message)
  })

  // Check on launch after a short delay
  setTimeout(() => {
    void autoUpdater.checkForUpdates().catch(() => {
      // Network error or no releases — ignore
    })
  }, 5_000)
}

export function checkForUpdates(): void {
  void autoUpdater.checkForUpdates().catch(() => {})
}

export function downloadUpdate(): void {
  void autoUpdater.downloadUpdate().catch(() => {})
}

export function installUpdate(): void {
  autoUpdater.quitAndInstall()
}
