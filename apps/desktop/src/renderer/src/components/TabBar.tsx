import { useAppStore } from '../store/app-store'
import { cn } from '../lib/utils'
import { X } from '@phosphor-icons/react'

export function TabBar() {
  const tabs = useAppStore((s) => s.tabs)
  const activeTabId = useAppStore((s) => s.activeTabId)
  const setActiveTab = useAppStore((s) => s.setActiveTab)
  const closeTab = useAppStore((s) => s.closeTab)

  if (tabs.length === 0) return null

  const handleClose = (e: React.MouseEvent, tabId: string) => {
    e.stopPropagation()
    const tab = tabs.find((t) => t.id === tabId)
    if (tab) {
      void window.api.unwatchFile(tab.path)
    }
    closeTab(tabId)
  }

  return (
    <div className="flex h-9 shrink-0 items-center overflow-x-auto border-b border-border-subtle bg-background scrollbar-none">
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId
        const filename = tab.path.split(/[/\\]/).pop() || 'Untitled'
        return (
          <button
            key={tab.id}
            type="button"
            className={cn(
              'tab-btn group/tab relative flex h-full items-center gap-1.5 px-3 text-xs',
              isActive ? 'text-foreground font-medium' : 'text-muted-foreground',
            )}
            onClick={() => setActiveTab(tab.id)}
            onMouseDown={(e) => {
              if (e.button === 1) {
                e.preventDefault()
                handleClose(e, tab.id)
              }
            }}
          >
            {isActive && (
              <span className="tab-active-indicator absolute inset-x-0 bottom-0 h-[2px] bg-foreground/20" />
            )}
            <span className="max-w-[140px] truncate">{filename}</span>
            <button
              type="button"
              tabIndex={-1}
              className="tab-close-btn flex size-4 items-center justify-center rounded-sm opacity-0 group-hover/tab:opacity-60"
              onClick={(e) => handleClose(e, tab.id)}
            >
              <X className="size-3" />
            </button>
          </button>
        )
      })}
    </div>
  )
}
