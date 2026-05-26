import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { invalidateRecents } from '../lib/query-keys'

export function useOpenFileDialog() {
  const openTab = useAppStore((s) => s.openTab)
  const queryClient = useQueryClient()

  return useCallback(async () => {
    const result = await window.api.openFileDialog()
    if (result) {
      openTab(result)
      invalidateRecents(queryClient)
    }
  }, [openTab, queryClient])
}
