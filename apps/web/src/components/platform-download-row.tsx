import type { ReactNode } from 'react'
import { DownloadIcon } from './icons'
import { cn } from '~/lib/utils'

export interface DownloadFormat {
  label: string
  detail?: string
  url: string
}

interface PlatformDownloadRowProps {
  icon: ReactNode
  platform: string
  description: string
  formats: DownloadFormat[]
  highlighted?: boolean
}

export function PlatformDownloadRow({
  icon,
  platform,
  description,
  formats,
  highlighted = false,
}: PlatformDownloadRowProps) {
  return (
    <div className="flex flex-col gap-4 px-5 py-5 sm:flex-row sm:items-center sm:justify-between sm:px-6">
      <div className="flex items-center gap-4">
        <span
          className={cn(
            'flex size-10 shrink-0 items-center justify-center rounded-xl border border-border-subtle bg-surface',
            highlighted && 'border-accent/30 bg-accent/8 text-accent',
          )}
          aria-hidden
        >
          {icon}
        </span>
        <div>
          <h3 className="flex items-center gap-2 font-medium">
            {platform}
            {highlighted && (
              <span className="rounded-full bg-accent/12 px-2 py-0.5 text-[11px] font-medium text-accent">
                Your system
              </span>
            )}
          </h3>
          <p className="mt-0.5 text-sm text-muted-foreground">{description}</p>
        </div>
      </div>
      <div className="flex flex-wrap gap-2 sm:justify-end">
        {formats.length === 0 ? (
          <p className="text-sm text-muted-foreground">Not available in this release</p>
        ) : (
          formats.map((f) => (
            <a
              key={f.url}
              href={f.url}
              className="btn btn-secondary btn-sm"
              aria-label={`Download ${platform} ${f.label}${f.detail ? ` (${f.detail})` : ''}`}
            >
              <DownloadIcon className="size-4 text-muted-foreground" />
              {f.label}
              {f.detail && <span className="text-muted-foreground">{f.detail}</span>}
            </a>
          ))
        )}
      </div>
    </div>
  )
}
