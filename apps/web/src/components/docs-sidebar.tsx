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
    <nav className="space-y-7 text-sm" aria-label="Documentation">
      {groups.map((group) => (
        <div key={group.category}>
          <h2 className="mb-2 px-3 text-xs font-medium text-faint">{group.category}</h2>
          <ul className="space-y-px">
            {group.docs.map((doc) => {
              const active = doc.slug === currentSlug
              return (
                <li key={doc.slug}>
                  <Link
                    to="/docs/$"
                    params={{ _splat: doc.slug }}
                    aria-current={active ? 'page' : undefined}
                    className={cn(
                      'block rounded-lg px-3 py-2 transition-[background-color,color] duration-150 ease lg:py-1.5',
                      active
                        ? 'bg-muted font-medium text-foreground'
                        : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
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
