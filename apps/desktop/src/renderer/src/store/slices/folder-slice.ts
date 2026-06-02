import type { StateCreator } from 'zustand'
import type { TreeNode } from '../../../../shared/types'

export interface FolderSlice {
  openFolderPath: string | null
  folderTree: TreeNode[]
  folderTreeTruncated: boolean
  setOpenFolder: (path: string, tree: TreeNode[], truncated: boolean) => void
  setFolderTree: (tree: TreeNode[], truncated: boolean) => void
}

export const createFolderSlice: StateCreator<FolderSlice, [], [], FolderSlice> = (set) => ({
  openFolderPath: null,
  folderTree: [],
  folderTreeTruncated: false,
  setOpenFolder: (path, tree, truncated) =>
    set({ openFolderPath: path, folderTree: tree, folderTreeTruncated: truncated }),
  setFolderTree: (tree, truncated) => set({ folderTree: tree, folderTreeTruncated: truncated }),
})
