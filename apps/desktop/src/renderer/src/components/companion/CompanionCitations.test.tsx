import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from '../../store/app-store'
import { CompanionMessages } from './CompanionMessages'

const openMarkdownFile = vi.hoisted(() => vi.fn(async () => {}))

vi.mock('../../hooks/useOpenMarkdownFile', () => ({
  useOpenMarkdownFile: () => openMarkdownFile,
}))

describe('companion citations', () => {
  beforeEach(() => {
    openMarkdownFile.mockClear()
    useAppStore.getState().resetCompanion()
  })

  it('opens cited markdown files from source chips', () => {
    useAppStore.getState().setCompanionMessages([
      {
        id: 'a1',
        role: 'assistant',
        content: 'See the docs.',
        status: 'complete',
        createdAt: 1,
        citations: [{ sourceId: 'src_active', title: 'README.md', path: '/docs/README.md' }],
      },
    ])

    render(<CompanionMessages />)

    fireEvent.click(screen.getByRole('button', { name: 'Used 1 sources' }))
    fireEvent.click(screen.getByRole('button', { name: 'README.md' }))

    expect(openMarkdownFile).toHaveBeenCalledWith('/docs/README.md')
  })
})
