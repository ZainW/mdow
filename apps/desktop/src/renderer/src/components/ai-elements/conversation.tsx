import { cn } from '@renderer/lib/utils'
import type { ComponentProps, HTMLAttributes, ReactNode } from 'react'
import { useEffect, useRef } from 'react'
import { Button } from '@renderer/components/ui/button'
import { ArrowDownIcon } from 'lucide-react'

export function Conversation({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      role="log"
      aria-live="polite"
      className={cn('relative flex min-h-0 flex-1 flex-col overflow-hidden', className)}
      {...props}
    />
  )
}

export function ConversationContent({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn('flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-3 py-3', className)}
      {...props}
    />
  )
}

export function ConversationEmptyState({
  className,
  title = 'Start a conversation',
  description,
  icon,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement> & {
  title?: string
  description?: string
  icon?: ReactNode
}) {
  return (
    <div
      className={cn(
        'flex flex-1 flex-col items-center justify-center gap-2 px-4 text-center text-sm text-muted-foreground',
        className,
      )}
      {...props}
    >
      {icon}
      <p className="font-medium text-foreground">{title}</p>
      {description && <p className="max-w-56 text-xs leading-5">{description}</p>}
      {children}
    </div>
  )
}

export function ConversationScrollButton({
  containerRef,
  className,
  ...props
}: ComponentProps<typeof Button> & {
  containerRef: React.RefObject<HTMLDivElement | null>
}) {
  const visibleRef = useRef(false)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const onScroll = () => {
      const distance = el.scrollHeight - el.scrollTop - el.clientHeight
      visibleRef.current = distance > 80
    }
    el.addEventListener('scroll', onScroll, { passive: true })
    return () => el.removeEventListener('scroll', onScroll)
  }, [containerRef])

  return (
    <Button
      type="button"
      size="icon-sm"
      variant="secondary"
      className={cn('absolute right-3 bottom-3 shadow-sm', className)}
      aria-label="Scroll to latest message"
      onClick={() => {
        containerRef.current?.scrollTo({
          top: containerRef.current.scrollHeight,
          behavior: 'smooth',
        })
      }}
      {...props}
    >
      <ArrowDownIcon />
    </Button>
  )
}
