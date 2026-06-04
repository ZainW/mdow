import { Menu, app, BrowserWindow } from 'electron'
import { is } from '@electron-toolkit/utils'
import { isMac } from './platform'
import { getRecents } from './store'

function sendToMainWindow(
  getMainWindow: () => BrowserWindow | null,
  channel: string,
  ...args: unknown[]
): void {
  const win = getMainWindow()
  if (!win || win.isDestroyed() || win.webContents.isDestroyed()) return
  win.webContents.send(channel, ...args)
}

function buildRecentSubmenu(
  getMainWindow: () => BrowserWindow | null,
): Electron.MenuItemConstructorOptions[] {
  const recents = getRecents()
  if (recents.length === 0) {
    return [{ label: 'No Recent Files', enabled: false }]
  }
  return recents.map((path) => ({
    label: path.split(/[/\\]/).pop() ?? path,
    click: () => sendToMainWindow(getMainWindow, 'menu:open-recent', path),
  }))
}

function buildDevToolsItems(): Electron.MenuItemConstructorOptions[] {
  if (!is.dev) return []
  return [
    { role: 'reload' },
    { role: 'forceReload' },
    { role: 'toggleDevTools' },
    { type: 'separator' },
  ]
}

export function createMenu(getMainWindow: () => BrowserWindow | null): void {
  const template: Electron.MenuItemConstructorOptions[] = [
    ...(isMac
      ? [
          {
            label: app.name,
            submenu: [
              { role: 'about' as const },
              { type: 'separator' as const },
              { role: 'services' as const },
              { type: 'separator' as const },
              { role: 'hide' as const },
              { role: 'hideOthers' as const },
              { role: 'unhide' as const },
              { type: 'separator' as const },
              { role: 'quit' as const },
            ],
          },
        ]
      : []),
    {
      label: 'File',
      submenu: [
        {
          label: 'Open File...',
          accelerator: 'CmdOrCtrl+O',
          click: () => sendToMainWindow(getMainWindow, 'menu:open-file'),
        },
        {
          label: 'Open Folder...',
          accelerator: 'CmdOrCtrl+Shift+O',
          click: () => sendToMainWindow(getMainWindow, 'menu:open-folder'),
        },
        {
          label: 'Open Recent',
          submenu: buildRecentSubmenu(getMainWindow),
        },
        { type: 'separator' },
        {
          label: 'Close Tab',
          accelerator: 'CmdOrCtrl+W',
          click: () => sendToMainWindow(getMainWindow, 'menu:close-tab'),
        },
        ...(isMac ? [] : [{ type: 'separator' as const }, { role: 'quit' as const }]),
      ],
    },
    {
      label: 'Edit',
      submenu: [
        { role: 'undo' },
        { role: 'redo' },
        { type: 'separator' },
        { role: 'cut' },
        { role: 'copy' },
        { role: 'paste' },
        { role: 'selectAll' },
        { type: 'separator' },
        {
          label: 'Find...',
          accelerator: 'CmdOrCtrl+F',
          click: () => sendToMainWindow(getMainWindow, 'menu:find'),
        },
      ],
    },
    {
      label: 'View',
      submenu: [
        {
          label: 'Toggle Sidebar',
          accelerator: 'CmdOrCtrl+B',
          click: () => sendToMainWindow(getMainWindow, 'menu:toggle-sidebar'),
        },
        { type: 'separator' },
        {
          label: 'Settings...',
          accelerator: 'CmdOrCtrl+,',
          click: () => sendToMainWindow(getMainWindow, 'menu:settings'),
        },
        { type: 'separator' },
        ...buildDevToolsItems(),
        {
          label: 'Actual Size',
          accelerator: 'CmdOrCtrl+0',
          click: () => sendToMainWindow(getMainWindow, 'menu:zoom-reset'),
        },
        {
          label: 'Zoom In',
          accelerator: 'CmdOrCtrl+=',
          click: () => sendToMainWindow(getMainWindow, 'menu:zoom-in'),
        },
        {
          label: 'Zoom Out',
          accelerator: 'CmdOrCtrl+-',
          click: () => sendToMainWindow(getMainWindow, 'menu:zoom-out'),
        },
        { type: 'separator' },
        { role: 'togglefullscreen' },
      ],
    },
    {
      label: 'Window',
      submenu: [
        { role: 'minimize' },
        { role: 'zoom' },
        ...(isMac
          ? [{ type: 'separator' as const }, { role: 'front' as const }]
          : [{ role: 'close' as const }]),
      ],
    },
    {
      label: 'Help',
      submenu: [
        {
          label: 'Keyboard Shortcuts',
          accelerator: 'CmdOrCtrl+/',
          click: () => sendToMainWindow(getMainWindow, 'menu:shortcuts'),
        },
        { type: 'separator' },
        {
          label: 'Check for Updates…',
          click: () => {
            sendToMainWindow(getMainWindow, 'menu:check-for-updates')
          },
        },
      ],
    },
  ]

  const menu = Menu.buildFromTemplate(template)
  Menu.setApplicationMenu(menu)
}

export function rebuildMenu(getMainWindow: () => BrowserWindow | null): void {
  createMenu(getMainWindow)
}
