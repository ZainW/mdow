import { useEffect, useRef, useState } from 'react'
import { cn } from '~/lib/utils'
import { CheckIcon, CopyIcon } from './icons'

interface CopyButtonProps {
  value: string
  /** Visible text. Omit for an icon-only button. */
  label?: string
  className?: string
}

export function CopyButton({ value, label, className }: CopyButtonProps) {
  const [copied, setCopied] = useState(false)
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined)

  useEffect(() => () => clearTimeout(timer.current), [])

  async function onClick() {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      clearTimeout(timer.current)
      timer.current = setTimeout(() => setCopied(false), 1500)
    } catch {
      // ignore — older browsers without clipboard permission
    }
  }

  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={copied ? 'Copied' : label ? undefined : 'Copy to clipboard'}
      className={cn(
        'inline-flex h-8 items-center gap-1.5 rounded-md border border-border-subtle bg-card px-2 text-xs font-medium text-muted-foreground',
        'transition-[background-color,color] duration-150 ease hover:bg-muted hover:text-foreground',
        className,
      )}
    >
      <span className="relative size-3.5">
        <CopyIcon
          className={cn(
            'absolute inset-0 size-3.5 transition-[opacity,transform] duration-150 ease-out',
            copied && 'scale-75 opacity-0',
          )}
        />
        <CheckIcon
          className={cn(
            'absolute inset-0 size-3.5 text-accent transition-[opacity,transform] duration-150 ease-out',
            !copied && 'scale-75 opacity-0',
          )}
        />
      </span>
      {label && <span>{copied ? 'Copied' : label}</span>}
    </button>
  )
}
