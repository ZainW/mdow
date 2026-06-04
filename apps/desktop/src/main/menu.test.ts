import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { BrowserWindow } from 'electron'

const mockBuildFromTemplate = vi.hoisted(() => vi.fn((template) => ({ template })))
const mockSetApplicationMenu = vi.hoisted(() => vi.fn())
const mockGetRecents = vi.hoisted(() => vi.fn(() => []))

vi.mock('electron', () => ({
  Menu: {
    buildFromTemplate: mockBuildFromTemplate,
    setApplicationMenu: mockSetApplicationMenu,
  },
  app: { name: 'Mdow' },
  BrowserWindow: vi.fn(),
}))

vi.mock('@electron-toolkit/utils', () => ({ is: { dev: false } }))
vi.mock('./platform', () => ({ isMac: true }))
vi.mock('./store', () => ({ getRecents: mockGetRecents }))

import { createMenu } from './menu'

type MenuItem = Electron.MenuItemConstructorOptions

function createMockWindow(isDestroyed: boolean): {
  send: ReturnType<typeof vi.fn>
  win: BrowserWindow
} {
  const send = vi.fn()
  const win = {
    isDestroyed: () => isDestroyed,
    webContents: { isDestroyed: () => false, send },
  } as unknown as BrowserWindow
  return { send, win }
}

function getMenuItem(menuLabel: string, itemLabel: string): MenuItem {
  const template = mockBuildFromTemplate.mock.calls.at(-1)?.[0] as MenuItem[]
  const menu = template.find((item) => item.label === menuLabel)
  const submenu = menu?.submenu as MenuItem[]
  const item = submenu.find((entry) => entry.label === itemLabel)
  if (!item) throw new Error(`Missing menu item: ${menuLabel} > ${itemLabel}`)
  return item
}

describe('menu', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetRecents.mockReturnValue([])
  })

  it('ignores folder shortcut clicks when the main window has been destroyed', () => {
    const { send, win } = createMockWindow(true)
    createMenu(() => win)

    const openFolder = getMenuItem('File', 'Open Folder...')

    expect(() => openFolder.click?.({} as Electron.MenuItem, {} as BrowserWindow, {})).not.toThrow()
    expect(send).not.toHaveBeenCalled()
  })
})
