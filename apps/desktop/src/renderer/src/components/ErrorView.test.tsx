import { fireEvent, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { ErrorView } from './ErrorView'
import { useAppStore } from '../store/app-store'
import { renderWithProviders } from '../test/renderWithProviders'
import { createMinimalWindowApi, stubWindowApi } from '../test/stubWindowApi'

const readFile = vi.fn()
const showInFolder = vi.fn()
const unwatchFile = vi.fn()

stubWindowApi(() => createMinimalWindowApi({ readFile, showInFolder, unwatchFile }))

describe('ErrorView', () => {
  beforeEach(() => {
    readFile.mockReset()
    showInFolder.mockReset()
    unwatchFile.mockReset()
    useAppStore.setState({ tabs: [], activeTabId: null })
  })

  function renderError(type: 'not-found' | 'permission-denied' | 'deleted' | 'read-error') {
    const path = '/docs/example.md'
    useAppStore.getState().openErrorTab(path, { type, path })
    const tab = useAppStore.getState().tabs[0]
    renderWithProviders(<ErrorView error={tab.error!} tabId={tab.id} />)
    return tab
  }

  it('shows the not-found copy', () => {
    renderError('not-found')
    expect(screen.getByText('File not found')).toBeInTheDocument()
    expect(screen.getByText(/wandered off/)).toBeInTheDocument()
  })

  it('shows the permission-denied copy without a show-in-folder action', () => {
    renderError('permission-denied')
    expect(screen.getByText('Access denied')).toBeInTheDocument()
    expect(screen.queryByText('Show in folder')).not.toBeInTheDocument()
  })

  it('retries reading the file and clears the error on success', async () => {
    const tab = renderError('read-error')
    readFile.mockResolvedValue('# Recovered')

    fireEvent.click(screen.getByText('Try again'))

    await waitFor(() => {
      expect(readFile).toHaveBeenCalledWith('/docs/example.md')
      expect(useAppStore.getState().tabs[0]?.error).toBeNull()
      expect(useAppStore.getState().tabs[0]?.content).toBe('# Recovered')
    })
    expect(tab.id).toBe(useAppStore.getState().activeTabId)
  })

  it('reveals the file in the folder on demand', () => {
    renderError('not-found')
    fireEvent.click(screen.getByText('Show in folder'))
    expect(showInFolder).toHaveBeenCalledWith('/docs/example.md')
  })

  it('unwatches and closes the tab', () => {
    renderError('deleted')
    fireEvent.click(screen.getByText('Close tab'))
    expect(unwatchFile).toHaveBeenCalledWith('/docs/example.md')
    expect(useAppStore.getState().tabs).toHaveLength(0)
    expect(useAppStore.getState().activeTabId).toBeNull()
  })
})
