import { GradientSection } from '~/components/gradient-section'
import { formatShortcut, useModKey } from '~/hooks/use-mod-key'
import { cardLiftClass } from '~/lib/button-styles'
import { cn } from '~/lib/utils'

export function LandingHighlights() {
  const modKey = useModKey()

  const highlights = [
    {
      title: 'Tabbed reading',
      description: 'Keep multiple documents open and switch between them instantly.',
    },
    {
      title: 'Command palette',
      description: `Jump to any file or action with ${formatShortcut(modKey, 'K') || '⌘K'} — fast navigation without leaving the keyboard.`,
    },
    {
      title: 'Find in document',
      description: `Search within the current file with ${formatShortcut(modKey, 'F') || '⌘F'} and highlighted matches.`,
    },
    {
      title: 'Live file watching',
      description: 'Edits on disk show up automatically while you read.',
    },
    {
      title: 'Document outline',
      description: 'Jump between headings with a sidebar outline for long reads.',
    },
    {
      title: 'Wide reading mode',
      description: 'Hide chrome and focus on the content when you need more space.',
    },
  ]

  return (
    <GradientSection variant="surface" innerClassName="py-16 md:py-24">
      <div className="mx-auto max-w-2xl text-center">
        <h2 className="text-3xl font-bold tracking-tight sm:text-4xl text-balance">
          Everything you need for serious reading
        </h2>
        <p className="mt-4 text-muted-foreground text-balance">
          Mdow is a viewer, not an editor — built for people who write in their editor and read in
          mdow.
        </p>
      </div>
      <div className="mx-auto mt-12 grid max-w-5xl gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {highlights.map((item) => (
          <div
            key={item.title}
            className={cn(
              'rounded-xl border border-border-subtle bg-card p-5 shadow-soft',
              cardLiftClass,
            )}
          >
            <h3 className="font-semibold">{item.title}</h3>
            <p className="mt-2 text-sm text-muted-foreground">{item.description}</p>
          </div>
        ))}
      </div>
    </GradientSection>
  )
}
