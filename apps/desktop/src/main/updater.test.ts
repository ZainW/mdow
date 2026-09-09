import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { BrowserWindow } from 'electron'

const electronUpdaterHandlers = vi.hoisted(
  () => new Map<string, Array<(...args: unknown[]) => void>>(),
)
const nativeUpdaterHandlers = vi.hoisted(
  () => new Map<string, Array<(...args: unknown[]) => void>>(),
)
const mockQuitAndInstall = vi.hoisted(() => vi.fn())
const mockNativeOn = vi.hoisted(() => vi.fn())

vi.mock('electron-updater', () => ({
  default: {
    autoUpdater: {
      logger: null,
      autoDownload: false,
      autoInstallOnAppQuit: true,
      on: (event: string, handler: (...args: unknown[]) => void) => {
        const list = electronUpdaterHandlers.get(event) ?? []
        list.push(handler)
        electronUpdaterHandlers.set(event, list)
      },
      checkForUpdates: vi.fn(() => Promise.resolve()),
      downloadUpdate: vi.fn(() => Promise.resolve()),
      quitAndInstall: mockQuitAndInstall,
    },
  },
}))

vi.mock('electron', () => ({
  BrowserWindow: vi.fn(),
  autoUpdater: {
    on: (event: string, handler: (...args: unknown[]) => void) => {
      mockNativeOn(event, handler)
      const list = nativeUpdaterHandlers.get(event) ?? []
      list.push(handler)
      nativeUpdaterHandlers.set(event, list)
    },
  },
}))

vi.mock('electron-log', () => ({
  default: {
    error: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
  },
}))

vi.mock('./store', () => ({
  isAutoUpdateEnabled: vi.fn(() => false),
}))

import {
  installUpdate,
  restartReadyFromElectronUpdater,
  scheduleQuitAndInstall,
  shouldAnnounceDownloadedUpdate,
} from './updater'

const originalPlatform = process.platform

function emit(handlers: Map<string, Array<(...args: unknown[]) => void>>, event: string): void {
  for (const handler of handlers.get(event) ?? []) {
    handler()
  }
}

function stubPlatform(platform: NodeJS.Platform): void {
  Object.defineProperty(process, 'platform', { value: platform, configurable: true })
}

async function loadUpdaterForPlatform(platform: NodeJS.Platform) {
  vi.resetModules()
  electronUpdaterHandlers.clear()
  nativeUpdaterHandlers.clear()
  mockNativeOn.mockClear()
  stubPlatform(platform)
  return import('./updater')
}

function createWindow() {
  const send = vi.fn()
  const win = {
    isDestroyed: () => false,
    webContents: { send },
  } as unknown as BrowserWindow
  return { send, win }
}

describe('restartReadyFromElectronUpdater', () => {
  it('is false on darwin and true on win32 and linux', () => {
    expect(restartReadyFromElectronUpdater('darwin')).toBe(false)
    expect(restartReadyFromElectronUpdater('win32')).toBe(true)
    expect(restartReadyFromElectronUpdater('linux')).toBe(true)
  })
})

describe('shouldAnnounceDownloadedUpdate', () => {
  it('announces electron-updater only when restart is ready from that source', () => {
    expect(shouldAnnounceDownloadedUpdate('darwin', 'electron-updater')).toBe(false)
    expect(shouldAnnounceDownloadedUpdate('win32', 'electron-updater')).toBe(true)
    expect(shouldAnnounceDownloadedUpdate('linux', 'electron-updater')).toBe(true)
  })

  it('announces native only on darwin', () => {
    expect(shouldAnnounceDownloadedUpdate('darwin', 'native')).toBe(true)
    expect(shouldAnnounceDownloadedUpdate('win32', 'native')).toBe(false)
    expect(shouldAnnounceDownloadedUpdate('linux', 'native')).toBe(false)
  })
})

describe('scheduleQuitAndInstall', () => {
  it('defers quitAndInstall and calls it with (false, true)', async () => {
    const quitAndInstall = vi.fn()
    scheduleQuitAndInstall(quitAndInstall)
    expect(quitAndInstall).not.toHaveBeenCalled()
    await new Promise<void>((resolve) => setImmediate(resolve))
    expect(quitAndInstall).toHaveBeenCalledWith(false, true)
  })
})

describe('banner announce path', () => {
  afterEach(() => {
    stubPlatform(originalPlatform)
  })

  it('does not announce electron-updater update-downloaded on darwin', async () => {
    const { initAutoUpdater } = await loadUpdaterForPlatform('darwin')
    const { send, win } = createWindow()
    initAutoUpdater(() => win)

    emit(electronUpdaterHandlers, 'update-downloaded')

    expect(send).not.toHaveBeenCalled()
  })

  it('announces native update-downloaded on darwin', async () => {
    const { initAutoUpdater } = await loadUpdaterForPlatform('darwin')
    const { send, win } = createWindow()
    initAutoUpdater(() => win)

    expect(mockNativeOn).toHaveBeenCalledWith('update-downloaded', expect.any(Function))
    emit(nativeUpdaterHandlers, 'update-downloaded')

    expect(send).toHaveBeenCalledWith('updater:update-downloaded')
  })

  it('announces electron-updater update-downloaded on win32 and skips native', async () => {
    const { initAutoUpdater } = await loadUpdaterForPlatform('win32')
    const { send, win } = createWindow()
    initAutoUpdater(() => win)

    expect(mockNativeOn).not.toHaveBeenCalled()
    emit(electronUpdaterHandlers, 'update-downloaded')

    expect(send).toHaveBeenCalledWith('updater:update-downloaded')
  })

  it('announces electron-updater update-downloaded on linux', async () => {
    const { initAutoUpdater } = await loadUpdaterForPlatform('linux')
    const { send, win } = createWindow()
    initAutoUpdater(() => win)

    expect(mockNativeOn).not.toHaveBeenCalled()
    emit(electronUpdaterHandlers, 'update-downloaded')

    expect(send).toHaveBeenCalledWith('updater:update-downloaded')
  })
})

describe('installUpdate', () => {
  beforeEach(() => {
    mockQuitAndInstall.mockClear()
  })

  it('uses deferred quitAndInstall with (false, true)', async () => {
    installUpdate()
    expect(mockQuitAndInstall).not.toHaveBeenCalled()
    await new Promise<void>((resolve) => setImmediate(resolve))
    expect(mockQuitAndInstall).toHaveBeenCalledWith(false, true)
  })
})
