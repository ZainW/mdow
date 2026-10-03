import { useMemo } from 'react'
import type { CompanionChangeStatus, CompanionFileChange } from '../../../shared/types'
import { applyUnifiedPatch, isPatchApplied } from '../lib/apply-patch'
import { useAppStore } from '../store/app-store'

export interface DocumentReview {
  /** The change's id in the conversation; stable for the life of the review. */
  toolCallId: string
  /** Set while OpenCode waits for a decision; null once the user has decided. */
  permissionId: string | null
  status: CompanionChangeStatus
  file: CompanionFileChange
  before: string
  after: string
}

/**
 * The companion's suggested change to the document at `path`, if any, with the proposed text.
 * Only the latest response is considered. A change stays on screen after approval until the
 * reader reloads the edited file, so the document never flashes back to the old text.
 */
export function useDocumentReview(path: string, content: string): DocumentReview | null {
  const change = useAppStore((state) => {
    const last = state.companionMessages.at(-1)
    if (last?.role !== 'assistant') return null
    return (
      last.parts.findLast(
        (part) =>
          part.kind === 'change' &&
          (part.status === 'pending' || part.status === 'applied') &&
          part.files.some((file) => file.path === path && file.patch),
      ) ?? null
    )
  })

  return useMemo(() => {
    if (change?.kind !== 'change') return null
    const file = change.files.find((candidate) => candidate.path === path)
    if (!file) return null
    // A patch that no longer applies means the document already holds the new text (or was
    // edited elsewhere), so there is nothing left to review here.
    if (isPatchApplied(content, file.patch)) return null
    const after = applyUnifiedPatch(content, file.patch)
    if (after === null || after === content) return null
    return {
      toolCallId: change.toolCallId,
      permissionId: change.permissionId ?? null,
      status: change.status,
      file,
      before: content,
      after,
    }
  }, [change, path, content])
}
