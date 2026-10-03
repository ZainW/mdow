import { fireEvent, render, screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { stubWindowApi } from '../../test/stubWindowApi'
import { useAppStore } from '../../store/app-store'
import { CompanionPanel, CompanionShell, CompanionWorkspace } from './CompanionPanel'

const sendCompanionMessage = vi.fn(() => new Promise<void>(() => undefined))
const replyCompanionPermission = vi.fn().mockResolvedValue(undefined)

const models = {
  options: [
    {
      value: 'opencode/claude-sonnet-5-5',
      name: 'Claude Sonnet 5.5',
      providerId: 'opencode',
      providerName: 'OpenCode Zen',
    },
  ],
  currentValue: 'opencode/claude-sonnet-5-5',
  stale: false,
}

stubWindowApi(() => ({
  startCompanion: vi.fn().mockResolvedValue({ availability: 'available', version: '2.0.16' }),
  getCompanionModels: vi.fn().mockResolvedValue(models),
  setCompanionModel: vi.fn().mockResolvedValue(models),
  sendCompanionMessage,
  replyCompanionPermission,
  cancelCompanion: vi.fn().mockResolvedValue(undefined),
  resetCompanion: vi.fn().mockResolvedValue(undefined),
  openExternal: vi.fn().mockResolvedValue(undefined),
  saveAppState: vi.fn().mockResolvedValue(undefined),
}))

beforeEach(() => {
  sendCompanionMessage.mockClear()
  replyCompanionPermission.mockClear()
  useAppStore.getState().resetCompanionConversation()
  useAppStore.setState({
    companionPresentation: 'drawer',
    companionStatus: { availability: 'available', version: '2.0.16' },
    companionModelState: models,
    companionTags: [],
    folderTree: [
      { name: 'overview.md', path: '/docs/overview.md', isDirectory: false },
      { name: 'risks.md', path: '/docs/risks.md', isDirectory: false },
    ],
    openFolderPath: '/docs',
    tabs: [],
    activeTabId: null,
  })
})

describe('CompanionPanel', () => {
  it('locks the composer while the first request is pending', () => {
    render(<CompanionPanel />)
    const composer = screen.getByRole('combobox', { name: 'Message the companion' })

    fireEvent.change(composer, { target: { value: 'When is launch?' } })
    fireEvent.keyDown(composer, { key: 'Enter' })
    fireEvent.change(composer, { target: { value: 'Send this too' } })
    fireEvent.keyDown(composer, { key: 'Enter' })

    expect(sendCompanionMessage).toHaveBeenCalledOnce()
    expect(sendCompanionMessage).toHaveBeenCalledWith({
      text: 'When is launch?',
      activePath: null,
      openFolderPath: '/docs',
      tags: [],
    })
    expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled()
  })

  it('sends a suggestion with one click', () => {
    render(<CompanionPanel />)
    fireEvent.click(screen.getByRole('button', { name: /what's in this folder/i }))
    expect(sendCompanionMessage).toHaveBeenCalledWith(
      expect.objectContaining({ text: 'Give me a quick tour of the documents in this folder.' }),
    )
  })

  it('offers document suggestions for the viewed document', () => {
    useAppStore.setState({
      tabs: [{ id: 't1', path: '/docs/overview.md' } as never],
      activeTabId: 't1',
    })
    render(<CompanionPanel />)
    expect(screen.getByText('Ask about overview.md')).toBeVisible()
    expect(screen.getByRole('button', { name: /fix spelling and grammar/i })).toBeVisible()
  })

  it('supports keyboard selection in the document mention list', () => {
    render(<CompanionPanel />)
    const composer = screen.getByRole('combobox', { name: 'Message the companion' })

    fireEvent.change(composer, { target: { value: '@r' } })
    expect(screen.getByRole('listbox', { name: 'Documents' })).toBeVisible()
    fireEvent.keyDown(composer, { key: 'Enter' })

    expect(screen.getByRole('button', { name: 'Remove risks.md' })).toBeVisible()
    expect(screen.queryByRole('listbox', { name: 'Documents' })).not.toBeInTheDocument()
  })

  it('shows install steps when OpenCode is missing', () => {
    useAppStore.setState({
      companionStatus: { availability: 'missing', detail: 'Install OpenCode 2' },
    })
    render(<CompanionPanel />)
    expect(screen.getByRole('heading', { name: 'Connect OpenCode' })).toBeVisible()
    expect(screen.getByText('opencode auth login')).toBeVisible()
    expect(screen.queryByRole('combobox', { name: 'Message the companion' })).toBeNull()
  })

  it('asks for an upgrade when OpenCode 1 is installed', () => {
    useAppStore.setState({ companionStatus: { availability: 'outdated', version: '1.18.0' } })
    render(<CompanionPanel />)
    expect(screen.getByRole('heading', { name: 'Update OpenCode' })).toBeVisible()
    expect(screen.getByText('opencode upgrade')).toBeVisible()
  })

  it('reviews a proposed change from its card', () => {
    useAppStore.setState({
      companionStreaming: true,
      companionMessages: [
        {
          id: 'assistant-1',
          role: 'assistant',
          content: '',
          status: 'streaming',
          parts: [
            {
              kind: 'change',
              toolCallId: 'call_1',
              permissionId: 'per_1',
              status: 'pending',
              files: [
                {
                  path: '/docs/overview.md',
                  displayPath: 'overview.md',
                  patch: '@@ -3 +3 @@\n-Ship in Q4.\n+Ship in Q1.\n',
                  additions: 1,
                  deletions: 1,
                  status: 'modified',
                },
              ],
            },
          ],
        },
      ],
    })
    render(<CompanionPanel />)

    const card = screen.getByRole('region', { name: 'Change to overview.md' })
    expect(within(card).getByText('Ship in Q4.')).toBeVisible()
    expect(within(card).getByText('Ship in Q1.')).toBeVisible()
    fireEvent.click(within(card).getByRole('button', { name: 'Accept' }))

    expect(replyCompanionPermission).toHaveBeenCalledWith('per_1', 'approve')
    expect(within(card).queryByRole('button', { name: 'Accept' })).toBeNull()
    expect(within(card).getByText('Applying')).toBeVisible()
  })

  it('uses an overlay on narrow windows instead of shrinking the document', () => {
    render(<CompanionPanel />)

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toHaveClass(
      'max-lg:fixed',
      'lg:relative',
    )
  })

  it('renders the expanded companion as a workspace with the model picker', () => {
    useAppStore.setState({ companionPresentation: 'workspace' })
    render(<CompanionWorkspace />)

    expect(screen.getByRole('region', { name: 'AI companion workspace' })).toBeVisible()
    expect(screen.getByRole('combobox', { name: 'Model: Claude Sonnet 5.5' })).toBeVisible()

    fireEvent.click(screen.getByRole('button', { name: 'Back to document' }))
    expect(useAppStore.getState().companionPresentation).toBe('drawer')
  })

  it('replaces the reader shell while workspace mode is active', () => {
    useAppStore.setState({ companionPresentation: 'workspace' })
    render(
      <CompanionShell>
        <main aria-label="Test document">Reader content</main>
      </CompanionShell>,
    )

    expect(screen.getByRole('region', { name: 'AI companion workspace' })).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Back to document' }))
    expect(screen.getByRole('main', { name: 'Test document' })).toBeVisible()
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
              name: 'Searched for “launch”',
              state: 'running',
              input: '{"pattern":"launch"}',
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
    expect(screen.getByRole('button', { name: /searched for “launch”/i })).toHaveAttribute(
      'aria-expanded',
      'false',
    )
    expect(screen.queryByText('Long private reasoning')).not.toBeInTheDocument()
  })
})
