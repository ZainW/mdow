import { Link } from '@tanstack/react-router'
import type { DocMeta } from '~/lib/content'

interface DocsNavProps {
  docs: DocMeta[]
  currentSlug: string
}

export function DocsNav({ docs, currentSlug }: DocsNavProps) {
  const currentIndex = docs.findIndex((d) => d.slug === currentSlug)
  const prev = currentIndex > 0 ? docs[currentIndex - 1] : null
  const next = currentIndex < docs.length - 1 ? docs[currentIndex + 1] : null

  return (
    <div className="mt-12 flex justify-between border-t pt-6">
      {prev ? (
        <Link
          to="/docs/$"
          params={{ _splat: prev.slug }}
          className="text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          &larr; {prev.title}
        </Link>
      ) : (
        <span />
      )}
      {next ? (
        <Link
          to="/docs/$"
          params={{ _splat: next.slug }}
          className="text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          {next.title} &rarr;
        </Link>
      ) : (
        <span />
      )}
    </div>
  )
}
