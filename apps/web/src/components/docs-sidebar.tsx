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
    <nav className="space-y-6 text-sm">
      {groups.map((group) => (
        <div key={group.category}>
          <h4 className="mb-2 font-semibold text-foreground">{group.category}</h4>
          <ul className="space-y-1">
            {group.docs.map((doc) => (
              <li key={doc.slug}>
                <Link
                  to="/docs/$"
                  params={{ _splat: doc.slug }}
                  className={cn(
                    'block rounded-md px-3 py-1.5 transition-colors',
                    doc.slug === currentSlug
                      ? 'bg-muted font-medium text-foreground'
                      : 'text-muted-foreground hover:text-foreground',
                  )}
                >
                  {doc.title}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  )
}
