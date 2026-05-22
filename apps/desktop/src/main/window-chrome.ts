import { BrowserWindow, nativeTheme } from 'electron'
import { isMac, isWindows } from './platform'
import { WINDOW_COLORS } from './theme-colors'

const TITLEBAR_OVERLAY_HEIGHT = 32

function getActiveColors() {
  return nativeTheme.shouldUseDarkColors ? WINDOW_COLORS.dark : WINDOW_COLORS.light
}

export function getWindowChromeOptions() {
  const colors = getActiveColors()

  return {
    backgroundColor: colors.background,
    ...(isMac ? { titleBarStyle: 'hiddenInset' as const } : {}),
    ...(isWindows
      ? {
          titleBarOverlay: {
            color: colors.background,
            symbolColor: colors.foreground,
            height: TITLEBAR_OVERLAY_HEIGHT,
          },
        }
      : {}),
  }
}

export function applyWindowChrome(win: BrowserWindow): void {
  if (win.isDestroyed()) return

  const colors = getActiveColors()
  win.setBackgroundColor(colors.background)

  if (isWindows) {
    win.setTitleBarOverlay({
      color: colors.background,
      symbolColor: colors.foreground,
      height: TITLEBAR_OVERLAY_HEIGHT,
    })
  }
}
