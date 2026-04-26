import { describe, expect, it, beforeEach } from 'vitest'
import { useAppStore, selectActiveTab } from './app-store'

describe('app-store', () => {
  beforeEach(() => {
    useAppStore.setState({
      tabs: [],
      activeTabId: null,
      sidebarOpen: true,
      sidebarWidth: 260,
      openFolderPath: null,
      folderTree: [],
      wideMode: false,
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

    it('inserts new tab after active tab', () => {
      useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
      useAppStore.getState().openTab({ path: '/b.md', content: 'b' })
      // Active is now /b.md (index 1). Switch to /a.md, then open /c.md
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

    it('starts with default width of 260', () => {
      expect(useAppStore.getState().sidebarWidth).toBe(260)
    })

    it('sets sidebar width', () => {
      useAppStore.getState().setSidebarWidth(300)
      expect(useAppStore.getState().sidebarWidth).toBe(300)
    })
  })

  describe('folder', () => {
    it('starts with no open folder', () => {
      expect(useAppStore.getState().openFolderPath).toBeNull()
      expect(useAppStore.getState().folderTree).toEqual([])
    })

    it('sets open folder with path and tree', () => {
      const tree = [{ name: 'readme.md', path: '/docs/readme.md', isDirectory: false }]
      useAppStore.getState().setOpenFolder('/docs', tree)
      expect(useAppStore.getState().openFolderPath).toBe('/docs')
      expect(useAppStore.getState().folderTree).toEqual(tree)
    })

    it('sets folder tree independently', () => {
      const tree1 = [{ name: 'a.md', path: '/a.md', isDirectory: false }]
      const tree2 = [{ name: 'b.md', path: '/b.md', isDirectory: false }]
      useAppStore.getState().setOpenFolder('/docs', tree1)
      useAppStore.getState().setFolderTree(tree2)
      expect(useAppStore.getState().openFolderPath).toBe('/docs')
      expect(useAppStore.getState().folderTree).toEqual(tree2)
    })
  })

  describe('wideMode', () => {
    it('starts as false', () => {
      expect(useAppStore.getState().wideMode).toBe(false)
    })

    it('toggles wide mode', () => {
      useAppStore.getState().toggleWideMode()
      expect(useAppStore.getState().wideMode).toBe(true)
      useAppStore.getState().toggleWideMode()
      expect(useAppStore.getState().wideMode).toBe(false)
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
      useAppStore.getState().setOpenFolder('/docs', [])
      expect(useAppStore.getState().tabs).toHaveLength(1)
    })
  })

  describe('tab mode', () => {
    it('new tabs default to read mode', () => {
      useAppStore.getState().openTab({ path: '/x.md', content: 'a' })
      const tab = useAppStore.getState().tabs[0]
      expect(tab.mode).toBe('read')
    })

    it('toggleTabMode flips mode', () => {
      useAppStore.getState().openTab({ path: '/y.md', content: 'a' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().toggleTabMode(id)
      expect(useAppStore.getState().tabs[0].mode).toBe('edit')
      useAppStore.getState().toggleTabMode(id)
      expect(useAppStore.getState().tabs[0].mode).toBe('read')
    })

    it('setTabMode sets a specific mode', () => {
      useAppStore.getState().openTab({ path: '/z.md', content: 'a' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().setTabMode(id, 'edit')
      expect(useAppStore.getState().tabs[0].mode).toBe('edit')
    })

    it('markTabWritten records a timestamp by path', () => {
      useAppStore.getState().openTab({ path: '/w.md', content: 'a' })
      useAppStore.getState().markTabWritten('/w.md', 12345)
      expect(useAppStore.getState().tabs[0].lastDiskWriteAt).toBe(12345)
    })
  })

  describe('tab conflicts', () => {
    beforeEach(() => {
      useAppStore.setState({ tabs: [], activeTabId: null, tabConflicts: {} })
    })

    it('setTabConflict stores disk content by tab id', () => {
      useAppStore.getState().setTabConflict('t1', 'disk text')
      expect(useAppStore.getState().tabConflicts.t1).toBe('disk text')
    })

    it('setTabConflict with null clears the entry', () => {
      useAppStore.getState().setTabConflict('t1', 'x')
      useAppStore.getState().setTabConflict('t1', null)
      expect(useAppStore.getState().tabConflicts.t1).toBeUndefined()
    })

    it('closeTab clears conflict for that tab', () => {
      useAppStore.getState().openTab({ path: '/x.md', content: 'a' })
      const id = useAppStore.getState().tabs[0].id
      useAppStore.getState().setTabConflict(id, 'disk text')
      useAppStore.getState().closeTab(id)
      expect(useAppStore.getState().tabConflicts[id]).toBeUndefined()
    })
  })
})
