import type { StateCreator } from 'zustand'

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
  fontSize: number
  lineHeight: number
  setContentFont: (font: string) => void
  setCodeFont: (font: string) => void
  setFontSize: (size: number) => void
  setLineHeight: (height: number) => void
}

export const createSettingsSlice: StateCreator<SettingsSlice, [], [], SettingsSlice> = (set) => ({
  zoomLevel: 100,
  zoomIn: () =>
    set((state) => {
      const next = Math.min(state.zoomLevel + 10, 200)
      void window.api.saveAppState({ zoomLevel: next })
      return { zoomLevel: next }
    }),
  zoomOut: () =>
    set((state) => {
      const next = Math.max(state.zoomLevel - 10, 60)
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
  fontSize: 15.5,
  lineHeight: 1.65,
  setContentFont: (font) => {
    void window.api.saveAppState({ contentFont: font })
    set({ contentFont: font })
  },
  setCodeFont: (font) => {
    void window.api.saveAppState({ codeFont: font })
    set({ codeFont: font })
  },
  setFontSize: (size) => {
    void window.api.saveAppState({ fontSize: size })
    set({ fontSize: size })
  },
  setLineHeight: (height) => {
    void window.api.saveAppState({ lineHeight: height })
    set({ lineHeight: height })
  },
})
