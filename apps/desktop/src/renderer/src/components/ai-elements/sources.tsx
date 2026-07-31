import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@renderer/components/ui/collapsible'
import { cn } from '@renderer/lib/utils'
import { BookIcon, ChevronDownIcon } from 'lucide-react'
import type { ComponentProps, ReactNode } from 'react'

export function Sources({ className, ...props }: ComponentProps<'div'>) {
  return <div className={cn('not-prose mt-2 flex flex-col gap-1', className)} {...props} />
}

export function SourcesTrigger({
  count,
  className,
  children,
  ...props
}: ComponentProps<typeof CollapsibleTrigger> & { count: number; children?: ReactNode }) {
  return (
    <CollapsibleTrigger
      className={cn(
        'flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground',
        className,
      )}
      {...props}
    >
      <BookIcon className="size-3.5" />
      {children ?? (
        <span>
          Used {count} source{count === 1 ? '' : 's'}
        </span>
      )}
      <ChevronDownIcon className="size-3.5" />
    </CollapsibleTrigger>
  )
}

export function SourcesContent({ className, ...props }: ComponentProps<typeof CollapsibleContent>) {
  return <CollapsibleContent className={cn('mt-1 flex flex-col gap-1', className)} {...props} />
}

export function Source({
  href,
  title,
  className,
  onClick,
  ...props
}: ComponentProps<'button'> & { href?: string; title: string }) {
  return (
    <button
      type="button"
      className={cn(
        'flex w-fit items-center gap-1.5 rounded-sm bg-muted px-1.5 py-0.5 text-left text-[11px] text-muted-foreground hover:text-foreground',
        className,
      )}
      onClick={onClick}
      data-href={href}
      {...props}
    >
      <BookIcon className="size-3" />
      <span className="truncate">{title}</span>
    </button>
  )
}

export function SourcesCollapsible({
  className,
  count,
  children,
  ...props
}: ComponentProps<typeof Collapsible> & { count: number }) {
  if (count === 0) return null
  return (
    <Collapsible className={cn('w-full', className)} {...props}>
      <SourcesTrigger count={count} />
      <SourcesContent>{children}</SourcesContent>
    </Collapsible>
  )
}
