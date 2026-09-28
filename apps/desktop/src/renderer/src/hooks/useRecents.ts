import { useCallback } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { invalidateRecents, queryKeys } from '../lib/query-keys'

export function useRecents() {
  return useQuery({
    queryKey: queryKeys.recents(),
    queryFn: () => window.api.getRecents(),
  })
}

/** Forget every recent file. The list empties immediately, then refetches from disk. */
export function useClearRecents() {
  const queryClient = useQueryClient()
  return useCallback(async () => {
    queryClient.setQueryData(queryKeys.recents(), [])
    await window.api.saveAppState({ recents: [] })
    invalidateRecents(queryClient)
  }, [queryClient])
}
