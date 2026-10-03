import { createRef } from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DocumentReview } from '../../hooks/useDocumentReview'
import { useAppStore } from '../../store/app-store'
import { ReviewBar } from './ReviewBar'

const review: DocumentReview = {
  toolCallId: 'call_1',
  permissionId: 'per_1',
  status: 'pending',
  file: {
    path: '/docs/plan.md',
    displayPath: 'plan.md',
    patch: '',
    additions: 1,
    deletions: 1,
    status: 'modified',
  },
  before: 'a',
  after: 'b',
}

const decide = vi.fn().mockResolvedValue(undefined)

beforeEach(() => {
  decide.mockClear()
  useAppStore.setState({ reviewCompanionChange: decide })
  Element.prototype.scrollIntoView = vi.fn()
})

function renderBar(overrides: Partial<DocumentReview> = {}, changeCount = 3) {
  const containerRef = createRef<HTMLDivElement>()
  render(
    <div>
      <div ref={containerRef}>
        {Array.from({ length: changeCount }, (_, index) => (
          <div key={index} data-change-index={index} />
        ))}
      </div>
      <ReviewBar
        review={{ ...review, ...overrides }}
        changeCount={changeCount}
        containerRef={containerRef}
      />
    </div>,
  )
  return containerRef
}

describe('ReviewBar', () => {
  it('accepts and declines the suggestion', () => {
    renderBar()
    fireEvent.click(screen.getByRole('button', { name: /accept/i }))
    expect(decide).toHaveBeenCalledWith('per_1', 'approve')
    fireEvent.click(screen.getByRole('button', { name: 'Decline' }))
    expect(decide).toHaveBeenCalledWith('per_1', 'reject')
  })

  it('steps through changes and marks the active one', () => {
    const containerRef = renderBar()
    expect(screen.getByText('1 of 3')).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Next change' }))
    expect(screen.getByText('2 of 3')).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Previous change' }))
    fireEvent.click(screen.getByRole('button', { name: 'Previous change' }))
    expect(screen.getByText('3 of 3')).toBeVisible()
    const changes = containerRef.current!.querySelectorAll<HTMLElement>('[data-change-index]')
    expect(changes[2].dataset.reviewActive).toBe('')
    expect(changes[0].dataset.reviewActive).toBeUndefined()
  })

  it('accepts with the keyboard, but not while typing', () => {
    renderBar()
    const field = document.createElement('textarea')
    document.body.append(field)
    fireEvent.keyDown(field, { key: 'Enter', ctrlKey: true, metaKey: true })
    expect(decide).not.toHaveBeenCalled()
    fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true, metaKey: true })
    expect(decide).toHaveBeenCalledWith('per_1', 'approve')
    field.remove()
  })

  it('shows progress instead of buttons once decided', () => {
    renderBar({ permissionId: null }, 1)
    expect(screen.getByText('Applying edit…')).toBeVisible()
    expect(screen.queryByRole('button', { name: /accept/i })).toBeNull()
    expect(screen.queryByText(/of 1/)).toBeNull()
  })
})
