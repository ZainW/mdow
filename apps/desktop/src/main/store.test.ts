import { describe, expect, it, beforeEach, vi } from 'vitest'

vi.mock('fs', () => ({
  existsSync: vi.fn(() => true),
}))

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
import { existsSync } from 'fs'

// Get reference to the mock's internal data for resetting
// eslint-disable-next-line typescript-eslint/no-unsafe-type-assertion -- accessing mock internals
const { __storeData: storeData } = (await import('electron-store')) as unknown as {
  __storeData: Map<string, unknown>
}

describe('store', () => {
  beforeEach(() => {
    storeData.set('recents', [])
    storeData.set('lastFolder', null)
    storeData.set('zoomLevel', 100)
    storeData.set('windowBounds', null)
    storeData.set('sessionTabs', [])
    storeData.set('sessionActiveTabPath', null)
    storeData.set('contentFont', 'inter')
    storeData.set('codeFont', 'geist-mono')
    // Legacy typography overrides should no longer be surfaced as app settings.
    storeData.set('fontSize', 15.5)
    storeData.set('lineHeight', 1.65)
    storeData.set('theme', 'system')
    storeData.set('autoUpdateEnabled', true)
    storeData.set('wideMode', false)
    storeData.set('interfaceScale', 'compact')
    storeData.set('readingWidth', 'standard')
    storeData.set('sidebarMode', 'recents')
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

    it('prunes non-existent files on getRecents', () => {
      vi.mocked(existsSync).mockImplementation((path) => path === '/exists.md')
      addRecent('/exists.md')
      addRecent('/missing.md')
      expect(getRecents()).toEqual(['/exists.md'])
    })
  })

  describe('appState', () => {
    it('returns default state', () => {
      const state = getAppState()
      expect(state).toEqual({
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
      })
    })

    it('returns default companion settings with app state', () => {
      const state = getAppState()

      expect(state.companionProvider).toBe('auto')
      expect(state.companionCustomCommand).toBe('')
    })

    it('saves partial state', () => {
      saveAppState({ wideMode: true })
      expect(getAppState().wideMode).toBe(true)
      expect(getAppState().lastFolder).toBeNull()
    })

    it('saves multiple fields at once', () => {
      saveAppState({ wideMode: true, lastFolder: '/docs' })
      const state = getAppState()
      expect(state.wideMode).toBe(true)
      expect(state.lastFolder).toBe('/docs')
    })

    it('saves display preference fields', () => {
      saveAppState({ interfaceScale: 'large', readingWidth: 'comfortable' })
      const state = getAppState()
      expect(state.interfaceScale).toBe('large')
      expect(state.readingWidth).toBe('comfortable')
    })

    it('persists companion provider and custom command through saveAppState', () => {
      saveAppState({
        companionProvider: 'custom',
        companionCustomCommand: '/usr/local/bin/custom-acp --stdio',
      })

      expect(getAppState().companionProvider).toBe('custom')
      expect(getAppState().companionCustomCommand).toBe('/usr/local/bin/custom-acp --stdio')
    })
  })

  describe('windowBounds', () => {
    it('returns null initially', () => {
      expect(getWindowBounds()).toBeNull()
    })

    it('saves and retrieves bounds', () => {
      const bounds = { x: 100, y: 200, width: 800, height: 600 }
      saveWindowBounds(bounds)
      expect(getWindowBounds()).toEqual({ ...bounds, isMaximized: false })
    })

    it('saves isMaximized state', () => {
      const bounds = { x: 100, y: 200, width: 800, height: 600 }
      saveWindowBounds(bounds, true)
      expect(getWindowBounds()?.isMaximized).toBe(true)
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
