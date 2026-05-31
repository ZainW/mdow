import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { getReadErrorType } from '../lib/error-utils'
import { invalidateRecents } from '../lib/query-keys'

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
