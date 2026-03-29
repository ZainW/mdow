import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { useIpcEvent } from './useIpcEvent'
import { useAppStore } from '../store/app-store'

export function useFileContent(filePath: string | null) {
  const queryClient = useQueryClient()
  const updateActiveFileContent = useAppStore((s) => s.updateActiveFileContent)

  const handleFileChanged = useCallback(
    (content: string) => {
      if (filePath) {
        queryClient.setQueryData(['file', filePath], content)
        updateActiveFileContent(content)
      }
    },
    [filePath, queryClient, updateActiveFileContent],
  )

  useIpcEvent(window.api.onFileChanged, handleFileChanged)

  return useQuery({
    queryKey: ['file', filePath],
    queryFn: () => window.api.readFile(filePath!),
    enabled: !!filePath,
    staleTime: Infinity,
  })
}
