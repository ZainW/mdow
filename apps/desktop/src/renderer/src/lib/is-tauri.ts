/** Tauri v2 injects this global before the renderer bundle runs. */
export const TAURI_INTERNALS_KEY = '__TAURI_INTERNALS__'

export function isTauri(): boolean {
  if (typeof globalThis === 'undefined') return false
  return TAURI_INTERNALS_KEY in globalThis
}

export function isElectronPreload(): boolean {
  return typeof window !== 'undefined' && window.api !== undefined
}

export function isNativeApiInstalled(): boolean {
  return isElectronPreload()
}
