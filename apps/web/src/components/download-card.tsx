import { cn } from '~/lib/utils'

interface DownloadCardProps {
  platform: string
  icon: string
  formats: { label: string; url: string }[]
  recommended?: boolean
}

export function DownloadCard({ platform, icon, formats, recommended }: DownloadCardProps) {
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
        {formats.map((f) => (
          <a
            key={f.label}
            href={f.url}
            className={cn(
              'inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium transition-colors',
              recommended
                ? 'bg-primary text-primary-foreground hover:bg-primary/90'
                : 'border hover:bg-muted',
            )}
          >
            {f.label}
          </a>
        ))}
      </div>
    </div>
  )
}
