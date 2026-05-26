import type { QueryClient } from '@tanstack/react-query'

export const queryKeys = {
  recents: () => ['recents'] as const,
  folder: (path: string) => ['folder', path] as const,
}

export function invalidateRecents(queryClient: QueryClient): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.recents() })
}
