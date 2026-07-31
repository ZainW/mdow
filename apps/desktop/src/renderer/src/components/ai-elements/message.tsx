import { cn } from '@renderer/lib/utils'
import type { HTMLAttributes } from 'react'
import { CompanionMarkdown } from './markdown'

export function Message({
  className,
  from,
  ...props
}: HTMLAttributes<HTMLDivElement> & { from: 'user' | 'assistant' | 'system' }) {
  return (
    <div
      className={cn(
        'group flex w-full max-w-[95%] flex-col gap-2',
        from === 'user' ? 'is-user ml-auto justify-end' : 'is-assistant',
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
        'group-[.is-user]:ml-auto group-[.is-user]:rounded-lg group-[.is-user]:bg-secondary group-[.is-user]:px-3 group-[.is-user]:py-2 group-[.is-user]:text-foreground',
        'group-[.is-assistant]:text-foreground',
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
  ...props
}: HTMLAttributes<HTMLDivElement> & { streaming?: boolean }) {
  const text = typeof children === 'string' ? children : ''
  return (
    <div className={cn('companion-response min-w-0', className)} {...props}>
      <CompanionMarkdown text={text} streaming={streaming} />
    </div>
  )
}
