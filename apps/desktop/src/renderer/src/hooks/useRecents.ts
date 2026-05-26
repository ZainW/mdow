import { useQuery } from '@tanstack/react-query'
import { queryKeys } from '../lib/query-keys'

export function useRecents() {
  return useQuery({
    queryKey: queryKeys.recents(),
    queryFn: () => window.api.getRecents(),
  })
}
