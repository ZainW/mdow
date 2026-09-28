import type { StateCreator } from 'zustand'
import type { InterfaceScale } from '../../../../shared/types'
import { DEFAULT_READING_WIDTH, type ReadingWidth } from '../../../../shared/reading-width'

export const ZOOM_MIN = 60
export const ZOOM_MAX = 200
const ZOOM_STEP = 10

export interface SettingsSlice {
  zoomLevel: number
  zoomIn: () => void
  zoomOut: () => void
  resetZoom: () => void
  theme: string
  setTheme: (theme: string) => void
  autoUpdateEnabled: boolean
  setAutoUpdateEnabled: (enabled: boolean) => void
  contentFont: string
  codeFont: string
  interfaceScale: InterfaceScale
  readingWidth: ReadingWidth
  /** The constrained width to return to when leaving Full (not persisted). */
  lastConstrainedWidth: Exclude<ReadingWidth, 'full'>
  setContentFont: (font: string) => void
  setCodeFont: (font: string) => void
  setInterfaceScale: (scale: InterfaceScale) => void
  setReadingWidth: (width: ReadingWidth) => void
  /** Flip between Full line width and the last constrained width. */
  toggleWideMode: () => void
}

function persistReadingWidth(width: ReadingWidth) {
  if (typeof window !== 'undefined' && window.api) {
    void window.api.saveAppState({ readingWidth: width })
  }
}

export const createSettingsSlice: StateCreator<SettingsSlice, [], [], SettingsSlice> = (set) => ({
  zoomLevel: 100,
  zoomIn: () =>
    set((state) => {
      const next = Math.min(state.zoomLevel + ZOOM_STEP, ZOOM_MAX)
      void window.api.saveAppState({ zoomLevel: next })
      return { zoomLevel: next }
    }),
  zoomOut: () =>
    set((state) => {
      const next = Math.max(state.zoomLevel - ZOOM_STEP, ZOOM_MIN)
      void window.api.saveAppState({ zoomLevel: next })
      return { zoomLevel: next }
    }),
  resetZoom: () => {
    void window.api.saveAppState({ zoomLevel: 100 })
    return set({ zoomLevel: 100 })
  },

  theme: 'system',
  setTheme: (theme) => {
    void window.api.setTheme(theme)
    set({ theme })
  },

  autoUpdateEnabled: true,
  setAutoUpdateEnabled: (enabled) => {
    void window.api.saveAppState({ autoUpdateEnabled: enabled })
    void window.api.setAutoUpdateScheduling(enabled)
    set({ autoUpdateEnabled: enabled })
  },

  contentFont: 'inter',
  codeFont: 'geist-mono',
  interfaceScale: 'compact',
  readingWidth: DEFAULT_READING_WIDTH,
  lastConstrainedWidth: 'medium',
  setContentFont: (font) => {
    void window.api.saveAppState({ contentFont: font })
    set({ contentFont: font })
  },
  setCodeFont: (font) => {
    void window.api.saveAppState({ codeFont: font })
    set({ codeFont: font })
  },
  setInterfaceScale: (scale) => {
    void window.api.saveAppState({ interfaceScale: scale })
    set({ interfaceScale: scale })
  },
  setReadingWidth: (width) => {
    persistReadingWidth(width)
    set(
      width === 'full'
        ? { readingWidth: width }
        : { readingWidth: width, lastConstrainedWidth: width },
    )
  },
  toggleWideMode: () =>
    set((state) => {
      const readingWidth = state.readingWidth === 'full' ? state.lastConstrainedWidth : 'full'
      persistReadingWidth(readingWidth)
      return { readingWidth }
    }),
})
