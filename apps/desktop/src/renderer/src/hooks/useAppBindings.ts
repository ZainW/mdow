import { useEffect, useEffectEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'

export function useAppMenuBindings(): void {
  const queryClient = useQueryClient()
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const updateTabContent = useAppStore((s) => s.updateTabContent)
  const setTabError = useAppStore((s) => s.setTabError)
  const setSearchOpen = useAppStore((s) => s.setSearchOpen)
  const toggleSidebar = useAppStore((s) => s.toggleSidebar)
  const zoomIn = useAppStore((s) => s.zoomIn)
  const zoomOut = useAppStore((s) => s.zoomOut)
  const resetZoom = useAppStore((s) => s.resetZoom)
  const setShortcutsDialogOpen = useAppStore((s) => s.setShortcutsDialogOpen)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
  const closeTab = useAppStore((s) => s.closeTab)

  const onMenuOpenFile = useEffectEvent(() => {
    void window.api.openFileDialog().then((result) => {
      if (result) {
        openTab(result)
        void queryClient.invalidateQueries({ queryKey: ['recents'] })
      }
    })
  })

  const onMenuOpenFolder = useEffectEvent(() => {
    void window.api.openFolderDialog().then((result) => {
      if (result) {
        setOpenFolder(result.path, result.tree, result.truncated)
      }
    })
  })

  const onFileOpened = useEffectEvent((file: { path: string; content: string }) => {
    openTab(file)
    void queryClient.invalidateQueries({ queryKey: ['recents'] })
  })

  const onFileChanged = useEffectEvent((data: { path: string; content: string }) => {
    updateTabContent(data.path, data.content)
  })

  const onFileDeleted = useEffectEvent((path: string) => {
    setTabError(path, { type: 'deleted', path })
  })

  const onMenuFind = useEffectEvent(() => setSearchOpen(true))
  const onMenuToggleSidebar = useEffectEvent(() => toggleSidebar())
  const onMenuZoomIn = useEffectEvent(() => zoomIn())
  const onMenuZoomOut = useEffectEvent(() => zoomOut())
  const onMenuZoomReset = useEffectEvent(() => resetZoom())
  const onMenuShortcuts = useEffectEvent(() => setShortcutsDialogOpen(true))
  const onMenuSettings = useEffectEvent(() => setSettingsOpen(true))

  const onMenuCloseTab = useEffectEvent(() => {
    const state = useAppStore.getState()
    if (state.activeTabId) closeTab(state.activeTabId)
    else void window.api.closeWindow()
  })

  useEffect(() => {
    const unsubs = [
      window.api.onMenuOpenFile(() => onMenuOpenFile()),
      window.api.onMenuOpenFolder(() => onMenuOpenFolder()),
      window.api.onFileOpened((file) => onFileOpened(file)),
      window.api.onFileChanged((data) => onFileChanged(data)),
      window.api.onFileDeleted((path) => onFileDeleted(path)),
      window.api.onMenuFind(() => onMenuFind()),
      window.api.onMenuToggleSidebar(() => onMenuToggleSidebar()),
      window.api.onMenuZoomIn(() => onMenuZoomIn()),
      window.api.onMenuZoomOut(() => onMenuZoomOut()),
      window.api.onMenuZoomReset(() => onMenuZoomReset()),
      window.api.onMenuShortcuts(() => onMenuShortcuts()),
      window.api.onMenuSettings(() => onMenuSettings()),
      window.api.onMenuCloseTab(() => onMenuCloseTab()),
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [])
}

export function useAppKeyboardShortcuts(): void {
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const setSearchOpen = useAppStore((s) => s.setSearchOpen)
  const toggleSidebar = useAppStore((s) => s.toggleSidebar)
  const zoomIn = useAppStore((s) => s.zoomIn)
  const zoomOut = useAppStore((s) => s.zoomOut)
  const resetZoom = useAppStore((s) => s.resetZoom)
  const setShortcutsDialogOpen = useAppStore((s) => s.setShortcutsDialogOpen)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
  const cycleTab = useAppStore((s) => s.cycleTab)
  const selectTabByIndex = useAppStore((s) => s.selectTabByIndex)

  const onKeyDown = useEffectEvent((e: KeyboardEvent) => {
    const mod = e.metaKey || e.ctrlKey
    if (mod && e.key === 'k') {
      e.preventDefault()
      setCommandPaletteOpen(true)
    }
    if (mod && e.key === 'b') {
      e.preventDefault()
      toggleSidebar()
    }
    if (mod && e.key === 'f') {
      e.preventDefault()
      setSearchOpen(true)
    }
    if (mod && (e.key === '=' || e.key === '+')) {
      e.preventDefault()
      zoomIn()
    }
    if (mod && e.key === '-') {
      e.preventDefault()
      zoomOut()
    }
    if (mod && e.key === '0') {
      e.preventDefault()
      resetZoom()
    }
    if (mod && e.key === '/') {
      e.preventDefault()
      setShortcutsDialogOpen(true)
    }
    if (mod && e.key === ',') {
      e.preventDefault()
      setSettingsOpen(true)
    }
    if (mod && e.altKey && e.key === 'ArrowRight') {
      e.preventDefault()
      cycleTab(1)
    }
    if (mod && e.altKey && e.key === 'ArrowLeft') {
      e.preventDefault()
      cycleTab(-1)
    }
    if (mod && !e.altKey && !e.shiftKey && /^[1-9]$/.test(e.key)) {
      const tabs = useAppStore.getState().tabs
      if (tabs.length === 0) return
      e.preventDefault()
      const n = Number(e.key)
      if (n === 9) selectTabByIndex(tabs.length - 1)
      else selectTabByIndex(n - 1)
    }
  })

  useEffect(() => {
    const handler = (e: KeyboardEvent) => onKeyDown(e)
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [])
}
