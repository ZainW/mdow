import { useState } from 'react'
import { cn } from '~/lib/utils'

interface CopyButtonProps {
  value: string
  className?: string
}

export function CopyButton({ value, className }: CopyButtonProps) {
  const [copied, setCopied] = useState(false)

  async function onClick() {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      // ignore — older browsers without clipboard permission
    }
  }

  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={copied ? 'Copied' : 'Copy to clipboard'}
      className={cn(
        'inline-flex h-7 items-center rounded-md border border-border-subtle bg-surface px-2 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-muted',
        className,
      )}
    >
      {copied ? 'Copied' : 'Copy'}
    </button>
  )
}
