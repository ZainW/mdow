import { useMemo } from 'react'
import { useAppStore, type AppStore } from '../store/app-store'

/**
 * Documents with a companion edit waiting for a decision, newline-joined so the selection stays
 * referentially stable while nothing changes. OpenCode pauses on one edit at a time, so in
 * practice this is a single document, but an edit may touch several.
 */
export function selectPendingReviewPaths(state: AppStore): string {
  const last = state.companionMessages.at(-1)
  if (last?.role !== 'assistant') return ''
  const paths = new Set<string>()
  for (const part of last.parts) {
    if (part.kind !== 'change' || part.status !== 'pending' || !part.permissionId) continue
    for (const file of part.files) paths.add(file.path)
  }
  return [...paths].join('\n')
}

export function usePendingReviewPaths(): ReadonlySet<string> {
  const joined = useAppStore(selectPendingReviewPaths)
  return useMemo(() => new Set(joined ? joined.split('\n') : []), [joined])
}
