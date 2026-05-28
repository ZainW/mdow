import { describe, expect, it, beforeEach } from 'vitest'
import { useAppStore } from '../store/app-store'
import { DEV_WORKSPACE_PATHS, openDevWorkspace } from './open-dev-workspace'

describe('openDevWorkspace', () => {
  beforeEach(() => {
    useAppStore.setState({
      tabs: [],
      activeTabId: null,
    })
  })

  it('opens showcase and diff tabs with in-memory content', () => {
    openDevWorkspace()

    const { tabs, activeTabId } = useAppStore.getState()
    expect(tabs).toHaveLength(2)
    expect(tabs.map((tab) => tab.path)).toEqual([...DEV_WORKSPACE_PATHS])
    expect(tabs.every((tab) => tab.content.length > 0)).toBe(true)
    expect(activeTabId).toBe(tabs[1]?.id)
  })
})
