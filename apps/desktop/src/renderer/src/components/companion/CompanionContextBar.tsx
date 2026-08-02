import { basename } from '@renderer/lib/path-utils'
import { FileText, Gauge, Search } from 'lucide-react'
import type { CompanionContextTrace } from '../../../../shared/types'
import { Button } from '../ui/button'
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from '../ui/popover'

function formatTokens(tokens: number): string {
  if (tokens < 1_000) return String(tokens)
  return `${(tokens / 1_000).toFixed(tokens >= 10_000 ? 0 : 1)}k`
}

function formatBytes(bytes: number): string {
  if (bytes < 1_024) return `${bytes} B`
  return `${(bytes / 1_024).toFixed(1)} KB`
}

export function CompanionContextBar({
  trace,
  warnings,
}: {
  trace: CompanionContextTrace | null
  warnings: string[]
}) {
  if (!trace) return null
  const focused = trace.items.find((item) => item.reason === 'focused')
  const adaptiveLabel =
    trace.retrievalMode === 'adaptive-fff'
      ? 'Adaptive · FFF'
      : trace.retrievalMode === 'adaptive-local'
        ? 'Adaptive · local'
        : 'Adaptive'

  return (
    <div className="flex min-h-9 items-center gap-1.5 overflow-hidden border-t border-border-subtle px-3 py-1.5 text-[11px] text-muted-foreground">
      <Popover>
        <PopoverTrigger
          render={
            <Button
              variant="secondary"
              size="xs"
              className="min-w-0 max-w-52 gap-1.5 rounded-full px-2 font-normal"
            />
          }
        >
          <FileText className="size-3" aria-hidden />
          <span className="truncate">{focused ? basename(focused.path) : 'No focused doc'}</span>
          <span className="shrink-0">· {trace.focusedCount} focused</span>
        </PopoverTrigger>
        <PopoverContent align="start" side="top" className="w-80 gap-3">
          <PopoverHeader>
            <PopoverTitle>Context added by Mdow</PopoverTitle>
            <PopoverDescription>
              Estimates cover Mdow source injection, not the provider&apos;s full context window.
            </PopoverDescription>
          </PopoverHeader>
          <div className="space-y-1.5">
            {trace.items.map((item, index) => (
              <div key={`${item.path}:${item.reason}:${index}`} className="flex gap-2">
                <span className="min-w-0 flex-1 truncate" title={item.path}>
                  {basename(item.path)} · {item.reason}
                </span>
                <span className="shrink-0 tabular-nums">{formatBytes(item.bytes)}</span>
              </div>
            ))}
          </div>
          {warnings.length > 0 && (
            <div className="border-t border-border-subtle pt-2 text-amber-600 dark:text-amber-400">
              {warnings.map((warning, index) => (
                <p key={`${warning}:${index}`}>{warning}</p>
              ))}
            </div>
          )}
        </PopoverContent>
      </Popover>
      <span className="inline-flex h-6 shrink-0 items-center gap-1 rounded-full bg-muted/60 px-2">
        <Search className="size-3" aria-hidden />
        {adaptiveLabel}
        {trace.searchedCount > 0 && ` · ${trace.readRangeCount} read`}
      </span>
      <span className="ml-auto inline-flex h-6 shrink-0 items-center gap-1 rounded-full px-1.5 tabular-nums">
        <Gauge className="size-3" aria-hidden />≈{formatTokens(trace.estimatedTokens)} added
      </span>
    </div>
  )
}
