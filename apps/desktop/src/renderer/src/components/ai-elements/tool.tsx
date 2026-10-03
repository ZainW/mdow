import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@renderer/components/ui/collapsible'
import { cn } from '@renderer/lib/utils'
import type { CompanionToolState } from '../../../../shared/types'
import {
  CheckIcon,
  ChevronRightIcon,
  CircleDashedIcon,
  FileTextIcon,
  GlobeIcon,
  ListTodoIcon,
  LoaderCircleIcon,
  SearchIcon,
  WrenchIcon,
  XIcon,
} from 'lucide-react'
import type { ComponentProps } from 'react'
import { Shimmer } from './shimmer'

function ToolIcon({ name, state }: { name: string; state: CompanionToolState }) {
  const className = 'size-3.5 shrink-0'
  if (state === 'running') {
    return <LoaderCircleIcon className={cn(className, 'motion-safe:animate-spin')} aria-hidden />
  }
  if (state === 'pending') return <CircleDashedIcon className={className} aria-hidden />
  if (state === 'error') return <XIcon className={cn(className, 'text-destructive')} aria-hidden />
  if (state === 'cancelled') return <XIcon className={className} aria-hidden />
  if (name.startsWith('Read')) return <FileTextIcon className={className} aria-hidden />
  if (name.startsWith('Searched the web') || name.startsWith('Fetched')) {
    return <GlobeIcon className={className} aria-hidden />
  }
  if (name.startsWith('Searched') || name.startsWith('Looked')) {
    return <SearchIcon className={className} aria-hidden />
  }
  if (name.startsWith('Updated its plan')) return <ListTodoIcon className={className} aria-hidden />
  if (state === 'completed') return <CheckIcon className={className} aria-hidden />
  return <WrenchIcon className={className} aria-hidden />
}

const stateLabels: Record<CompanionToolState, string> = {
  pending: 'starting',
  running: 'running',
  completed: 'done',
  error: 'failed',
  cancelled: 'stopped',
}

export function Tool({ className, ...props }: ComponentProps<typeof Collapsible>) {
  return <Collapsible className={cn('not-prose group/tool w-full', className)} {...props} />
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
  const active = state === 'running' || state === 'pending'
  return (
    <CollapsibleTrigger
      aria-label={`${name}, ${stateLabels[state]}`}
      className={cn(
        'flex w-full min-w-0 items-center gap-2 rounded-sm py-1 text-left text-xs text-muted-foreground transition-colors hover:text-foreground',
        state === 'error' && 'text-destructive hover:text-destructive',
        className,
      )}
      {...props}
    >
      <ToolIcon name={name} state={state} />
      <span className="min-w-0 flex-1 truncate">
        {active ? <Shimmer className="max-w-full truncate align-bottom">{name}</Shimmer> : name}
      </span>
      <ChevronRightIcon
        className="size-3 shrink-0 opacity-0 transition-[opacity,rotate] group-hover/tool:opacity-100 group-data-[open]/tool:rotate-90 group-data-[open]/tool:opacity-100"
        aria-hidden
      />
    </CollapsibleTrigger>
  )
}

export function ToolContent({ className, ...props }: ComponentProps<typeof CollapsibleContent>) {
  return (
    <CollapsibleContent
      className={cn('ml-1.5 border-l border-border-subtle py-1 pl-3.5 text-xs', className)}
      {...props}
    />
  )
}

function Detail({ label, text, error }: { label: string; text: string; error?: boolean }) {
  return (
    <div className="mb-2 last:mb-0">
      <p className="mb-1 text-[11px] font-medium text-muted-foreground">{label}</p>
      <pre
        className={cn(
          'max-h-48 overflow-auto rounded-md p-2 font-mono text-[11px] leading-4 whitespace-pre-wrap',
          error ? 'bg-destructive/10 text-destructive' : 'bg-muted',
        )}
      >
        {text}
      </pre>
    </div>
  )
}

export function ToolInput({ input }: { input?: string }) {
  if (!input) return null
  return <Detail label="Input" text={input} />
}

export function ToolOutput({ output, error }: { output?: string; error?: string }) {
  if (error) return <Detail label="Error" text={error} error />
  if (!output) return null
  return <Detail label="Result" text={output} />
}
