import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore, type ErrorType } from '../store/app-store'
import { invalidateRecents } from '../lib/query-keys'

function getReadErrorType(error: unknown): ErrorType {
  if (error instanceof Error) {
    if (error.message === 'not-found') return 'not-found'
    if (error.message === 'permission-denied') return 'permission-denied'
    if (error.message === 'read-error') return 'read-error'
  }
  return 'read-error'
}

export function useOpenMarkdownFile() {
  const openTab = useAppStore((s) => s.openTab)
  const openErrorTab = useAppStore((s) => s.openErrorTab)
  const setOpeningPath = useAppStore((s) => s.setOpeningPath)
  const queryClient = useQueryClient()

  return useCallback(
    async (path: string) => {
      setOpeningPath(path)
      try {
        const content = await window.api.readFile(path)
        openTab({ path, content })
        invalidateRecents(queryClient)
      } catch (error) {
        openErrorTab(path, { path, type: getReadErrorType(error) })
      } finally {
        setOpeningPath(null)
      }
    },
    [openTab, openErrorTab, setOpeningPath, queryClient],
  )
}
