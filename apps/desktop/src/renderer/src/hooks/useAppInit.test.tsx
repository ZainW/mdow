import { renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppInit } from './useAppInit'
import { useAppStore } from '../store/app-store'
import type { AppState } from '../../../shared/types'

function appState(overrides: Partial<AppState> = {}): AppState {
  return {
    zoomLevel: 100,
    lastFolder: null,
    windowBounds: null,
    sessionTabs: [],
    sessionActiveTabPath: null,
    contentFont: 'inter',
    codeFont: 'geist-mono',
    theme: 'system',
    autoUpdateEnabled: true,
    wideMode: false,
    interfaceScale: 'compact',
    readingWidth: 'standard',
    sidebarMode: 'recents',
    companionProvider: 'auto',
    companionCustomCommand: '',
    ...overrides,
  }
}

describe('useAppInit', () => {
  beforeEach(() => {
    useAppStore.setState({
      initialized: false,
      tabs: [],
      activeTabId: null,
      openFolderPath: null,
      folderTree: [],
      folderTreeTruncated: false,
      sidebarMode: 'recents',
    })
  })

  it('initializes after restoring the active tab without waiting for inactive tabs', async () => {
    const readFile = vi.fn((path: string) => {
      if (path === '/docs/active.md') return Promise.resolve('# Active')
      return new Promise<string>(() => {
        // Keep inactive reads pending to prove they do not block first startup content.
      })
    })

    Object.defineProperty(window, 'api', {
      value: {
        getAppState: vi.fn().mockResolvedValue(
          appState({
            sessionTabs: [{ path: '/docs/inactive.md' }, { path: '/docs/active.md' }],
            sessionActiveTabPath: '/docs/active.md',
          }),
        ),
        readFile,
        readFolderTree: vi.fn(),
        saveAppState: vi.fn().mockResolvedValue(undefined),
      },
      configurable: true,
    })

    renderHook(() => useAppInit())

    await waitFor(() => expect(useAppStore.getState().initialized).toBe(true))
    const state = useAppStore.getState()
    expect(state.tabs).toHaveLength(1)
    expect(state.tabs[0].path).toBe('/docs/active.md')
    expect(state.tabs[0].content).toBe('# Active')
    expect(state.activeTabId).toBe(state.tabs[0].id)
    expect(readFile).toHaveBeenCalledWith('/docs/active.md')
  })
})
