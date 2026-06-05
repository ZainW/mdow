import { describe, expect, it, vi, beforeEach } from 'vitest'
import type { BrowserWindow } from 'electron'

const handlers = vi.hoisted(() => new Map<string, (...args: unknown[]) => unknown>())

const mockReadFileContent = vi.hoisted(() => vi.fn())
const mockUnwatchFile = vi.hoisted(() => vi.fn())
const mockSetActiveFileWatch = vi.hoisted(() => vi.fn())
const mockSaveAppState = vi.hoisted(() => vi.fn())
const mockCompanionService = vi.hoisted(() => ({
  detectProviders: vi.fn(),
  send: vi.fn(),
  cancel: vi.fn(),
  shutdown: vi.fn(),
}))
const mockCompanionOptions = vi.hoisted(
  () =>
    ({ current: null }) as {
      current: { emitUpdate: (update: unknown) => void } | null
    },
)
const mockCreateDefaultCompanionService = vi.hoisted(() =>
  vi.fn((options: { emitUpdate: (update: unknown) => void }) => {
    mockCompanionOptions.current = options
    return mockCompanionService
  }),
)
const mockNativeTheme = vi.hoisted(() => ({ themeSource: 'system' as string }))
const mockApplyWindowChrome = vi.hoisted(() => vi.fn())
const mockGetMainWindow = vi.hoisted(() =>
  vi.fn(() => ({
    isDestroyed: () => false,
    webContents: { send: vi.fn() },
  })),
)

vi.mock('electron', () => ({
  ipcMain: {
    handle: (channel: string, handler: (...args: unknown[]) => unknown) => {
      handlers.set(channel, handler)
    },
  },
  shell: { showItemInFolder: vi.fn(), openExternal: vi.fn() },
  BrowserWindow: Object.assign(vi.fn(), {
    fromWebContents: vi.fn(() => mockGetMainWindow()),
    getAllWindows: vi.fn(() => [mockGetMainWindow()]),
  }),
  nativeTheme: mockNativeTheme,
  app: { addRecentDocument: vi.fn() },
}))

vi.mock('./file-service', () => ({
  openFileDialog: vi.fn(),
  readFileContent: mockReadFileContent,
  watchFile: vi.fn(),
  unwatchFile: mockUnwatchFile,
  setActiveFileWatch: mockSetActiveFileWatch,
}))

vi.mock('./folder-service', () => ({
  openFolderDialog: vi.fn(),
  scanFolder: vi.fn(),
  watchFolder: vi.fn(),
}))

vi.mock('./store', () => ({
  getRecents: vi.fn(() => []),
  addRecent: vi.fn(),
  getAppState: vi.fn(),
  saveAppState: mockSaveAppState,
  setLastFolder: vi.fn(),
}))

vi.mock('./updater', () => ({
  checkForUpdates: vi.fn(),
  downloadUpdate: vi.fn(),
  installUpdate: vi.fn(),
  setAutoUpdateScheduling: vi.fn(),
}))

vi.mock('./platform', () => ({ isMac: false }))
vi.mock('./window-chrome', () => ({ applyWindowChrome: mockApplyWindowChrome }))
vi.mock('./allowed-paths', () => ({
  registerAllowedFile: vi.fn(),
  registerAllowedPath: vi.fn(),
}))
vi.mock('./menu', () => ({ rebuildMenu: vi.fn() }))
vi.mock('./companion/service', () => ({
  createDefaultCompanionService: mockCreateDefaultCompanionService,
}))

import { registerIpcHandlers } from './ipc'

