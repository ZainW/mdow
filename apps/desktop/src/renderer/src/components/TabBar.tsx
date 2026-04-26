import { useEffect, useRef, useState } from 'react'
import { useAppStore } from '../store/app-store'
import { cn } from '../lib/utils'
import { FileText, X, Pencil } from '@phosphor-icons/react'

interface ContextMenuState {
  tabId: string
  x: number
  y: number
}

export function TabBar() {
  const tabs = useAppStore((s) => s.tabs)
  const activeTabId = useAppStore((s) => s.activeTabId)
  const setActiveTab = useAppStore((s) => s.setActiveTab)
  const closeTab = useAppStore((s) => s.closeTab)
  const closeOtherTabs = useAppStore((s) => s.closeOtherTabs)
  const closeTabsToRight = useAppStore((s) => s.closeTabsToRight)
  const closeAllTabs = useAppStore((s) => s.closeAllTabs)
  const reorderTabs = useAppStore((s) => s.reorderTabs)

  const [dragIndex, setDragIndex] = useState<number | null>(null)
  const [dropIndex, setDropIndex] = useState<number | null>(null)
  const [menu, setMenu] = useState<ContextMenuState | null>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const activeTabRef = useRef<HTMLDivElement>(null)

  // Keep the active tab visible when it changes (e.g. via Cmd+1..9, cycle, programmatic switch)
  useEffect(() => {
    activeTabRef.current?.scrollIntoView({
      behavior: 'smooth',
      block: 'nearest',
      inline: 'nearest',
    })
  }, [activeTabId])

  if (tabs.length === 0) return null

  const handleClose = (tabId: string) => closeTab(tabId)

  const handleDragOver = (e: React.DragEvent, index: number) => {
    if (dragIndex === null) return
    e.preventDefault()
    e.dataTransfer.dropEffect = 'move'
    const rect = e.currentTarget.getBoundingClientRect()
    const isAfter = e.clientX - rect.left > rect.width / 2
    const target = isAfter ? index + 1 : index
    if (target !== dropIndex) setDropIndex(target)
  }

  const handleDrop = () => {
    if (dragIndex !== null && dropIndex !== null) {
      reorderTabs(dragIndex, dropIndex)
    }
    setDragIndex(null)
    setDropIndex(null)
  }

  const handleDragEnd = () => {
    setDragIndex(null)
    setDropIndex(null)
  }

  return (
    <>
      <div
        ref={containerRef}
        className="relative flex h-9 shrink-0 items-stretch gap-px overflow-x-auto border-b border-border-subtle bg-background px-1.5 scrollbar-none"
        onDragOver={(e) => {
          // Allow drop after the last tab
          if (dragIndex === null) return
          const lastTab = (e.currentTarget as HTMLDivElement).querySelector<HTMLDivElement>(
            '[data-tab]:last-of-type',
          )
          if (!lastTab) return
          const rect = lastTab.getBoundingClientRect()
          if (e.clientX > rect.right) {
            e.preventDefault()
            if (dropIndex !== tabs.length) setDropIndex(tabs.length)
          }
        }}
      >
        {tabs.map((tab, index) => {
          const isActive = tab.id === activeTabId
          const filename = tab.path.split(/[/\\]/).pop() ?? 'Untitled'
          const prevIsActive = index > 0 && tabs[index - 1].id === activeTabId
          const showSeparator = !isActive && !prevIsActive && index > 0
          const isDragging = dragIndex === index
          return (
            <div
              key={tab.id}
              ref={isActive ? activeTabRef : undefined}
              data-tab
              data-active={isActive}
              draggable
              className={cn(
                'tab group/tab relative flex h-full select-none items-center',
                isDragging && 'opacity-40',
              )}
              onDragStart={(e) => {
                setDragIndex(index)
                e.dataTransfer.effectAllowed = 'move'
                e.dataTransfer.setData('text/plain', tab.id)
              }}
              onDragOver={(e) => handleDragOver(e, index)}
              onDrop={handleDrop}
              onDragEnd={handleDragEnd}
              onContextMenu={(e) => {
                e.preventDefault()
                setMenu({ tabId: tab.id, x: e.clientX, y: e.clientY })
              }}
            >
              {showSeparator && (
                <span
                  aria-hidden
                  className="pointer-events-none absolute left-0 top-1/2 h-3.5 w-px -translate-y-1/2 bg-border-subtle"
                />
              )}
              {dropIndex === index && (
                <span
                  aria-hidden
                  className="pointer-events-none absolute left-0 top-1 bottom-1 w-0.5 -translate-x-1/2 rounded-full bg-primary"
                />
              )}
              {dropIndex === tabs.length && index === tabs.length - 1 && (
                <span
                  aria-hidden
                  className="pointer-events-none absolute right-0 top-1 bottom-1 w-0.5 translate-x-1/2 rounded-full bg-primary"
                />
              )}
              <div
                className={cn(
                  'tab-btn flex h-7 max-w-[200px] items-center gap-1.5 self-center rounded-md text-xs',
                  isActive
                    ? 'bg-card text-foreground shadow-[0_1px_0_var(--color-border-subtle),0_1px_2px_oklch(0_0_0/0.04)] ring-1 ring-border-subtle'
                    : 'text-muted-foreground',
                )}
              >
                <button
                  type="button"
                  title={tab.path}
                  aria-label={`${filename} — ${tab.path}`}
                  aria-pressed={isActive}
                  onClick={() => setActiveTab(tab.id)}
                  onMouseDown={(e) => {
                    if (e.button === 1) {
                      e.preventDefault()
                      handleClose(tab.id)
                    }
                  }}
                  className="flex min-w-0 items-center gap-1.5 px-2.5 text-inherit"
                >
                  <FileText
                    weight={isActive ? 'fill' : 'regular'}
                    className={cn(
                      'size-3.5 shrink-0',
                      isActive ? 'text-muted-foreground/80' : 'text-muted-foreground/60',
                    )}
                  />
                  {tab.mode === 'edit' && (
                    <Pencil
                      weight="regular"
                      className={cn(
                        'size-3 shrink-0 opacity-60',
                        isActive ? 'text-muted-foreground/80' : 'text-muted-foreground/60',
                      )}
                    />
                  )}
                  <span className="truncate">{filename}</span>
                </button>
                <button
                  type="button"
                  tabIndex={-1}
                  aria-label={`Close ${filename}`}
                  className={cn(
                    'tab-close-btn mr-1 flex size-4 shrink-0 items-center justify-center rounded-sm text-muted-foreground',
                    isActive ? 'opacity-50' : 'opacity-0 group-hover/tab:opacity-50',
                  )}
                  onClick={(e) => {
                    e.stopPropagation()
                    handleClose(tab.id)
                  }}
                  onMouseDown={(e) => e.stopPropagation()}
                >
                  <X className="size-3" />
                </button>
              </div>
            </div>
          )
        })}
      </div>
      {menu && (
        <TabContextMenu
          {...menu}
          tabCount={tabs.length}
          tabIndex={tabs.findIndex((t) => t.id === menu.tabId)}
          onClose={() => setMenu(null)}
          onCloseTab={() => closeTab(menu.tabId)}
          onCloseOthers={() => closeOtherTabs(menu.tabId)}
          onCloseRight={() => closeTabsToRight(menu.tabId)}
          onCloseAll={() => closeAllTabs()}
          onCopyPath={() => {
            const tab = tabs.find((t) => t.id === menu.tabId)
            if (tab) void navigator.clipboard.writeText(tab.path)
          }}
          onRevealInFolder={() => {
            const tab = tabs.find((t) => t.id === menu.tabId)
            if (tab) void window.api.showInFolder(tab.path)
          }}
        />
      )}
    </>
  )
}

