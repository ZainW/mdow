import { useAppStore } from '../store/app-store'
import { cn } from '../lib/utils'
import { XIcon } from 'lucide-react'

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
    <div className="flex h-9 shrink-0 items-center overflow-x-auto border-b border-border/60 bg-sidebar/30 scrollbar-none">
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId
        const filename = tab.path.split(/[/\\]/).pop() || 'Untitled'
        return (
          <button
            key={tab.id}
            type="button"
            className={cn(
              'group/tab flex h-full items-center gap-1.5 border-r border-border/40 px-3 text-xs transition-colors duration-100',
              isActive
                ? 'bg-background text-foreground'
                : 'text-muted-foreground hover:bg-background/50 hover:text-foreground',
            )}
            onClick={() => setActiveTab(tab.id)}
            onMouseDown={(e) => {
              if (e.button === 1) {
                e.preventDefault()
                handleClose(e, tab.id)
              }
            }}
          >
            <span className="max-w-[140px] truncate">{filename}</span>
            <span
              role="button"
              tabIndex={-1}
              className="flex size-4 items-center justify-center rounded-sm opacity-0 transition-opacity hover:bg-muted group-hover/tab:opacity-60"
              onClick={(e) => handleClose(e, tab.id)}
              onKeyDown={() => {}}
            >
              <XIcon className="size-3" />
            </span>
          </button>
        )
      })}
    </div>
  )
}
