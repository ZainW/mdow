import { describe, expect, it, beforeEach, vi } from 'vitest'

// Mock electron-store before importing the module
vi.mock('electron-store', () => {
  const storeData = new Map<string, unknown>()

  return {
    default: class MockStore {
      private defaults: Record<string, unknown>

      constructor(opts: { defaults: Record<string, unknown> }) {
        this.defaults = opts.defaults
        for (const [key, value] of Object.entries(this.defaults)) {
          if (!storeData.has(key)) {
            storeData.set(key, value)
          }
        }
      }

      get(key: string) {
        return storeData.has(key) ? storeData.get(key) : this.defaults[key]
      }

      set(key: string, value: unknown) {
        storeData.set(key, value)
      }
    },
    __storeData: storeData,
  }
})

import {
  getRecents,
  addRecent,
  getAppState,
  saveAppState,
  getWindowBounds,
  saveWindowBounds,
  getLastFolder,
  setLastFolder,
} from './store'

// Get reference to the mock's internal data for resetting
// eslint-disable-next-line typescript-eslint/no-unsafe-type-assertion -- accessing mock internals
const { __storeData: storeData } = (await import('electron-store')) as unknown as {
  __storeData: Map<string, unknown>
}

describe('store', () => {
  beforeEach(() => {
    storeData.set('recents', [])
    storeData.set('lastFolder', null)
    storeData.set('sidebarWidth', 260)
    storeData.set('windowBounds', null)
  })

  describe('recents', () => {
    it('returns empty array initially', () => {
      expect(getRecents()).toEqual([])
    })

    it('adds a recent file', () => {
      addRecent('/docs/readme.md')
      expect(getRecents()).toEqual(['/docs/readme.md'])
    })

    it('adds to the front of the list', () => {
      addRecent('/first.md')
      addRecent('/second.md')
      expect(getRecents()[0]).toBe('/second.md')
    })

    it('deduplicates entries', () => {
      addRecent('/a.md')
      addRecent('/b.md')
      addRecent('/a.md')
      const recents = getRecents()
      expect(recents).toEqual(['/a.md', '/b.md'])
    })

    it('limits to 20 entries', () => {
      for (let i = 0; i < 25; i++) {
        addRecent(`/file${i}.md`)
      }
      expect(getRecents()).toHaveLength(20)
      expect(getRecents()[0]).toBe('/file24.md')
    })
  })

  describe('appState', () => {
    it('returns default state', () => {
      const state = getAppState()
      expect(state).toEqual({
        sidebarWidth: 260,
        lastFolder: null,
        windowBounds: null,
      })
    })

    it('saves partial state', () => {
      saveAppState({ sidebarWidth: 300 })
      expect(getAppState().sidebarWidth).toBe(300)
      expect(getAppState().lastFolder).toBeNull()
    })

    it('saves multiple fields at once', () => {
      saveAppState({ sidebarWidth: 400, lastFolder: '/docs' })
      const state = getAppState()
      expect(state.sidebarWidth).toBe(400)
      expect(state.lastFolder).toBe('/docs')
    })
  })

  describe('windowBounds', () => {
    it('returns null initially', () => {
      expect(getWindowBounds()).toBeNull()
    })

    it('saves and retrieves bounds', () => {
      const bounds = { x: 100, y: 200, width: 800, height: 600 }
      saveWindowBounds(bounds)
      expect(getWindowBounds()).toEqual(bounds)
    })
  })

  describe('lastFolder', () => {
    it('returns null initially', () => {
      expect(getLastFolder()).toBeNull()
    })

    it('saves and retrieves folder path', () => {
      setLastFolder('/Users/zain/docs')
      expect(getLastFolder()).toBe('/Users/zain/docs')
    })

    it('can be cleared with null', () => {
      setLastFolder('/docs')
      setLastFolder(null)
      expect(getLastFolder()).toBeNull()
    })
  })
})
