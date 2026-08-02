import { ArrowLeft, Expand, MessageSquare, X } from 'lucide-react'
import { cn } from '@renderer/lib/utils'
import { useAppStore } from '../../store/app-store'
import { Button } from '../ui/button'
import { CompanionModelPicker } from './CompanionModelPicker'

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
  const modelState = useAppStore((state) => state.companionModelState)
  const selectModel = useAppStore((state) => state.selectCompanionModel)

  return (
    <header
      className={cn(
        'flex h-(--tabbar-height) shrink-0 items-center gap-1 border-b border-border-subtle',
        layout === 'workspace' ? 'px-4' : 'px-2',
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
      <MessageSquare className="size-3.5 text-muted-foreground" aria-hidden />
      <h2 className="min-w-0 flex-1 truncate text-sm font-medium">Companion</h2>
      <div className={cn('min-w-0', layout === 'drawer' ? 'max-w-36' : 'max-w-60')}>
        <CompanionModelPicker
          state={modelState}
          onValueChange={(value) => void selectModel(value)}
        />
      </div>
      {onExpand && (
        <Button size="icon-xs" variant="ghost" aria-label="Expand companion" onClick={onExpand}>
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
