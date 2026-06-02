import { useEffect, useRef, useState } from 'react'
import { useAppStore } from '../store/app-store'
import { cn, isMac } from '../lib/utils'
import { FileText, X, AlertCircle } from 'lucide-react'
import { iconStroke } from '../lib/icons'
import { rovingTabIndex, useRovingFocus } from '../hooks/useRovingFocus'
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuTrigger,
} from './ui/context-menu'

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
  const activeTabRef = useRef<HTMLDivElement>(null)
  const tablistRoving = useRovingFocus({ orientation: 'horizontal' })

  useEffect(() => {
    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    activeTabRef.current?.scrollIntoView({
      behavior: reduceMotion ? 'auto' : 'smooth',
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

  const mod = isMac ? '⌘' : 'Ctrl'
  const revealLabel = isMac ? 'Reveal in Finder' : 'Show in Folder'

  return (
    // oxlint-disable-next-line jsx-a11y/interactive-supports-focus -- per WAI-ARIA, focus rests on the active tab inside, not the tablist itself
    <div
      ref={tablistRoving.containerRef}
      role="tablist"
      aria-label="Open documents"
      onKeyDown={tablistRoving.onKeyDown}
      className="relative flex h-(--tabbar-height) shrink-0 items-stretch gap-px overflow-x-auto border-b border-border-subtle bg-background px-1.5 scrollbar-none"
      onDragOver={(e) => {
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
        const hasOthers = tabs.length > 1
        const hasRight = index < tabs.length - 1

        return (
          <ContextMenu key={tab.id}>
            <ContextMenuTrigger className="contents">
              <div
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
                    'tab-btn flex h-(--tab-height) max-w-(--tab-max-width) items-center gap-1.5 self-center rounded-md text-[length:var(--control-font-size)]',
                    isActive
                      ? 'bg-card text-foreground shadow-[0_1px_0_var(--color-border-subtle),0_1px_2px_oklch(0_0_0/0.04)] ring-1 ring-border-subtle'
                      : 'text-muted-foreground',
                  )}
                >
                  <button
                    type="button"
                    role="tab"
                    title={tab.path}
                    aria-label={`${filename}${tab.error ? ' — error' : ''} — ${tab.path}`}
                    aria-selected={isActive}
                    aria-controls={`tabpanel-${tab.id}`}
                    aria-setsize={tabs.length}
                    aria-posinset={index + 1}
                    tabIndex={rovingTabIndex(isActive)}
                    onClick={() => setActiveTab(tab.id)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault()
                        setActiveTab(tab.id)
                      }
                    }}
                    onMouseDown={(e) => {
                      if (e.button === 1) {
                        e.preventDefault()
                        handleClose(tab.id)
                      }
                    }}
                    className="flex min-w-0 items-center gap-1.5 px-2.5 text-inherit"
                  >
                    {tab.error ? (
                      <AlertCircle
                        className="size-(--tab-icon-size) shrink-0 text-destructive"
                        aria-hidden
                      />
                    ) : (
                      <FileText
                        className={cn(
                          'size-(--tab-icon-size) shrink-0',
                          isActive ? 'text-muted-foreground/80' : 'text-muted-foreground/60',
                        )}
                        strokeWidth={iconStroke.default}
                      />
                    )}
                    <span className="truncate">{filename}</span>
                  </button>
                  <button
                    type="button"
                    tabIndex={-1}
                    aria-label={`Close ${filename}`}
                    className={cn(
                      'tab-close-btn mr-1 flex size-(--tab-close-size) shrink-0 items-center justify-center rounded-sm text-muted-foreground',
                      isActive ? 'opacity-50' : 'opacity-0 group-hover/tab:opacity-50',
                    )}
                    onClick={(e) => {
                      e.stopPropagation()
                      handleClose(tab.id)
                    }}
                    onMouseDown={(e) => e.stopPropagation()}
                  >
                    <X className="size-(--button-sm-icon-size)" strokeWidth={iconStroke.emphasis} />
                  </button>
                </div>
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent className="min-w-[200px]">
              <ContextMenuItem onClick={() => closeTab(tab.id)}>
                Close
                <ContextMenuShortcut>{mod} W</ContextMenuShortcut>
              </ContextMenuItem>
              <ContextMenuItem disabled={!hasOthers} onClick={() => closeOtherTabs(tab.id)}>
                Close Others
              </ContextMenuItem>
              <ContextMenuItem disabled={!hasRight} onClick={() => closeTabsToRight(tab.id)}>
                Close to the Right
              </ContextMenuItem>
              <ContextMenuItem onClick={() => closeAllTabs()}>Close All</ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onClick={() => void navigator.clipboard.writeText(tab.path)}>
                Copy Path
              </ContextMenuItem>
              <ContextMenuItem onClick={() => void window.api.showInFolder(tab.path)}>
                {revealLabel}
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        )
      })}
    </div>
  )
}
