import type { StateCreator } from 'zustand'
import type { DocHeading } from '../../lib/markdown'

export type SidebarMode = 'recents' | 'folder' | 'outline'

export interface UiSlice {
  initialized: boolean
  sidebarOpen: boolean
  sidebarMode: SidebarMode
  toggleSidebar: () => void
  setSidebarMode: (mode: SidebarMode) => void
  wideMode: boolean
  toggleWideMode: () => void
  commandPaletteOpen: boolean
  setCommandPaletteOpen: (open: boolean) => void
  searchOpen: boolean
  setSearchOpen: (open: boolean) => void
  shortcutsDialogOpen: boolean
  setShortcutsDialogOpen: (open: boolean) => void
  settingsOpen: boolean
  setSettingsOpen: (open: boolean) => void
  docHeadings: DocHeading[]
  activeHeadingId: string | null
  setDocHeadings: (headings: DocHeading[]) => void
  setActiveHeadingId: (id: string | null) => void
}

export const createUiSlice: StateCreator<UiSlice, [], [], UiSlice> = (set) => ({
  initialized: false,
  sidebarOpen: true,
  sidebarMode: 'recents',
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarMode: (mode) => {
    if (typeof window !== 'undefined' && window.api) {
      void window.api.saveAppState({ sidebarMode: mode })
    }
    set({ sidebarMode: mode })
  },

  wideMode: false,
  toggleWideMode: () =>
    set((state) => {
      const wideMode = !state.wideMode
      if (typeof window !== 'undefined' && window.api) {
        void window.api.saveAppState({ wideMode })
      }
      return { wideMode }
    }),

  docHeadings: [],
  activeHeadingId: null,
  setDocHeadings: (headings) => set({ docHeadings: headings }),
  setActiveHeadingId: (id) => set({ activeHeadingId: id }),

  commandPaletteOpen: false,
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),

  searchOpen: false,
  setSearchOpen: (open) => set({ searchOpen: open }),

  shortcutsDialogOpen: false,
  setShortcutsDialogOpen: (open) => set({ shortcutsDialogOpen: open }),

  settingsOpen: false,
  setSettingsOpen: (open) => set({ settingsOpen: open }),
})
