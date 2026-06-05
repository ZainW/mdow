import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@renderer/components/ui/dialog'
import { useAppStore } from '@renderer/store/app-store'
import { CompanionComposer } from './CompanionComposer'
import { CompanionMessages } from './CompanionMessages'
import { CompanionSetup } from './CompanionSetup'
import { CompanionStatus } from './CompanionStatus'

export function CompanionFullscreen() {
  const open = useAppStore((state) => state.companionFullscreen)
  const hasProvider = useAppStore((state) =>
    state.companionProviders.some((provider) => provider.status === 'available'),
  )
  const error = useAppStore((state) => state.companionError)
  const setOpen = useAppStore((state) => state.setCompanionFullscreen)

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent
        aria-label="AI companion"
        className="flex h-[min(760px,calc(100vh-2rem))] max-w-[min(920px,calc(100vw-2rem))] flex-col gap-3 p-4 sm:max-w-[min(920px,calc(100vw-2rem))]"
      >
        <DialogHeader>
          <div className="flex items-center justify-between gap-3 pr-8">
            <div>
              <DialogTitle>AI companion</DialogTitle>
              <DialogDescription>
                Chat with local context from your Markdown workspace.
              </DialogDescription>
            </div>
            <CompanionStatus />
          </div>
        </DialogHeader>
        {error && (
          <div
            className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-destructive text-xs"
            role="alert"
          >
            {error}
          </div>
        )}
        {hasProvider ? (
          <>
            <CompanionMessages />
            <CompanionComposer />
          </>
        ) : (
          <CompanionSetup />
        )}
      </DialogContent>
    </Dialog>
  )
}
