import { describe, expect, it, beforeEach } from 'vitest'
import { useAppStore } from './app-store'

describe('app-store', () => {
  beforeEach(() => {
    // Reset store to defaults between tests
    useAppStore.setState({
      activeFile: null,
      sidebarOpen: true,
      sidebarWidth: 260,
      openFolderPath: null,
      folderTree: [],
      wideMode: false,
      commandPaletteOpen: false,
    })
  })

  describe('activeFile', () => {
    it('starts as null', () => {
      expect(useAppStore.getState().activeFile).toBeNull()
    })

    it('sets active file', () => {
      const file = { path: '/docs/readme.md', content: '# Hello' }
      useAppStore.getState().setActiveFile(file)
      expect(useAppStore.getState().activeFile).toEqual(file)
    })

    it('clears active file with null', () => {
      useAppStore.getState().setActiveFile({ path: '/a.md', content: 'hi' })
      useAppStore.getState().setActiveFile(null)
      expect(useAppStore.getState().activeFile).toBeNull()
    })

    it('updates active file content', () => {
      useAppStore.getState().setActiveFile({ path: '/a.md', content: 'old' })
      useAppStore.getState().updateActiveFileContent('new')
      expect(useAppStore.getState().activeFile).toEqual({ path: '/a.md', content: 'new' })
    })

    it('updateActiveFileContent is a no-op when no active file', () => {
      useAppStore.getState().updateActiveFileContent('new')
      expect(useAppStore.getState().activeFile).toBeNull()
    })

    it('preserves path when updating content', () => {
      useAppStore.getState().setActiveFile({ path: '/docs/file.md', content: 'v1' })
      useAppStore.getState().updateActiveFileContent('v2')
      expect(useAppStore.getState().activeFile!.path).toBe('/docs/file.md')
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
    it('toggling sidebar does not affect other state', () => {
      useAppStore.getState().setActiveFile({ path: '/a.md', content: 'hi' })
      useAppStore.getState().toggleSidebar()
      expect(useAppStore.getState().activeFile).toEqual({ path: '/a.md', content: 'hi' })
      expect(useAppStore.getState().wideMode).toBe(false)
    })

    it('setting folder does not affect active file', () => {
      useAppStore.getState().setActiveFile({ path: '/a.md', content: 'hi' })
      useAppStore.getState().setOpenFolder('/docs', [])
      expect(useAppStore.getState().activeFile).toEqual({ path: '/a.md', content: 'hi' })
    })
  })
})
