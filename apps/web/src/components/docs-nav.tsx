import { Link } from '@tanstack/react-router'
import type { DocMeta } from '~/lib/content'
import { ArrowLeftIcon, ArrowRightIcon } from './icons'

interface DocsNavProps {
  docs: DocMeta[]
  currentSlug: string
}

const cardClass =
  'group surface-card flex flex-col gap-1 rounded-xl px-5 py-4 transition-[box-shadow] duration-150 ease'

export function DocsNav({ docs, currentSlug }: DocsNavProps) {
  const currentIndex = docs.findIndex((d) => d.slug === currentSlug)
  const prev = currentIndex > 0 ? docs[currentIndex - 1] : null
  const next = currentIndex < docs.length - 1 ? docs[currentIndex + 1] : null

  return (
    <nav className="mt-16 grid gap-3 sm:grid-cols-2" aria-label="Pagination">
      {prev ? (
        <Link to="/docs/$" params={{ _splat: prev.slug }} className={cardClass}>
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <ArrowLeftIcon className="size-3.5 transition-transform duration-200 ease-out group-hover:-translate-x-0.5" />
            Previous
          </span>
          <span className="font-medium">{prev.title}</span>
        </Link>
      ) : (
        <span className="hidden sm:block" />
      )}
      {next && (
        <Link
          to="/docs/$"
          params={{ _splat: next.slug }}
          className={`${cardClass} items-end text-right`}
        >
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            Next
            <ArrowRightIcon className="size-3.5 transition-transform duration-200 ease-out group-hover:translate-x-0.5" />
          </span>
          <span className="font-medium">{next.title}</span>
        </Link>
      )}
    </nav>
  )
}
