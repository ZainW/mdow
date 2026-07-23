import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@renderer/components/ui/collapsible'
import { cn } from '@renderer/lib/utils'
import { BrainIcon, ChevronDownIcon } from 'lucide-react'
import type { ComponentProps, ReactNode } from 'react'
import { createContext, useContext, useEffect, useState } from 'react'
import { CompanionMarkdown } from './markdown'
import { Shimmer } from './shimmer'

interface ReasoningContextValue {
  isStreaming: boolean
  isOpen: boolean
  setIsOpen: (open: boolean) => void
}

const ReasoningContext = createContext<ReasoningContextValue | null>(null)

function useReasoning() {
  const context = useContext(ReasoningContext)
  if (!context) throw new Error('Reasoning components must be used within Reasoning')
  return context
}

export function Reasoning({
  className,
  isStreaming = false,
  defaultOpen,
  children,
  ...props
}: ComponentProps<typeof Collapsible> & {
  isStreaming?: boolean
  defaultOpen?: boolean
}) {
  const [isOpen, setIsOpen] = useState(defaultOpen ?? isStreaming)

  useEffect(() => {
    if (isStreaming) setIsOpen(true)
  }, [isStreaming])

  useEffect(() => {
    if (!isStreaming && defaultOpen === undefined) {
      const timer = window.setTimeout(() => setIsOpen(false), 800)
      return () => window.clearTimeout(timer)
    }
    return undefined
  }, [isStreaming, defaultOpen])

  return (
    <ReasoningContext.Provider value={{ isStreaming, isOpen, setIsOpen }}>
      <Collapsible
        open={isOpen}
        onOpenChange={setIsOpen}
        className={cn('not-prose mb-2 w-full rounded-md border border-border-subtle', className)}
        {...props}
      >
        {children}
      </Collapsible>
    </ReasoningContext.Provider>
  )
}

export function ReasoningTrigger({
  className,
  children,
  ...props
}: ComponentProps<typeof CollapsibleTrigger> & { children?: ReactNode }) {
  const { isStreaming, isOpen } = useReasoning()
  return (
    <CollapsibleTrigger
      className={cn(
        'flex w-full items-center gap-2 px-2.5 py-2 text-left text-xs text-muted-foreground hover:text-foreground',
        className,
      )}
      {...props}
    >
      <BrainIcon className="size-3.5 shrink-0" />
      <span className="min-w-0 flex-1 font-medium">
        {children ??
          (isStreaming ? <Shimmer className="w-fit">Thinking</Shimmer> : 'Thought process')}
      </span>
      <ChevronDownIcon
        className={cn('size-3.5 shrink-0 transition-transform', isOpen && 'rotate-180')}
      />
    </CollapsibleTrigger>
  )
}

export function ReasoningContent({
  className,
  children,
  ...props
}: ComponentProps<typeof CollapsibleContent> & { children: string }) {
  const { isStreaming } = useReasoning()
  return (
    <CollapsibleContent
      className={cn(
        'border-t border-border-subtle px-2.5 py-2 text-xs text-muted-foreground',
        className,
      )}
      {...props}
    >
      <CompanionMarkdown text={children} streaming={isStreaming} />
    </CollapsibleContent>
  )
}
