import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from './app-store'

beforeEach(() => {
  Object.defineProperty(window, 'api', {
    value: {
      saveAppState: vi.fn().mockResolvedValue(undefined),
      unwatchFile: vi.fn().mockResolvedValue(undefined),
    },
    configurable: true,
  })
  useAppStore.setState({
    tabs: [],
    activeTabId: null,
  })
})

describe('app-store tab performance', () => {
  it('opens many files within a single-frame-scale budget', () => {
    const startedAt = performance.now()

    for (let i = 0; i < 500; i++) {
      useAppStore.getState().openTab({
        path: `/docs/file-${i}.md`,
        content: `# File ${i}\n\ncontent`,
      })
    }

    expect(performance.now() - startedAt).toBeLessThan(200)
    expect(useAppStore.getState().tabs).toHaveLength(500)
  })

  it('focuses an already-open file without adding tab churn', () => {
    for (let i = 0; i < 300; i++) {
      useAppStore.getState().openTab({
        path: `/docs/file-${i}.md`,
        content: `# File ${i}`,
      })
    }

    const startedAt = performance.now()
    useAppStore.getState().openTab({
      path: '/docs/file-12.md',
      content: '# File 12\n\nupdated',
    })

    const state = useAppStore.getState()
    expect(performance.now() - startedAt).toBeLessThan(20)
    expect(state.tabs).toHaveLength(300)
    expect(state.tabs[12].content).toContain('updated')
    expect(state.activeTabId).toBe(state.tabs[12].id)
  })
})
