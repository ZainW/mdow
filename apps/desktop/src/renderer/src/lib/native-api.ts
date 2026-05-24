import type { ElectronAPI } from '../../../shared/api-types'
import { isElectronPreload, isTauri } from './is-tauri'
import { createTauriApi } from './tauri-api'

function assignWindowApi(api: ElectronAPI): void {
  ;(window as Window & { api: ElectronAPI }).api = api
}

/** Install window.api from Electron preload (no-op) or Tauri bridge. Safe in jsdom. */
export async function installNativeApi(): Promise<void> {
  if (typeof window === 'undefined') return
  if (isElectronPreload()) return
  if (!isTauri()) return

  assignWindowApi(await createTauriApi())
}
