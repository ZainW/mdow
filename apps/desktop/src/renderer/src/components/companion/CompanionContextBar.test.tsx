import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { CompanionContextTrace } from '../../../../shared/types'
import { CompanionContextBar } from './CompanionContextBar'

const trace: CompanionContextTrace = {
  focusedCount: 1,
  attachedCount: 0,
  searchedCount: 1,
  readRangeCount: 2,
  injectedBytes: 5_000,
  estimatedTokens: 1_250,
  retrievalMode: 'adaptive-fff',
  items: [
    { path: '/docs/overview.md', reason: 'focused', bytes: 3_000 },
    { path: '/docs/architecture.md', reason: 'retrieved', bytes: 2_000 },
  ],
}

describe('CompanionContextBar', () => {
  it('keeps context compact and reveals honest details in a popover', async () => {
    render(<CompanionContextBar trace={trace} warnings={['Selected relevant sections']} />)

    expect(screen.getByText(/1 focused/)).toBeVisible()
    expect(screen.getByText(/Adaptive · FFF/)).toBeVisible()
    expect(screen.getByText(/≈1.3k added/)).toBeVisible()

    fireEvent.click(screen.getByText(/1 focused/))
    expect(await screen.findByText('Context added by Mdow')).toBeVisible()
    expect(screen.getByText(/not the provider's full context window/i)).toBeVisible()
    expect(screen.getByText('Selected relevant sections')).toBeVisible()
  })

  it('renders nothing before a context packet exists', () => {
    const { container } = render(<CompanionContextBar trace={null} warnings={[]} />)
    expect(container).toBeEmptyDOMElement()
  })
})