interface TabContextMenuProps {
  x: number
  y: number
  tabIndex: number
  tabCount: number
  onClose: () => void
  onCloseTab: () => void
  onCloseOthers: () => void
  onCloseRight: () => void
  onCloseAll: () => void
  onCopyPath: () => void
  onRevealInFolder: () => void
}

function TabContextMenu({
  x,
  y,
  tabIndex,
  tabCount,
  onClose,
  onCloseTab,
  onCloseOthers,
  onCloseRight,
  onCloseAll,
  onCopyPath,
  onRevealInFolder,
}: TabContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target
      if (!(target instanceof Node)) return
      if (ref.current && !ref.current.contains(target)) onClose()
    }
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleKey)
    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('keydown', handleKey)
    }
  }, [onClose])

  // Keep menu within viewport
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    let nx = x
    let ny = y
    if (rect.right > window.innerWidth - 8) nx = window.innerWidth - rect.width - 8
    if (rect.bottom > window.innerHeight - 8) ny = window.innerHeight - rect.height - 8
    el.style.left = `${nx}px`
    el.style.top = `${ny}px`
  }, [x, y])

  const hasOthers = tabCount > 1
  const hasRight = tabIndex < tabCount - 1

  const item = (label: string, onClick: () => void, disabled = false, danger = false) => (
    <button
      type="button"
      disabled={disabled}
      onClick={() => {
        onClick()
        onClose()
      }}
      className={cn(
        'tab-menu-item flex w-full items-center justify-between rounded-sm px-2 py-1.5 text-left text-xs',
        disabled
          ? 'text-muted-foreground/40'
          : danger
            ? 'text-destructive hover:bg-destructive/10 hover:text-destructive'
            : 'text-foreground hover:bg-muted',
      )}
    >
      {label}
    </button>
  )

  return (
    <div
      ref={ref}
      role="menu"
      className="tab-context-menu fixed z-50 min-w-[180px] rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md"
      style={{ left: x, top: y }}
    >
      {item('Close', onCloseTab)}
      {item('Close Others', onCloseOthers, !hasOthers)}
      {item('Close to the Right', onCloseRight, !hasRight)}
      {item('Close All', onCloseAll)}
      <div className="my-1 h-px bg-border-subtle" />
      {item('Copy Path', onCopyPath)}
      {item('Reveal in Finder', onRevealInFolder)}
    </div>
  )
}
