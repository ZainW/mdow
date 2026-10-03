import { cn } from '@renderer/lib/utils'
import type { HTMLAttributes } from 'react'
import { CompanionMarkdown, type DocumentResolver } from './markdown'

export function Message({
  className,
  from,
  ...props
}: HTMLAttributes<HTMLDivElement> & { from: 'user' | 'assistant' | 'system' }) {
  return (
    <div
      className={cn(
        'group flex w-full flex-col gap-2',
        from === 'user' ? 'is-user ml-auto max-w-[85%] items-end' : 'is-assistant',
        className,
      )}
      data-role={from}
      {...props}
    />
  )
}

export function MessageContent({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        'flex w-fit min-w-0 max-w-full flex-col gap-2 overflow-hidden text-sm',
        'group-[.is-user]:ml-auto group-[.is-user]:rounded-2xl group-[.is-user]:rounded-br-md group-[.is-user]:bg-secondary group-[.is-user]:px-3 group-[.is-user]:py-2 group-[.is-user]:text-foreground',
        'group-[.is-assistant]:w-full group-[.is-assistant]:gap-2.5 group-[.is-assistant]:text-foreground',
        className,
      )}
      {...props}
    />
  )
}

export function MessageResponse({
  children,
  className,
  streaming = false,
  resolveDocument,
  ...props
}: HTMLAttributes<HTMLDivElement> & { streaming?: boolean; resolveDocument?: DocumentResolver }) {
  const text = typeof children === 'string' ? children : ''
  return (
    <div className={cn('companion-response min-w-0', className)} {...props}>
      <CompanionMarkdown text={text} streaming={streaming} resolveDocument={resolveDocument} />
    </div>
  )
}
