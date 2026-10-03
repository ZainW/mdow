import { renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import type { CompanionMessage, CompanionPart } from '../../../shared/types'
import { useAppStore } from '../store/app-store'
import { useDocumentReview } from './useDocumentReview'

const before = '# Plan\n\nWe ship in Q4.\n'
const patch = '@@ -3 +3 @@\n-We ship in Q4.\n+We ship in Q1.\n'

function change(overrides: Partial<Extract<CompanionPart, { kind: 'change' }>> = {}) {
  return {
    kind: 'change' as const,
    toolCallId: 'call_1',
    permissionId: 'per_1',
    status: 'pending' as const,
    files: [
      {
        path: '/docs/plan.md',
        displayPath: 'plan.md',
        patch,
        additions: 1,
        deletions: 1,
        status: 'modified' as const,
      },
    ],
    ...overrides,
  }
}

function assistant(...parts: CompanionPart[]): CompanionMessage {
  return { id: 'a1', role: 'assistant', content: '', parts, status: 'streaming' }
}

describe('useDocumentReview', () => {
  beforeEach(() => {
    useAppStore.setState({ companionMessages: [] })
  })

  it('offers the proposed text for the document the change targets', () => {
    useAppStore.setState({ companionMessages: [assistant(change())] })
    const { result } = renderHook(() => useDocumentReview('/docs/plan.md', before))
    expect(result.current).toMatchObject({
      toolCallId: 'call_1',
      permissionId: 'per_1',
      before,
      after: '# Plan\n\nWe ship in Q1.\n',
    })
    expect(renderHook(() => useDocumentReview('/docs/other.md', before)).result.current).toBeNull()
  })

  it('keeps an accepted change on screen until the file reloads', () => {
    useAppStore.setState({
      companionMessages: [assistant(change({ status: 'applied', permissionId: undefined }))],
    })
    const stale = renderHook(() => useDocumentReview('/docs/plan.md', before))
    expect(stale.result.current?.status).toBe('applied')
    const reloaded = renderHook(() =>
      useDocumentReview('/docs/plan.md', '# Plan\n\nWe ship in Q1.\n'),
    )
    expect(reloaded.result.current).toBeNull()
  })

  it('ignores declined changes and older responses', () => {
    useAppStore.setState({ companionMessages: [assistant(change({ status: 'rejected' }))] })
    expect(renderHook(() => useDocumentReview('/docs/plan.md', before)).result.current).toBeNull()

    useAppStore.setState({
      companionMessages: [
        assistant(change()),
        { id: 'u2', role: 'user', content: 'next', parts: [], status: 'complete' },
      ],
    })
    expect(renderHook(() => useDocumentReview('/docs/plan.md', before)).result.current).toBeNull()
  })
})
