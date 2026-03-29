import { create } from 'zustand'

interface ActiveFile {
  path: string
  content: string
}

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

interface AppStore {
  activeFile: ActiveFile | null
  setActiveFile: (file: ActiveFile | null) => void
  updateActiveFileContent: (content: string) => void

  sidebarOpen: boolean
  sidebarWidth: number
  toggleSidebar: () => void
  setSidebarWidth: (width: number) => void

  openFolderPath: string | null
  folderTree: TreeNode[]
  setOpenFolder: (path: string, tree: TreeNode[]) => void
  setFolderTree: (tree: TreeNode[]) => void

  commandPaletteOpen: boolean
  setCommandPaletteOpen: (open: boolean) => void
}

export const useAppStore = create<AppStore>((set) => ({
  activeFile: null,
  setActiveFile: (file) => set({ activeFile: file }),
  updateActiveFileContent: (content) =>
    set((state) => {
      if (!state.activeFile) return state
      return { activeFile: { ...state.activeFile, content } }
    }),

  sidebarOpen: true,
  sidebarWidth: 260,
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarWidth: (width) => set({ sidebarWidth: width }),

  openFolderPath: null,
  folderTree: [],
  setOpenFolder: (path, tree) => set({ openFolderPath: path, folderTree: tree }),
  setFolderTree: (tree) => set({ folderTree: tree }),

  commandPaletteOpen: false,
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
}))
