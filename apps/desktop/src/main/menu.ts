import { Menu, app, BrowserWindow, shell } from 'electron'
import { isMac } from './platform'

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
          click: () => getMainWindow()?.webContents.send('menu:open-file'),
        },
        {
          label: 'Open Folder...',
          accelerator: 'CmdOrCtrl+Shift+O',
          click: () => getMainWindow()?.webContents.send('menu:open-folder'),
        },
        { type: 'separator' },
        ...(isMac
          ? [
              {
                label: 'Close Tab',
                accelerator: 'CmdOrCtrl+W',
                click: () => getMainWindow()?.webContents.send('menu:close-tab'),
              },
            ]
          : [{ role: 'quit' as const }]),
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
          click: () => getMainWindow()?.webContents.send('menu:find'),
        },
      ],
    },
    {
      label: 'View',
      submenu: [
        {
          label: 'Toggle Sidebar',
          accelerator: 'CmdOrCtrl+B',
          click: () => getMainWindow()?.webContents.send('menu:toggle-sidebar'),
        },
        { type: 'separator' },
        {
          label: 'Settings...',
          accelerator: 'CmdOrCtrl+,',
          click: () => getMainWindow()?.webContents.send('menu:settings'),
        },
        { type: 'separator' },
        { role: 'reload' },
        { role: 'forceReload' },
        { role: 'toggleDevTools' },
        { type: 'separator' },
        {
          label: 'Actual Size',
          accelerator: 'CmdOrCtrl+0',
          click: () => getMainWindow()?.webContents.send('menu:zoom-reset'),
        },
        {
          label: 'Zoom In',
          accelerator: 'CmdOrCtrl+=',
          click: () => getMainWindow()?.webContents.send('menu:zoom-in'),
        },
        {
          label: 'Zoom Out',
          accelerator: 'CmdOrCtrl+-',
          click: () => getMainWindow()?.webContents.send('menu:zoom-out'),
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
          click: () => getMainWindow()?.webContents.send('menu:shortcuts'),
        },
        { type: 'separator' },
        {
          label: 'Check for Updates…',
          click: () => {
            if (isMac) {
              void shell.openExternal('https://github.com/ZainW/mdow/releases/latest')
            } else {
              getMainWindow()?.webContents.send('menu:check-for-updates')
            }
          },
        },
      ],
    },
  ]

  const menu = Menu.buildFromTemplate(template)
  Menu.setApplicationMenu(menu)
}
