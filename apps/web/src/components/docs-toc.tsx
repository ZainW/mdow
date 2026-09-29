import { useMemo } from 'react'
import { useScrollspy } from '~/hooks/use-scrollspy'
import { cn } from '~/lib/utils'

export interface TocItem {
  id: string
  text: string
  level: number
}

interface DocsTocProps {
  headings: TocItem[]
}

export function DocsToc({ headings }: DocsTocProps) {
  const ids = useMemo(() => headings.map((h) => h.id), [headings])
  const active = useScrollspy(ids)

  if (headings.length === 0) return null

  return (
    <nav className="hidden w-48 shrink-0 xl:block" aria-label="On this page">
      <div className="sticky top-16 max-h-[calc(100dvh-4rem)] overflow-y-auto pb-10 pt-10">
        <h2 className="mb-3 text-xs font-medium text-faint">On this page</h2>
        <ul className="space-y-0.5 border-l border-border-subtle text-[13px]">
          {headings.map((h) => {
            const isActive = h.id === active
            return (
              <li key={h.id}>
                <a
                  href={`#${h.id}`}
                  aria-current={isActive ? 'location' : undefined}
                  className={cn(
                    '-ml-px block border-l py-1 leading-snug transition-[border-color,color] duration-150 ease',
                    h.level > 2 ? 'pl-6' : 'pl-3',
                    isActive
                      ? 'border-foreground text-foreground'
                      : 'border-transparent text-muted-foreground hover:text-foreground',
                  )}
                >
                  {h.text}
                </a>
              </li>
            )
          })}
        </ul>
      </div>
    </nav>
  )
}