describe('ipc handlers', () => {
  beforeEach(() => {
    handlers.clear()
    vi.clearAllMocks()
    mockCompanionOptions.current = null
    mockCompanionService.detectProviders.mockResolvedValue([])
    mockCompanionService.send.mockResolvedValue(undefined)
    mockNativeTheme.themeSource = 'system'
    registerIpcHandlers(mockGetMainWindow as unknown as () => BrowserWindow | null)
  })

  describe('file:read', () => {
    it('maps ENOENT to not-found', async () => {
      const err = Object.assign(new Error('ENOENT'), { code: 'ENOENT' })
      mockReadFileContent.mockRejectedValue(err)
      const handler = handlers.get('file:read')!
      await expect(handler({}, '/missing.md')).rejects.toMatchObject({ message: 'not-found' })
    })

    it('maps EACCES to permission-denied', async () => {
      const err = Object.assign(new Error('EACCES'), { code: 'EACCES' })
      mockReadFileContent.mockRejectedValue(err)
      const handler = handlers.get('file:read')!
      await expect(handler({}, '/secret.md')).rejects.toMatchObject({
        message: 'permission-denied',
      })
    })

    it('maps other errors to read-error', async () => {
      mockReadFileContent.mockRejectedValue(new Error('boom'))
      const handler = handlers.get('file:read')!
      await expect(handler({}, '/bad.md')).rejects.toMatchObject({ message: 'read-error' })
    })

    it('returns content on success', async () => {
      mockReadFileContent.mockResolvedValue('# Hello')
      const handler = handlers.get('file:read')!
      await expect(handler({}, '/readme.md')).resolves.toBe('# Hello')
    })

    it('rejects path traversal', async () => {
      const handler = handlers.get('file:read')!
      await expect(handler({}, '../etc/passwd.md')).rejects.toMatchObject({
        message: 'invalid-path',
      })
    })

    it('rejects non-markdown extensions', async () => {
      const handler = handlers.get('file:read')!
      await expect(handler({}, '/readme.txt')).rejects.toMatchObject({
        message: 'invalid-extension',
      })
    })
  })

  describe('shell:open-external', () => {
    it('allows http and https URLs', async () => {
      const handler = handlers.get('shell:open-external')!
      await expect(Promise.resolve(handler({}, 'https://example.com'))).resolves.toBe(true)
    })

    it('rejects non-http URLs', async () => {
      const handler = handlers.get('shell:open-external')!
      await expect(Promise.resolve(handler({}, 'file:///etc/passwd'))).resolves.toBe(false)
    })
  })

  describe('theme:set', () => {
    it('applies valid theme sources', async () => {
      const handler = handlers.get('theme:set')!
      await handler({}, 'dark')
      expect(mockNativeTheme.themeSource).toBe('dark')
      expect(mockSaveAppState).toHaveBeenCalledWith({ theme: 'dark' })
      expect(mockApplyWindowChrome).toHaveBeenCalled()
    })

    it('accepts light and system themes', async () => {
      const handler = handlers.get('theme:set')!
      await handler({}, 'light')
      expect(mockNativeTheme.themeSource).toBe('light')
      mockSaveAppState.mockClear()
      await handler({}, 'system')
      expect(mockNativeTheme.themeSource).toBe('system')
      expect(mockSaveAppState).toHaveBeenCalledWith({ theme: 'system' })
    })

    it('ignores invalid theme values', async () => {
      const handler = handlers.get('theme:set')!
      mockNativeTheme.themeSource = 'system'
      await handler({}, 'neon')
      expect(mockNativeTheme.themeSource).toBe('system')
      expect(mockSaveAppState).not.toHaveBeenCalled()
      expect(mockApplyWindowChrome).not.toHaveBeenCalled()
    })
  })

  describe('companion ipc', () => {
    it('rejects invalid companion settings without saving', async () => {
      const handler = handlers.get('companion:save-settings')!

      await expect(
        Promise.resolve().then(() =>
          handler({}, { provider: 'bogus', customCommand: 'custom acp' }),
        ),
      ).rejects.toMatchObject({ message: 'invalid-companion-settings' })

      expect(mockSaveAppState).not.toHaveBeenCalled()
    })

    it('rejects invalid companion send requests without calling the service', async () => {
      const handler = handlers.get('companion:send')!

      await expect(
        Promise.resolve().then(() =>
          handler({}, { messageId: 'msg_1', text: '', provider: 'opencode' }),
        ),
      ).rejects.toMatchObject({ message: 'invalid-companion-request' })

      expect(mockCompanionService.send).not.toHaveBeenCalled()
    })

    it('returns cleanup that shuts down the companion service', async () => {
      handlers.clear()
      const cleanup = registerIpcHandlers(
        mockGetMainWindow as unknown as () => BrowserWindow | null,
      )
      const handler = handlers.get('companion:detect-providers')!

      await handler({ sender: createMockWebContents(1) })

      cleanup()

      expect(mockCompanionService.shutdown).toHaveBeenCalledOnce()
    })

    it('routes companion updates to the sender window instead of the current main window', async () => {
      const senderSend = vi.fn()
      const mainSend = vi.fn()
      const senderWindow = {
        isDestroyed: () => false as const,
        webContents: createMockWebContents(1, senderSend),
      }
      const mainWindow = {
        isDestroyed: () => false as const,
        webContents: createMockWebContents(2, mainSend),
      }
      mockGetMainWindow.mockReturnValue(mainWindow)
      mockCompanionService.send.mockImplementation(() => {
        mockCompanionOptions.current?.emitUpdate({ type: 'status', status: 'streaming' })
        return Promise.resolve()
      })
      const handler = handlers.get('companion:send')!

      await handler(
        { sender: senderWindow.webContents },
        {
          messageId: 'msg_1',
          text: 'Hi',
          provider: 'opencode',
          activePath: null,
          openFolderPath: null,
        },
      )

      expect(senderSend).toHaveBeenCalledWith('companion:update', {
        type: 'status',
        status: 'streaming',
      })
      expect(mainSend).not.toHaveBeenCalled()
    })

    it('does not accumulate destroyed listeners across explicit shutdown and recreate', async () => {
      const sender = createMockWebContents(7)
      const sendHandler = handlers.get('companion:send')!
      const shutdownHandler = handlers.get('companion:shutdown')!
      const request = {
        messageId: 'msg_1',
        text: 'Hi',
        provider: 'opencode',
        activePath: null,
        openFolderPath: null,
      }

      await sendHandler({ sender }, request)
      await shutdownHandler({ sender })
      await sendHandler({ sender }, { ...request, messageId: 'msg_2' })

      expect(sender.once).toHaveBeenCalledTimes(2)
      expect(sender.off).toHaveBeenCalledTimes(1)
    })
  })
})

function createMockWebContents(id: number, send = vi.fn()) {
  return {
    id,
    send,
    isDestroyed: () => false,
    once: vi.fn(),
    off: vi.fn(),
  }
}
