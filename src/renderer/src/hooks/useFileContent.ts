import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { useIpcEvent } from './useIpcEvent'
import { useAppStore } from '../store/app-store'

export function useFileContent(filePath: string | null) {
  const queryClient = useQueryClient()
  const updateTabContent = useAppStore((s) => s.updateTabContent)

  const handleFileChanged = useCallback(
    (data: { path: string; content: string }) => {
      if (filePath) {
        queryClient.setQueryData(['file', filePath], data.content)
        updateTabContent(data.path, data.content)
      }
    },
    [filePath, queryClient, updateTabContent],
  )

  useIpcEvent(window.api.onFileChanged, handleFileChanged)

  return useQuery({
    queryKey: ['file', filePath],
    queryFn: () => window.api.readFile(filePath!),
    enabled: !!filePath,
    staleTime: Infinity,
  })
}
