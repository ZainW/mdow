import { GradientSection } from '~/components/gradient-section'
import { formatShortcut, useModKey } from '~/hooks/use-mod-key'
import { cardLiftClass } from '~/lib/button-styles'
import { cn } from '~/lib/utils'

const ICON_CLASS = 'h-5 w-5 shrink-0 text-primary'

function TabsIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      className={className}
    >
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M3 9h18" />
      <path d="M9 3v6" />
    </svg>
  )
}

function CommandIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      className={className}
    >
      <path d="m18 15-6-6-6 6" />
      <path d="M4 4h16" />
    </svg>
  )
}

function SearchIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      className={className}
    >
      <circle cx="11" cy="11" r="8" />
      <path d="m21 21-4.3-4.3" />
    </svg>
  )
}

function SyncIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      className={className}
    >
      <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
      <path d="M3 3v5h5" />
      <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
      <path d="M16 16h5v5" />
    </svg>
  )
}

function ListIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      className={className}
    >
      <line x1="8" y1="6" x2="21" y2="6" />
      <line x1="8" y1="12" x2="21" y2="12" />
      <line x1="8" y1="18" x2="21" y2="18" />
      <line x1="3" y1="6" x2="3.01" y2="6" />
      <line x1="3" y1="12" x2="3.01" y2="12" />
      <line x1="3" y1="18" x2="3.01" y2="18" />
    </svg>
  )
}

function CompanionIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      className={className}
    >
      <path d="M12 3v3m0 12v3M3 12h3m12 0h3" />
      <path d="m5.64 5.64 2.12 2.12m8.48 8.48 2.12 2.12m0-12.72-2.12 2.12m-8.48 8.48-2.12 2.12" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  )
}

export function LandingHighlights() {
  const modKey = useModKey()

  const highlights = [
    {
      icon: CompanionIcon,
      title: 'Local AI companion',
      description:
        'Ask questions across documents through OpenCode and ACP, with context you control and citations you can verify.',
    },
    {
      icon: TabsIcon,
      title: 'Tabbed reading',
      description: 'Keep multiple documents open and switch between them instantly.',
    },
    {
      icon: CommandIcon,
      title: 'Command palette',
      description: `Jump to any file or action with ${formatShortcut(modKey, 'K') || '⌘K'} — fast navigation without leaving the keyboard.`,
    },
    {
      icon: SearchIcon,
      title: 'Find in document',
      description: `Search within the current file with ${formatShortcut(modKey, 'F') || '⌘F'} and highlighted matches.`,
    },
    {
      icon: SyncIcon,
      title: 'Live file watching',
      description: 'Edits on disk show up automatically while you read.',
    },
    {
      icon: ListIcon,
      title: 'Document outline',
      description: 'Jump between headings with a sidebar outline for long reads.',
    },
  ]

  return (
    <GradientSection variant="surface" innerClassName="py-16 md:py-24">
      <div className="mx-auto max-w-2xl text-center">
        <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl text-balance">
          Everything you need for serious reading
        </h2>
        <p className="mt-4 text-muted-foreground text-balance">
          Mdow is a viewer, not an editor, built for people who write in their editor and read in
          mdow.
        </p>
      </div>
      <dl className="mx-auto mt-12 grid max-w-5xl gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {highlights.map((item) => (
          <div
            key={item.title}
            className={cn('group rounded-lg bg-card p-5 shadow-card', cardLiftClass)}
          >
            <div className="mb-3 inline-flex size-9 items-center justify-center rounded-md bg-accent/10 ring-1 ring-accent/20">
              <item.icon className={ICON_CLASS} />
            </div>
            <dt className="font-semibold">{item.title}</dt>
            <dd className="mt-2 text-sm leading-relaxed text-muted-foreground">
              {item.description}
            </dd>
          </div>
        ))}
      </dl>
    </GradientSection>
  )
}
