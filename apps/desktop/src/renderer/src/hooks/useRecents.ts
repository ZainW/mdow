import { useQuery } from '@tanstack/react-query'

export function useRecents() {
  return useQuery({
    queryKey: ['recents'],
    queryFn: () => window.api.getRecents(),
  })
}
