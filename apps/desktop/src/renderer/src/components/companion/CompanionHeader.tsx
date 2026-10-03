import { ArrowLeft, Expand, SquarePen, X } from 'lucide-react'
import { cn } from '@renderer/lib/utils'
import { useAppStore } from '../../store/app-store'
import { Button } from '../ui/button'

export function CompanionHeader({
  layout,
  onBack,
  onExpand,
  onClose,
}: {
  layout: 'drawer' | 'workspace'
  onBack?: () => void
  onExpand?: () => void
  onClose?: () => void
}) {
  const hasMessages = useAppStore((state) => state.companionMessages.length > 0)
  const reset = useAppStore((state) => state.resetCompanionConversation)
  const version = useAppStore((state) => state.companionStatus?.version)

  return (
    <header
      className={cn(
        'flex h-(--tabbar-height) shrink-0 items-center gap-1 border-b border-border-subtle',
        layout === 'workspace' ? 'px-4' : 'pr-2 pl-3',
      )}
    >
      {onBack && (
        <Button
          size="sm"
          variant="ghost"
          className="mr-2 gap-1.5 text-muted-foreground hover:text-foreground"
          onClick={onBack}
        >
          <ArrowLeft />
          Back to document
        </Button>
      )}
      <h2 className="min-w-0 flex-1 truncate text-sm font-medium">
        Companion
        {version && (
          <span className="ml-1.5 text-xs font-normal text-muted-foreground">
            OpenCode {version}
          </span>
        )}
      </h2>
      <Button
        size="icon-xs"
        variant="ghost"
        aria-label="New chat"
        title="New chat"
        disabled={!hasMessages}
        onClick={reset}
      >
        <SquarePen />
      </Button>
      {onExpand && (
        <Button
          size="icon-xs"
          variant="ghost"
          aria-label="Expand companion"
          title="Expand"
          onClick={onExpand}
        >
          <Expand />
        </Button>
      )}
      {onClose && (
        <Button size="icon-xs" variant="ghost" aria-label="Close companion" onClick={onClose}>
          <X />
        </Button>
      )}
    </header>
  )
}
