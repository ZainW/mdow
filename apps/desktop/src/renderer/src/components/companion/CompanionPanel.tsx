import { Button } from '@renderer/components/ui/button'
import { getReadyCompanionProvider } from '@renderer/lib/companion-provider'
import { useAppStore } from '@renderer/store/app-store'
import { Maximize2Icon, XIcon } from 'lucide-react'
import { useCompanionController } from '../../hooks/useCompanionController'
import { CompanionComposer } from './CompanionComposer'
import { CompanionMessages } from './CompanionMessages'
import { CompanionSetup } from './CompanionSetup'
import { CompanionStatus } from './CompanionStatus'

export function CompanionPanel() {
  const companion = useCompanionController()
  const open = useAppStore((state) => state.companionOpen)
  const provider = useAppStore((state) =>
    getReadyCompanionProvider({
      provider: state.companionProvider,
      customCommand: state.companionCustomCommand,
      providers: state.companionProviders,
    }),
  )
  const error = useAppStore((state) => state.companionError)
  const setOpen = useAppStore((state) => state.setCompanionOpen)
  const setFullscreen = useAppStore((state) => state.setCompanionFullscreen)

  if (!open) return null

  return (
    <aside
      aria-label="AI companion"
      className="absolute inset-y-0 right-0 z-40 flex w-80 max-w-[calc(100vw-1rem)] flex-col gap-3 border-l border-border bg-background p-3 shadow-xl lg:static lg:z-auto lg:h-full lg:w-80 lg:max-w-none lg:shrink-0 lg:shadow-none"
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
      {error && (
        <div
          className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-destructive text-xs"
          role="alert"
        >
          {error}
        </div>
      )}
      {provider ? (
        <>
          <CompanionMessages />
          <CompanionComposer cancel={companion.cancel} send={companion.send} />
        </>
      ) : (
        <CompanionSetup />
      )}
    </aside>
  )
}
