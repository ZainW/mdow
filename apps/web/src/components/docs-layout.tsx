import type { ReactNode } from 'react'
import type { DocMeta } from '~/lib/content'
import { DocsSidebar } from './docs-sidebar'
import { DocsToc } from './docs-toc'
import { DocsSearch } from './docs-search'
import type { TocItem } from './docs-toc'

interface DocsLayoutProps {
  docs: DocMeta[]
  currentSlug: string
  headings: TocItem[]
  children: ReactNode
}

export function DocsLayout({ docs, currentSlug, headings, children }: DocsLayoutProps) {
  return (
    <div className="mx-auto flex max-w-6xl gap-8 px-6 py-8">
      <div className="w-56 shrink-0 space-y-4">
        <DocsSearch docs={docs} />
        <DocsSidebar docs={docs} currentSlug={currentSlug} />
      </div>
      <article className="min-w-0 flex-1 prose prose-neutral dark:prose-invert max-w-none">
        {children}
      </article>
      <DocsToc headings={headings} />
    </div>
  )
}
