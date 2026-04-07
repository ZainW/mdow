import { Link } from '@tanstack/react-router'
import type { DocMeta } from '~/lib/content'
import { groupByCategory } from '~/lib/content'
import { cn } from '~/lib/utils'

interface DocsSidebarProps {
  docs: DocMeta[]
  currentSlug: string
}

export function DocsSidebar({ docs, currentSlug }: DocsSidebarProps) {
  const groups = groupByCategory(docs)

  return (
    <nav className="space-y-7 text-sm">
      {groups.map((group) => (
        <div key={group.category}>
          <h4 className="mb-2 px-3 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            {group.category}
          </h4>
          <ul className="space-y-0.5">
            {group.docs.map((doc) => {
              const active = doc.slug === currentSlug
              return (
                <li key={doc.slug}>
                  <Link
                    to="/docs/$"
                    params={{ _splat: doc.slug }}
                    className={cn(
                      'relative block rounded-md px-3 py-1.5 transition-colors',
                      active
                        ? 'bg-surface font-medium text-foreground before:absolute before:left-0 before:top-1.5 before:bottom-1.5 before:w-0.5 before:rounded-full before:bg-primary'
                        : 'text-muted-foreground hover:bg-surface/60 hover:text-foreground',
                    )}
                  >
                    {doc.title}
                  </Link>
                </li>
              )
            })}
          </ul>
        </div>
      ))}
    </nav>
  )
}
