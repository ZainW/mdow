import { Button } from '@renderer/components/ui/button'
import { useAppStore } from '@renderer/store/app-store'
import { Maximize2Icon, XIcon } from 'lucide-react'
import { CompanionComposer } from './CompanionComposer'
import { CompanionMessages } from './CompanionMessages'
import { CompanionSetup } from './CompanionSetup'
import { CompanionStatus } from './CompanionStatus'

export function CompanionPanel() {
  const open = useAppStore((state) => state.companionOpen)
  const hasProvider = useAppStore((state) =>
    state.companionProviders.some((provider) => provider.status === 'available'),
  )
  const hasMessages = useAppStore((state) => state.companionMessages.length > 0)
  const setOpen = useAppStore((state) => state.setCompanionOpen)
  const setFullscreen = useAppStore((state) => state.setCompanionFullscreen)
  const showChat = hasProvider || hasMessages

  if (!open) return null

  return (
    <aside
      aria-label="AI companion"
      className="flex h-full w-80 shrink-0 flex-col gap-3 border-l border-border bg-background p-3"
    >
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <h2 className="truncate text-sm font-medium">AI companion</h2>
          <CompanionStatus />
        </div>
        <div className="flex items-center gap-1">
          <Button
            aria-label="Expand companion"
            onClick={() => setFullscreen(true)}
            size="icon-sm"
            type="button"
            variant="ghost"
          >
            <Maximize2Icon />
          </Button>
          <Button
            aria-label="Close companion"
            onClick={() => setOpen(false)}
            size="icon-sm"
            type="button"
            variant="ghost"
          >
            <XIcon />
          </Button>
        </div>
      </div>
      {showChat ? (
        <>
          <CompanionMessages />
          {hasProvider && <CompanionComposer />}
        </>
      ) : (
        <CompanionSetup />
      )}
    </aside>
  )
}
