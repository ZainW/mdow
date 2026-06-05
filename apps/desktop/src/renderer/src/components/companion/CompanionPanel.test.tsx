import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from '../../store/app-store'
import { CompanionFullscreen } from './CompanionFullscreen'
import { CompanionPanel } from './CompanionPanel'

vi.mock('../../hooks/useCompanionController', () => ({
  useCompanionController: () => ({ send: vi.fn(), cancel: vi.fn() }),
}))

describe('Companion UI', () => {
  beforeEach(() => {
    useAppStore.getState().resetCompanion()
    useAppStore.setState({ sidebarMode: 'folder' })
  })

  it('renders the right companion panel without changing left sidebar mode', () => {
    useAppStore.setState({ companionOpen: true })
    render(<CompanionPanel />)

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toBeInTheDocument()
    expect(useAppStore.getState().sidebarMode).toBe('folder')
  })

  it('shows provider setup when no provider is available', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [] })
    render(<CompanionPanel />)

    expect(screen.getByText('Connect a local companion')).toBeInTheDocument()
    expect(screen.getByText('opencode acp')).toBeInTheDocument()
    expect(screen.getByText('npx --no-install @zed-industries/codex-acp')).toBeInTheDocument()
  })

  it('expands and collapses the same conversation', () => {
    useAppStore.setState({ companionOpen: true })
    useAppStore.getState().appendCompanionMessage('user', 'Hello')
    render(
      <>
        <CompanionPanel />
        <CompanionFullscreen />
      </>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Expand companion' }))

    expect(screen.getByRole('dialog', { name: 'AI companion' })).toBeInTheDocument()
    expect(screen.getAllByText('Hello')).toHaveLength(2)
  })
})
