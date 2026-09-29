import { createContext, useContext, useMemo, type ReactNode } from 'react'
import type { GpuixTheme } from '@gpuix/react'
import { activeScheme, useApp } from '../store'
import {
  CODE_FONT_FAMILY,
  CONTENT_FONT_FAMILY,
  READER_FONT_SIZE,
  READER_LINE_HEIGHT,
  SCALE_TOKENS,
  type ScaleTokens,
} from '../lib/prefs'
import { nativeTheme, themeFor, type Theme } from '../lib/theme'

export const UI_FONT = 'Inter'
export const UI_MONO = 'Geist Mono'

export const METRICS = {
  titlebarHeight: 40,
  trafficLightInset: 14,
  trafficLightClearance: 80,
  sidebarWidth: 244,
  minMainWidthWithSidebar: 320,
  chromeRowHeight: 44,
  breadcrumbHeight: 36,
  tabHeight: 28,
  tabMaxWidth: 200,
  readerInset: 32,
  readerTopPadding: 32,
  readerBottomPadding: 40,
  radius: 8,
}

export interface Ui {
  theme: Theme
  scale: ScaleTokens
  reader: {
    fontSize: number
    lineHeight: number
    contentFont: string
    codeFont: string
    native: GpuixTheme
  }
}

const UiContext = createContext<Ui | null>(null)

export function UiProvider({ children }: { children: ReactNode }) {
  const scheme = useApp(activeScheme)
  const prefs = useApp((state) => state.prefs)
  const value = useMemo<Ui>(() => {
    const theme = themeFor(scheme)
    const fontSize = Math.round(READER_FONT_SIZE * (prefs.zoomLevel / 100) * 10) / 10
    const reader = {
      fontSize,
      lineHeight: READER_LINE_HEIGHT,
      contentFont: CONTENT_FONT_FAMILY[prefs.contentFont],
      codeFont: CODE_FONT_FAMILY[prefs.codeFont],
    }
    return {
      theme,
      scale: SCALE_TOKENS[prefs.interfaceScale],
      reader: { ...reader, native: nativeTheme(theme, reader) },
    }
  }, [scheme, prefs.zoomLevel, prefs.contentFont, prefs.codeFont, prefs.interfaceScale])
  return <UiContext.Provider value={value}>{children}</UiContext.Provider>
}

export function useUi(): Ui {
  const ui = useContext(UiContext)
  if (!ui) throw new Error('useUi outside UiProvider')
  return ui
}
