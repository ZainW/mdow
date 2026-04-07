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
    <nav className="hidden w-52 shrink-0 xl:block">
      <div className="sticky top-20">
        <h4 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          On this page
        </h4>
        <ul className="space-y-1.5 text-sm">
          {headings.map((h) => {
            const isActive = h.id === active
            return (
              <li key={h.id} style={{ paddingLeft: `${(h.level - 2) * 12}px` }}>
                <a
                  href={`#${h.id}`}
                  className={cn(
                    'block border-l-2 pl-3 -ml-px transition-colors',
                    isActive
                      ? 'border-primary text-foreground font-medium'
                      : 'border-transparent text-muted-foreground hover:border-border hover:text-foreground',
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
