import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { useIpcEvent } from './useIpcEvent'
import { useAppStore } from '../store/app-store'

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

export function useFolderTree(folderPath: string | null) {
  const queryClient = useQueryClient()
  const setFolderTree = useAppStore((s) => s.setFolderTree)

  const handleFolderChanged = useCallback(
    (tree: TreeNode[]) => {
      if (folderPath) {
        queryClient.setQueryData(['folder', folderPath], tree)
        setFolderTree(tree)
      }
    },
    [folderPath, queryClient, setFolderTree],
  )

  useIpcEvent(window.api.onFolderChanged, handleFolderChanged)

  return useQuery({
    queryKey: ['folder', folderPath],
    queryFn: () => window.api.readFolderTree(folderPath!),
    enabled: !!folderPath,
    staleTime: Infinity,
  })
}
