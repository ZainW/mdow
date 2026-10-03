import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { CompanionMessage } from '../../../../shared/types'
import { selectPendingReviewPaths } from '../../hooks/usePendingReviews'
import { useAppStore } from '../../store/app-store'
import { stubWindowApi } from '../../test/stubWindowApi'
import { TabBar } from '../TabBar'
import { PendingReviewNotice } from './PendingReviewNotice'

stubWindowApi(() => ({ saveAppState: vi.fn().mockResolvedValue(undefined) }))

function waitingEdit(path: string, permissionId: string | null = 'per_1'): CompanionMessage {
  return {
    id: 'a1',
    role: 'assistant',
    content: '',
    status: 'streaming',
    parts: [
      {
        kind: 'change',
        toolCallId: 'call_1',
        permissionId: permissionId ?? undefined,
        status: 'pending',
        files: [
          {
            path,
            displayPath: path.split('/').pop()!,
            patch: '@@ -1 +1 @@\n-a\n+b\n',
            additions: 1,
            deletions: 1,
            status: 'modified',
          },
        ],
      },
    ],
  }
}

const tabs = [
  { id: 't1', path: '/docs/plan.md', content: '', scrollPosition: 0 },
  { id: 't2', path: '/docs/notes.md', content: '', scrollPosition: 0 },
]

beforeEach(() => {
  useAppStore.setState({
    companionMessages: [],
    companionPresentation: 'closed',
    tabs: tabs as never,
    activeTabId: 't1',
    splitView: false,
  })
})

describe('pending review cues', () => {
  it('collects only edits still waiting for a decision', () => {
    useAppStore.setState({ companionMessages: [waitingEdit('/docs/notes.md')] })
    expect(selectPendingReviewPaths(useAppStore.getState())).toBe('/docs/notes.md')
    useAppStore.setState({ companionMessages: [waitingEdit('/docs/notes.md', null)] })
    expect(selectPendingReviewPaths(useAppStore.getState())).toBe('')
  })

  it('points to an edit waiting in a document that is not on screen', () => {
    const listener = vi.fn()
    window.addEventListener('mdow:open-document-link', listener)
    useAppStore.setState({ companionMessages: [waitingEdit('/docs/notes.md')] })
    render(<PendingReviewNotice />)

    expect(screen.getByText('to notes.md')).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Review' }))
    window.removeEventListener('mdow:open-document-link', listener)
    expect((listener.mock.calls[0][0] as CustomEvent).detail).toEqual({ path: '/docs/notes.md' })
  })

  it('stays out of the way when the edit is already on screen', () => {
    useAppStore.setState({ companionMessages: [waitingEdit('/docs/plan.md')] })
    render(<PendingReviewNotice />)
    expect(screen.queryByRole('region', { name: /another document/ })).toBeNull()
  })

  it('marks the tab and the companion button', () => {
    useAppStore.setState({ companionMessages: [waitingEdit('/docs/notes.md')] })
    render(<TabBar />)
    expect(screen.getByRole('tab', { name: /notes\.md — suggested edit waiting/ })).toBeVisible()
    expect(screen.getByRole('tab', { name: /^plan\.md — / })).toBeVisible()
    expect(
      screen.getByRole('button', { name: 'Open companion — suggested edit waiting' }),
    ).toBeVisible()
  })
})
