import { Button } from './ui/button'
import { useAppStore } from '../store/app-store'

interface ConflictBannerProps {
  tabId: string
  diskContent: string
}

export function ConflictBanner({ tabId, diskContent }: ConflictBannerProps) {
  const updateTabContent = useAppStore((s) => s.updateTabContent)
  const setTabConflict = useAppStore((s) => s.setTabConflict)
  const tab = useAppStore((s) => s.tabs.find((t) => t.id === tabId))

  if (!tab) return null

  const reload = () => {
    updateTabContent(tab.path, diskContent)
    setTabConflict(tabId, null)
  }
  const keep = () => {
    setTabConflict(tabId, null)
    void window.api.writeFile(tab.path, tab.content)
  }

  return (
    <div className="border-b bg-yellow-50 dark:bg-yellow-900/20 px-4 py-2 text-sm flex items-center justify-between">
      <span>This file was changed on disk.</span>
      <div className="flex gap-2">
        <Button size="sm" variant="ghost" onClick={keep}>
          Keep my version
        </Button>
        <Button size="sm" onClick={reload}>
          Reload
        </Button>
      </div>
    </div>
  )
}
