import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { isElectronPreload, isNativeApiInstalled, isTauri, TAURI_INTERNALS_KEY } from './is-tauri'
import { installNativeApi } from './native-api'

const mockApi = { platform: 'darwin' as const }

vi.mock('./tauri-api', () => ({
  createTauriApi: vi.fn(() => Promise.resolve(mockApi)),
}))

function setWindowApi(value: unknown): void {
  Object.defineProperty(window, 'api', {
    value,
    configurable: true,
    writable: true,
  })
}

function restoreWindowApi(original: unknown): void {
  if (original === undefined) {
    Reflect.deleteProperty(window, 'api')
  } else {
    setWindowApi(original)
  }
}

describe('isTauri', () => {
  afterEach(() => {
    Reflect.deleteProperty(globalThis, TAURI_INTERNALS_KEY)
  })

  it('returns false in jsdom without Tauri globals', () => {
    expect(isTauri()).toBe(false)
  })

  it('returns true when __TAURI_INTERNALS__ is present', () => {
    Reflect.set(globalThis, TAURI_INTERNALS_KEY, {})
    expect(isTauri()).toBe(true)
  })
})

describe('isElectronPreload', () => {
  let originalApi: unknown

  beforeEach(() => {
    originalApi = window.api
  })

  afterEach(() => {
    restoreWindowApi(originalApi)
  })

  it('returns false when window.api is missing', () => {
    Reflect.deleteProperty(window, 'api')
    expect(isElectronPreload()).toBe(false)
    expect(isNativeApiInstalled()).toBe(false)
  })

  it('returns true when window.api is already set', () => {
    setWindowApi(mockApi)
    expect(isElectronPreload()).toBe(true)
    expect(isNativeApiInstalled()).toBe(true)
  })
})

describe('installNativeApi', () => {
  let originalApi: unknown

  beforeEach(() => {
    originalApi = window.api
    Reflect.deleteProperty(globalThis, TAURI_INTERNALS_KEY)
    vi.clearAllMocks()
  })

  afterEach(() => {
    Reflect.deleteProperty(globalThis, TAURI_INTERNALS_KEY)
    restoreWindowApi(originalApi)
  })

  it('no-ops in jsdom without Tauri globals', async () => {
    Reflect.deleteProperty(window, 'api')
    await installNativeApi()
    expect(window.api).toBeUndefined()
  })

  it('no-ops when Electron preload already set window.api', async () => {
    const existing = { platform: 'darwin' as const }
    setWindowApi(existing)
    await installNativeApi()
    expect(window.api).toBe(existing)
  })

  it('installs the Tauri bridge when running under Tauri', async () => {
    Reflect.deleteProperty(window, 'api')
    Reflect.set(globalThis, TAURI_INTERNALS_KEY, {})

    const { createTauriApi } = await import('./tauri-api')
    await installNativeApi()

    expect(createTauriApi).toHaveBeenCalledOnce()
    expect(window.api).toBe(mockApi)
  })
})
