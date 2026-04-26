/// <reference types="vite/client" />

declare module '*.css'

interface File {
  /** Electron-specific: the file's absolute path on disk */
  readonly path: string
}

type UpdaterUnsubscribe = () => void

// Updater-related contract surfaced by this PR. The full window.api surface
// is typed via `ElectronAPI` in apps/desktop/src/preload/env.d.ts — this
// interface is kept as a documented reference for the updater methods rather
// than a competing Window.api declaration (which would conflict with the
// preload-driven typing).
interface MdowApi {
  checkForUpdates: (opts?: { manual?: boolean }) => Promise<void>
  downloadUpdate: () => Promise<void>
  installUpdate: () => Promise<void>
  setAutoUpdateScheduling: (enabled: boolean) => Promise<void>
  onUpdateAvailable: (
    callback: (info: { version: string; releaseNotes?: string }) => void,
  ) => UpdaterUnsubscribe
  onUpdateUpToDate: (callback: (info: { wasManual: boolean }) => void) => UpdaterUnsubscribe
  onUpdateDownloadProgress: (
    callback: (progress: { percent: number }) => void,
  ) => UpdaterUnsubscribe
  onUpdateDownloaded: (callback: () => void) => UpdaterUnsubscribe
  onUpdateError: (callback: (message: string) => void) => UpdaterUnsubscribe
  onMenuCheckForUpdates: (callback: () => void) => UpdaterUnsubscribe
}
