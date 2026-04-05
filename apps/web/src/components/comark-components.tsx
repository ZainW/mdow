import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'

interface AlertProps {
  type?: 'info' | 'warning' | 'error' | 'success'
  children: ReactNode
}

function Alert({ type = 'info', children }: AlertProps) {
  const styles = {
    info: 'border-primary/30 bg-primary/5 text-primary',
    warning: 'border-accent/30 bg-accent/5 text-accent-foreground',
    error: 'border-destructive/30 bg-destructive/5 text-destructive',
    success: 'border-green-500/30 bg-green-500/5 text-green-700 dark:text-green-400',
  }

  return <div className={cn('my-4 rounded-lg border p-4', styles[type])}>{children}</div>
}

interface CalloutProps {
  title?: string
  children: ReactNode
}

function Callout({ title, children }: CalloutProps) {
  return (
    <div className="my-4 rounded-lg border border-border bg-muted/50 p-4">
      {title && <p className="mb-2 font-semibold">{title}</p>}
      <div className="text-sm text-muted-foreground">{children}</div>
    </div>
  )
}

export const docsComponents = {
  alert: Alert,
  callout: Callout,
}
