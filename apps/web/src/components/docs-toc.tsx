export interface TocItem {
  id: string
  text: string
  level: number
}

interface DocsTocProps {
  headings: TocItem[]
}

export function DocsToc({ headings }: DocsTocProps) {
  if (headings.length === 0) return null

  return (
    <nav className="hidden w-48 shrink-0 xl:block">
      <div className="sticky top-20">
        <h4 className="mb-3 text-sm font-semibold">On this page</h4>
        <ul className="space-y-1.5 text-sm">
          {headings.map((h) => (
            <li key={h.id} style={{ paddingLeft: `${(h.level - 2) * 12}px` }}>
              <a
                href={`#${h.id}`}
                className="block text-muted-foreground transition-colors hover:text-foreground"
              >
                {h.text}
              </a>
            </li>
          ))}
        </ul>
      </div>
    </nav>
  )
}
