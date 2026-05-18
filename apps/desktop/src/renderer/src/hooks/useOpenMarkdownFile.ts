import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore, type ErrorType } from '../store/app-store'

function getReadErrorType(error: unknown): ErrorType {
  if (error instanceof Error) {
    if (error.message === 'not-found') return 'not-found'
    if (error.message === 'permission-denied') return 'permission-denied'
  }
  return 'read-error'
}

export function useOpenMarkdownFile() {
  const openTab = useAppStore((s) => s.openTab)
  const openErrorTab = useAppStore((s) => s.openErrorTab)
  const queryClient = useQueryClient()

  return useCallback(
    async (path: string) => {
      try {
        const content = await window.api.readFile(path)
        openTab({ path, content })
        void queryClient.invalidateQueries({ queryKey: ['recents'] })
      } catch (error) {
        openErrorTab(path, { path, type: getReadErrorType(error) })
      }
    },
    [openTab, openErrorTab, queryClient],
  )
}
