import { useState } from 'react'
import { cn } from '~/lib/utils'

interface DownloadCardProps {
  platform: string
  icon: string
  formats: { label: string; url: string }[]
  recommended?: boolean
  note?: string
}

export function DownloadCard({ platform, icon, formats, recommended, note }: DownloadCardProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    if (!note) return
    await navigator.clipboard.writeText(note)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <div
      className={cn(
        'rounded-lg border p-6 text-center',
        recommended && 'border-primary bg-primary/5 ring-1 ring-primary/20',
      )}
    >
      <div className="mb-3 text-3xl">{icon}</div>
      <h3 className="mb-1 text-lg font-semibold">{platform}</h3>
      {recommended && (
        <p className="mb-3 text-xs font-medium text-primary">Recommended for your OS</p>
      )}
      <div className="flex flex-col gap-2">
        {formats.length === 0 ? (
          <p className="text-xs text-muted-foreground">Not available for this release.</p>
        ) : (
          formats.map((f) => (
            <a
              key={f.label}
              href={f.url}
              className={cn(
                'inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium transition-colors duration-150 ease-out',
                recommended
                  ? 'bg-primary text-primary-foreground hover:bg-primary/90'
                  : 'border hover:bg-muted',
              )}
            >
              {f.label}
            </a>
          ))
        )}
      </div>
      {note && (
        <div className="mt-4 flex items-center justify-between gap-2 rounded-md bg-muted/60 px-3 py-2 text-left">
          <code className="truncate text-xs">{note}</code>
          <button
            type="button"
            onClick={() => void handleCopy()}
            className="shrink-0 rounded px-2 py-1 text-xs text-muted-foreground transition-colors duration-150 ease-out hover:bg-muted"
            aria-label="Copy install command"
          >
            {copied ? 'Copied' : 'Copy'}
          </button>
        </div>
      )}
    </div>
  )
}
