import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { stubWindowApi } from '../../test/stubWindowApi'
import { useAppStore } from '../../store/app-store'
import { CompanionFullscreen, CompanionPanel } from './CompanionPanel'

const sendCompanionMessage = vi.fn(() => new Promise<void>(() => undefined))

stubWindowApi(() => ({
  detectCompanionProviders: vi.fn().mockResolvedValue([
    {
      id: 'opencode',
      label: 'OpenCode',
      commandDisplay: 'opencode acp',
      availability: 'available',
    },
  ]),
  getCompanionSettings: vi
    .fn()
    .mockResolvedValue({ preferredProvider: 'opencode', customCommand: '' }),
  saveCompanionSettings: vi.fn().mockResolvedValue(undefined),
  sendCompanionMessage,
  cancelCompanion: vi.fn().mockResolvedValue(undefined),
}))

beforeEach(() => {
  sendCompanionMessage.mockClear()
  useAppStore.getState().resetCompanionConversation()
  useAppStore.setState({
    companionOpen: true,
    companionFullscreen: false,
    companionProviders: [
      {
        id: 'opencode',
        label: 'OpenCode',
        commandDisplay: 'opencode acp',
        availability: 'available',
      },
    ],
    companionPreferredProvider: 'opencode',
    companionCustomCommand: '',
    companionTags: [],
    folderTree: [
      {
        name: 'overview.md',
        path: '/docs/overview.md',
        isDirectory: false,
      },
      {
        name: 'risks.md',
        path: '/docs/risks.md',
        isDirectory: false,
      },
    ],
    openFolderPath: '/docs',
  })
})

describe('CompanionPanel', () => {
  it('locks the composer immediately while the first request is pending', () => {
    render(<CompanionPanel />)
    const composer = screen.getByRole('textbox', { name: 'Ask about these docs' })

    fireEvent.change(composer, { target: { value: 'When is launch?' } })
    fireEvent.keyDown(composer, { key: 'Enter' })
    fireEvent.change(composer, { target: { value: 'Send this too' } })
    fireEvent.keyDown(composer, { key: 'Enter' })

    expect(sendCompanionMessage).toHaveBeenCalledOnce()
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeEnabled()
  })

  it('supports keyboard selection in the file mention listbox', () => {
    render(<CompanionPanel />)
    const composer = screen.getByRole('textbox', { name: 'Ask about these docs' })

    fireEvent.change(composer, { target: { value: '@r' } })
    expect(screen.getByRole('listbox', { name: 'Document suggestions' })).toBeVisible()
    fireEvent.keyDown(composer, { key: 'ArrowDown' })
    fireEvent.keyDown(composer, { key: 'Enter' })

    expect(screen.getByText('@risks.md')).toBeVisible()
    expect(screen.queryByRole('listbox', { name: 'Document suggestions' })).not.toBeInTheDocument()
  })

  it('uses an overlay on narrow windows instead of shrinking the document', () => {
    render(<CompanionPanel />)

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toHaveClass(
      'max-lg:fixed',
      'lg:relative',
    )
  })

  it('allows the expanded dialog to exceed the base small-dialog width', () => {
    useAppStore.setState({ companionFullscreen: true })
    const { container } = render(
      <>
        <CompanionPanel />
        <CompanionFullscreen />
      </>,
    )

    expect(screen.getByRole('dialog')).toHaveClass('sm:max-w-none')
    expect(container.querySelector('aside[aria-label="AI companion"]')).toHaveAttribute('inert')
  })

  it('keeps thinking and tool details collapsed while activity streams', () => {
    useAppStore.setState({
      companionMessages: [
        {
          id: 'assistant-1',
          role: 'assistant',
          content: '',
          status: 'streaming',
          citations: [],
          parts: [
            { kind: 'thinking', text: 'Long private reasoning', done: false },
            {
              kind: 'tool',
              toolCallId: 'tool-1',
              name: 'Search docs',
              state: 'running',
              input: '{"query":"launch"}',
            },
          ],
        },
      ],
      companionStreaming: true,
    })

    render(<CompanionPanel />)

    expect(screen.getByRole('button', { name: /thinking/i })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
    expect(screen.getByRole('button', { name: /search docs/i })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
    expect(screen.queryByText('Long private reasoning')).not.toBeInTheDocument()
    expect(screen.queryByText('{"query":"launch"}')).not.toBeInTheDocument()
  })
})
