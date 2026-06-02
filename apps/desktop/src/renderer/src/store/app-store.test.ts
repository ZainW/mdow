import { describe, expect, it, beforeEach, vi } from 'vitest'
import type { RenderResult } from '../lib/markdown'
import { useAppStore, selectActiveTab } from './app-store'

describe('app-store', () => {
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
      openingPath: null,
      renderCache: new Map(),
      sidebarOpen: true,
      sidebarMode: 'recents',
      openFolderPath: null,
      folderTree: [],
      wideMode: false,
      interfaceScale: 'compact',
      readingWidth: 'standard',
      commandPaletteOpen: false,
    })
  })

  describe('tabs', () => {
    it('starts with no tabs', () => {
      expect(useAppStore.getState().tabs).toEqual([])
      expect(useAppStore.getState().activeTabId).toBeNull()
    })

    it('opens a tab', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: '# Hello' })
      const state = useAppStore.getState()
      expect(state.tabs).toHaveLength(1)
      expect(state.tabs[0].path).toBe('/a.md')
      expect(state.tabs[0].content).toBe('# Hello')
      expect(state.activeTabId).toBe(state.tabs[0].id)
    })

    it('deduplicates by path — focuses existing tab', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'v1' })
      useAppStore.getState().openTab({ path: '/b.md', content: 'b' })
      useAppStore.getState().openTab({ path: '/a.md', content: 'v2' })
      const state = useAppStore.getState()
      expect(state.tabs).toHaveLength(2)
      expect(state.tabs[0].content).toBe('v2')
      expect(state.activeTabId).toBe(state.tabs[0].id)
    })

    it('clears render cache when reopening an existing tab with new content', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'v1' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().setRenderCache(id, {
        tree: {} as RenderResult['tree'],
        mermaidBlocks: [],
        headings: [],
        frontmatter: {},
      })
      useAppStore.getState().openTab({ path: '/a.md', content: 'v2' })
      expect(useAppStore.getState().renderCache.has(id)).toBe(false)
    })

    it('inserts new tab after active tab', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      useAppStore.getState().openTab({ path: '/b.md', content: 'b' })
      const aId = useAppStore.getState().tabs[0].id
      useAppStore.getState().setActiveTab(aId)
      useAppStore.getState().openTab({ path: '/c.md', content: 'c' })
      const paths = useAppStore.getState().tabs.map((t) => t.path)
      expect(paths).toEqual(['/a.md', '/c.md', '/b.md'])
    })

    it('closes a tab', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      useAppStore.getState().openTab({ path: '/b.md', content: 'b' })
      const bId = useAppStore.getState().tabs[1].id
      useAppStore.getState().closeTab(bId)
      expect(useAppStore.getState().tabs).toHaveLength(1)
      expect(useAppStore.getState().tabs[0].path).toBe('/a.md')
    })

    it('closing active tab activates next tab to right', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      useAppStore.getState().openTab({ path: '/b.md', content: 'b' })
      useAppStore.getState().openTab({ path: '/c.md', content: 'c' })
      const aId = useAppStore.getState().tabs[0].id
      useAppStore.getState().setActiveTab(aId)
      useAppStore.getState().closeTab(aId)
      expect(useAppStore.getState().activeTabId).toBe(useAppStore.getState().tabs[0].id)
      expect(useAppStore.getState().tabs[0].path).toBe('/b.md')
    })

    it('closing last tab results in null activeTabId', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().closeTab(id)
      expect(useAppStore.getState().tabs).toEqual([])
      expect(useAppStore.getState().activeTabId).toBeNull()
    })

    it('updates tab content by path', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'old' })
      useAppStore.getState().updateTabContent('/a.md', 'new')
      expect(useAppStore.getState().tabs[0].content).toBe('new')
    })

    it('updates tab scroll position', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().updateTabScroll(id, 500)
      expect(useAppStore.getState().tabs[0].scrollPosition).toBe(500)
    })

    it('does not recreate a tab for unchanged scroll position', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().updateTabScroll(id, 500)
      const tab = useAppStore.getState().tabs[0]
      useAppStore.getState().updateTabScroll(id, 500)
      expect(useAppStore.getState().tabs[0]).toBe(tab)
    })

    it('opens an error tab for failed reads', () => {
      useAppStore.getState().openErrorTab('/missing.md', { type: 'not-found', path: '/missing.md' })
      const state = useAppStore.getState()
      expect(state.tabs).toHaveLength(1)
      expect(state.tabs[0].path).toBe('/missing.md')
      expect(state.tabs[0].error).toEqual({ type: 'not-found', path: '/missing.md' })
      expect(state.activeTabId).toBe(state.tabs[0].id)
    })

    it('sets tab error by path', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      useAppStore.getState().setTabError('/a.md', { type: 'deleted', path: '/a.md' })
      expect(useAppStore.getState().tabs[0].error).toEqual({ type: 'deleted', path: '/a.md' })
    })

    it('clears tab error', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().setTabError('/a.md', { type: 'deleted', path: '/a.md' })
      useAppStore.getState().clearTabError(id)
      expect(useAppStore.getState().tabs[0].error).toBeNull()
    })

    it('clears render cache when a tab closes', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().setRenderCache(id, {
        tree: {} as RenderResult['tree'],
        mermaidBlocks: [],
        headings: [],
        frontmatter: {},
      })
      useAppStore.getState().closeTab(id)
      expect(useAppStore.getState().renderCache.has(id)).toBe(false)
    })
  })

  describe('openingPath', () => {
    it('starts as null', () => {
      expect(useAppStore.getState().openingPath).toBeNull()
    })

    it('tracks the path being opened', () => {
      useAppStore.getState().setOpeningPath('/a.md')
      expect(useAppStore.getState().openingPath).toBe('/a.md')
      useAppStore.getState().setOpeningPath(null)
      expect(useAppStore.getState().openingPath).toBeNull()
    })
  })

  describe('selectActiveTab', () => {
    it('returns null when no tabs', () => {
      expect(selectActiveTab(useAppStore.getState())).toBeNull()
    })

    it('returns the active tab', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'hello' })
      const tab = selectActiveTab(useAppStore.getState())
      expect(tab?.path).toBe('/a.md')
    })
  })

  describe('sidebar', () => {
    it('starts open', () => {
      expect(useAppStore.getState().sidebarOpen).toBe(true)
    })

    it('toggles sidebar', () => {
      useAppStore.getState().toggleSidebar()
      expect(useAppStore.getState().sidebarOpen).toBe(false)
      useAppStore.getState().toggleSidebar()
      expect(useAppStore.getState().sidebarOpen).toBe(true)
    })

    it('starts in recents mode', () => {
      expect(useAppStore.getState().sidebarMode).toBe('recents')
    })

    it('sets sidebar mode and persists it', () => {
      useAppStore.getState().setSidebarMode('outline')
      expect(useAppStore.getState().sidebarMode).toBe('outline')
      expect(window.api.saveAppState).toHaveBeenCalledWith({ sidebarMode: 'outline' })
    })
  })

  describe('folder', () => {
    it('starts with no open folder', () => {
      expect(useAppStore.getState().openFolderPath).toBeNull()
      expect(useAppStore.getState().folderTree).toEqual([])
    })

    it('sets open folder with path and tree', () => {
      const tree = [{ name: 'readme.md', path: '/docs/readme.md', isDirectory: false }]
      useAppStore.getState().setOpenFolder('/docs', tree, false)
      expect(useAppStore.getState().openFolderPath).toBe('/docs')
      expect(useAppStore.getState().folderTree).toEqual(tree)
      expect(useAppStore.getState().folderTreeTruncated).toBe(false)
    })

    it('sets folder tree independently', () => {
      const tree1 = [{ name: 'a.md', path: '/a.md', isDirectory: false }]
      const tree2 = [{ name: 'b.md', path: '/b.md', isDirectory: false }]
      useAppStore.getState().setOpenFolder('/docs', tree1, false)
      useAppStore.getState().setFolderTree(tree2, true)
      expect(useAppStore.getState().openFolderPath).toBe('/docs')
      expect(useAppStore.getState().folderTree).toEqual(tree2)
      expect(useAppStore.getState().folderTreeTruncated).toBe(true)
    })
  })

  describe('wideMode', () => {
    it('starts as false', () => {
      expect(useAppStore.getState().wideMode).toBe(false)
    })

    it('toggles wide mode and persists it', () => {
      useAppStore.getState().toggleWideMode()
      expect(useAppStore.getState().wideMode).toBe(true)
      expect(window.api.saveAppState).toHaveBeenCalledWith({ wideMode: true })
      useAppStore.getState().toggleWideMode()
      expect(useAppStore.getState().wideMode).toBe(false)
      expect(window.api.saveAppState).toHaveBeenCalledWith({ wideMode: false })
    })
  })

  describe('display preferences', () => {
    it('does not expose markdown size or spacing settings', () => {
      const state = useAppStore.getState()
      expect('fontSize' in state).toBe(false)
      expect('lineHeight' in state).toBe(false)
      expect('setFontSize' in state).toBe(false)
      expect('setLineHeight' in state).toBe(false)
    })

    it('starts with compact interface scale', () => {
      expect(useAppStore.getState().interfaceScale).toBe('compact')
    })

    it('sets interface scale and persists it', () => {
      useAppStore.getState().setInterfaceScale('comfortable')
      expect(useAppStore.getState().interfaceScale).toBe('comfortable')
      expect(window.api.saveAppState).toHaveBeenCalledWith({ interfaceScale: 'comfortable' })
    })

    it('starts with standard reading width', () => {
      expect(useAppStore.getState().readingWidth).toBe('standard')
    })

    it('sets reading width and persists it', () => {
      useAppStore.getState().setReadingWidth('comfortable')
      expect(useAppStore.getState().readingWidth).toBe('comfortable')
      expect(window.api.saveAppState).toHaveBeenCalledWith({ readingWidth: 'comfortable' })
    })
  })

  describe('commandPalette', () => {
    it('starts as closed', () => {
      expect(useAppStore.getState().commandPaletteOpen).toBe(false)
    })

    it('opens command palette', () => {
      useAppStore.getState().setCommandPaletteOpen(true)
      expect(useAppStore.getState().commandPaletteOpen).toBe(true)
    })

    it('closes command palette', () => {
      useAppStore.getState().setCommandPaletteOpen(true)
      useAppStore.getState().setCommandPaletteOpen(false)
      expect(useAppStore.getState().commandPaletteOpen).toBe(false)
    })
  })

  describe('state independence', () => {
    it('toggling sidebar does not affect tabs', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'hi' })
      useAppStore.getState().toggleSidebar()
      expect(useAppStore.getState().tabs).toHaveLength(1)
      expect(useAppStore.getState().wideMode).toBe(false)
    })

    it('setting folder does not affect tabs', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'hi' })
      useAppStore.getState().setOpenFolder('/docs', [], false)
      expect(useAppStore.getState().tabs).toHaveLength(1)
    })
  })
})
