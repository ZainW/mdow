import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import type { ReactNode } from 'react'
import { useOpenMarkdownFile } from './useOpenMarkdownFile'
import { useAppStore } from '../store/app-store'
import { createMinimalWindowApi, stubWindowApi } from '../test/stubWindowApi'

const readFile = vi.fn()

stubWindowApi(() => createMinimalWindowApi({ readFile }))

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

describe('useOpenMarkdownFile', () => {
  beforeEach(() => {
    readFile.mockReset()
    useAppStore.setState({ tabs: [], activeTabId: null })
  })

  it('opens a tab when read succeeds', async () => {
    readFile.mockResolvedValue('# Hello')
    const { result } = renderHook(() => useOpenMarkdownFile(), { wrapper })

    await result.current('/docs/readme.md')

    await waitFor(() => {
      expect(useAppStore.getState().tabs).toHaveLength(1)
    })
    expect(useAppStore.getState().tabs[0]).toMatchObject({
      path: '/docs/readme.md',
      content: '# Hello',
    })
  })

  it('opens an error tab for not-found failures', async () => {
    readFile.mockRejectedValue(new Error('not-found'))
    const { result } = renderHook(() => useOpenMarkdownFile(), { wrapper })

    await result.current('/missing.md')

    await waitFor(() => {
      expect(useAppStore.getState().tabs[0]?.error).toEqual({
        path: '/missing.md',
        type: 'not-found',
      })
    })
  })

  it('opens an error tab for permission-denied failures', async () => {
    readFile.mockRejectedValue(new Error('permission-denied'))
    const { result } = renderHook(() => useOpenMarkdownFile(), { wrapper })

    await result.current('/secret.md')

    await waitFor(() => {
      expect(useAppStore.getState().tabs[0]?.error?.type).toBe('permission-denied')
    })
  })

  it('opens an error tab for generic read failures', async () => {
    readFile.mockRejectedValue(new Error('read-error'))
    const { result } = renderHook(() => useOpenMarkdownFile(), { wrapper })

    await result.current('/broken.md')

    await waitFor(() => {
      expect(useAppStore.getState().tabs[0]?.error?.type).toBe('read-error')
    })
  })
})
