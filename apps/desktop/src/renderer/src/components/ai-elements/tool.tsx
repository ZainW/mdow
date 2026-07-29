import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@renderer/components/ui/collapsible'
import { Badge } from '@renderer/components/ui/badge'
import { cn } from '@renderer/lib/utils'
import type { CompanionToolState } from '../../../../shared/types'
import {
  CheckCircleIcon,
  ChevronDownIcon,
  CircleIcon,
  ClockIcon,
  WrenchIcon,
  XCircleIcon,
} from 'lucide-react'
import type { ComponentProps, ReactNode } from 'react'

const statusLabels: Record<CompanionToolState, string> = {
  pending: 'Pending',
  running: 'Running',
  completed: 'Completed',
  error: 'Error',
  cancelled: 'Cancelled',
}

const statusIcons: Record<CompanionToolState, ReactNode> = {
  pending: <CircleIcon className="size-3.5 text-muted-foreground" />,
  running: <ClockIcon className="size-3.5 animate-pulse text-amber-600" />,
  completed: <CheckCircleIcon className="size-3.5 text-emerald-600" />,
  error: <XCircleIcon className="size-3.5 text-destructive" />,
  cancelled: <XCircleIcon className="size-3.5 text-muted-foreground" />,
}

export function Tool({ className, ...props }: ComponentProps<typeof Collapsible>) {
  return (
    <Collapsible
      className={cn('not-prose w-full border-b border-border-subtle', className)}
      {...props}
    />
  )
}

export function ToolHeader({
  name,
  state,
  className,
  ...props
}: ComponentProps<typeof CollapsibleTrigger> & {
  name: string
  state: CompanionToolState
}) {
  return (
    <CollapsibleTrigger
      className={cn('flex w-full items-center gap-2 py-1.5 text-left text-xs', className)}
      {...props}
    >
      <WrenchIcon className="size-3.5 shrink-0 text-muted-foreground" />
      <span className="min-w-0 flex-1 truncate font-medium text-foreground">{name}</span>
      <Badge variant="secondary" className="gap-1 py-0.5 pr-1.5 pl-0.5 font-normal">
        {statusIcons[state]}
        {statusLabels[state]}
      </Badge>
      <ChevronDownIcon className="size-3.5 shrink-0 text-muted-foreground" />
    </CollapsibleTrigger>
  )
}

export function ToolContent({ className, ...props }: ComponentProps<typeof CollapsibleContent>) {
  return (
    <CollapsibleContent
      className={cn('border-t border-border-subtle py-2 text-xs', className)}
      {...props}
    />
  )
}

export function ToolInput({ input }: { input?: string }) {
  if (!input) return null
  return (
    <div className="mb-2">
      <p className="mb-1 font-medium text-muted-foreground">Input</p>
      <pre className="overflow-x-auto rounded-md bg-muted p-2 whitespace-pre-wrap">{input}</pre>
    </div>
  )
}

export function ToolOutput({ output, error }: { output?: string; error?: string }) {
  if (!output && !error) return null
  return (
    <div>
      <p className="mb-1 font-medium text-muted-foreground">{error ? 'Error' : 'Output'}</p>
      <pre
        className={cn(
          'overflow-x-auto rounded-md p-2 whitespace-pre-wrap',
          error ? 'bg-destructive/10 text-destructive' : 'bg-muted',
        )}
      >
        {error ?? output}
      </pre>
    </div>
  )
}
