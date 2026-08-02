import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { stubWindowApi } from '../../test/stubWindowApi'
import { useAppStore } from '../../store/app-store'
import { CompanionPanel, CompanionShell, CompanionWorkspace } from './CompanionPanel'

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
    .mockResolvedValue({ preferredProvider: 'opencode', customCommand: '', lastModel: null }),
  saveCompanionSettings: vi.fn().mockResolvedValue(undefined),
  startCompanionSession: vi.fn().mockResolvedValue({ ok: true, providerId: 'opencode' }),
  getCompanionModels: vi.fn().mockResolvedValue({
    options: [{ value: 'openai/gpt-5.4', name: 'GPT-5.4', provider: 'openai' }],
    currentValue: 'openai/gpt-5.4',
    stale: false,
  }),
  setCompanionModel: vi.fn().mockResolvedValue({
    options: [{ value: 'openai/gpt-5.4', name: 'GPT-5.4', provider: 'openai' }],
    currentValue: 'openai/gpt-5.4',
    stale: false,
  }),
  sendCompanionMessage,
  cancelCompanion: vi.fn().mockResolvedValue(undefined),
}))

beforeEach(() => {
  sendCompanionMessage.mockClear()
  useAppStore.getState().resetCompanionConversation()
  useAppStore.setState({
    companionPresentation: 'drawer',
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
    companionModelState: {
      options: [{ value: 'openai/gpt-5.4', name: 'GPT-5.4', provider: 'openai' }],
      currentValue: 'openai/gpt-5.4',
      stale: false,
    },
    companionContextTrace: null,
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

  it('sends with an available provider when the saved provider is missing', async () => {
    useAppStore.setState({
      companionProviders: [
        {
          id: 'opencode',
          label: 'OpenCode',
          commandDisplay: 'opencode acp',
          availability: 'available',
        },
        {
          id: 'codex-acp',
          label: 'Codex ACP',
          commandDisplay: 'codex-acp',
          availability: 'missing',
        },
      ],
      companionPreferredProvider: 'codex-acp',
    })
    vi.mocked(window.api.detectCompanionProviders).mockResolvedValueOnce(
      useAppStore.getState().companionProviders,
    )
    vi.mocked(window.api.getCompanionSettings).mockResolvedValueOnce({
      preferredProvider: 'codex-acp',
      customCommand: '',
      lastModel: null,
    })

    render(<CompanionPanel />)
    await waitFor(() => {
      expect(useAppStore.getState().companionPreferredProvider).toBe('opencode')
    })
    const composer = screen.getByRole('textbox', { name: 'Ask about these docs' })
    fireEvent.change(composer, { target: { value: 'Use the working provider' } })
    fireEvent.keyDown(composer, { key: 'Enter' })

    expect(sendCompanionMessage).toHaveBeenCalledWith(
      expect.objectContaining({ providerId: 'opencode' }),
    )
  })

  it('uses an overlay on narrow windows instead of shrinking the document', () => {
    render(<CompanionPanel />)

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toHaveClass(
      'max-lg:fixed',
      'lg:relative',
    )
  })

  it('renders the expanded companion as a workspace instead of a dialog', () => {
    useAppStore.setState({ companionPresentation: 'workspace' })
    render(<CompanionWorkspace />)

    expect(screen.getByRole('region', { name: 'AI companion workspace' })).toBeVisible()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Back to document' }))
    expect(useAppStore.getState().companionPresentation).toBe('drawer')
  })

  it('integrates the model picker and compact context bar in workspace mode', () => {
    useAppStore.setState({
      companionPresentation: 'workspace',
      companionContextTrace: {
        focusedCount: 1,
        attachedCount: 0,
        searchedCount: 0,
        readRangeCount: 0,
        injectedBytes: 800,
        estimatedTokens: 200,
        retrievalMode: 'focused-only',
        items: [{ path: '/docs/overview.md', reason: 'focused', bytes: 800 }],
      },
    })

    render(<CompanionWorkspace />)

    expect(screen.getByRole('combobox', { name: 'Model' })).toBeVisible()
    expect(screen.getByText(/1 focused/)).toBeVisible()
    expect(screen.getByText(/≈200 added/)).toBeVisible()
    expect(screen.queryByText(/using .*more/i)).not.toBeInTheDocument()
  })

  it('replaces the reader shell while workspace mode is active', () => {
    useAppStore.setState({ companionPresentation: 'workspace' })
    render(
      <CompanionShell>
        <main aria-label="Test document">Reader content</main>
      </CompanionShell>,
    )

    expect(screen.getByRole('region', { name: 'AI companion workspace' })).toBeVisible()
    expect(screen.queryByRole('main', { name: 'Test document' })).not.toBeInTheDocument()

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
