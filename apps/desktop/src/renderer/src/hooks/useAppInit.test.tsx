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
    companionLastModel: null,
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

  it('focuses a file the OS opened at launch over the saved session', async () => {
    const readFile = vi.fn((path: string) => Promise.resolve(`# ${path}`))

    Object.defineProperty(window, 'api', {
      value: {
        getAppState: vi.fn().mockResolvedValue(
          appState({
            sessionTabs: [{ path: '/docs/a.md' }, { path: '/docs/b.md' }],
            sessionActiveTabPath: '/docs/b.md',
            launchFiles: ['/downloads/new.md'],
          }),
        ),
        readFile,
        readFolderTree: vi.fn(),
        saveAppState: vi.fn().mockResolvedValue(undefined),
      },
      configurable: true,
    })

    renderHook(() => useAppInit())

    await waitFor(() => expect(useAppStore.getState().tabs).toHaveLength(3))
    const state = useAppStore.getState()
    const active = state.tabs.find((tab) => tab.id === state.activeTabId)
    expect(active?.path).toBe('/downloads/new.md')
    expect(readFile.mock.calls[0][0]).toBe('/downloads/new.md')
  })

  it('does not steal focus back after the reader switches tabs during restore', async () => {
    let releaseInactive: (content: string) => void = () => {}
    const readFile = vi.fn((path: string) =>
      path === '/docs/inactive.md'
        ? new Promise<string>((resolve) => {
            releaseInactive = resolve
          })
        : Promise.resolve(`# ${path}`),
    )

    Object.defineProperty(window, 'api', {
      value: {
        getAppState: vi.fn().mockResolvedValue(
          appState({
            sessionTabs: [{ path: '/docs/active.md' }, { path: '/docs/inactive.md' }],
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

    useAppStore.getState().openTab({ path: '/finder/opened.md', content: '# Opened' })
    releaseInactive('# Inactive')

    await waitFor(() => expect(useAppStore.getState().tabs).toHaveLength(3))
    await Promise.resolve()
    const state = useAppStore.getState()
    const active = state.tabs.find((tab) => tab.id === state.activeTabId)
    expect(active?.path).toBe('/finder/opened.md')
  })
})
