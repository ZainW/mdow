import type { ReactNode } from 'react'
import type { DocMeta } from '~/lib/content'
import { DocsSidebar } from './docs-sidebar'
import { DocsToc } from './docs-toc'
import { DocsSearch } from './docs-search'
import { DocsMobileNav } from './docs-mobile-nav'
import type { TocItem } from './docs-toc'

interface DocsLayoutProps {
  docs: DocMeta[]
  currentSlug: string
  headings: TocItem[]
  children: ReactNode
}

export function DocsLayout({ docs, currentSlug, headings, children }: DocsLayoutProps) {
  return (
    <div className="mx-auto max-w-6xl px-6 py-8">
      <div className="mb-6 flex items-center gap-3 lg:hidden">
        <DocsMobileNav docs={docs} currentSlug={currentSlug} />
        <div className="flex-1">
          <DocsSearch docs={docs} />
        </div>
      </div>
      <div className="flex gap-10">
        <div className="hidden w-56 shrink-0 space-y-4 lg:block">
          <DocsSearch docs={docs} />
          <DocsSidebar docs={docs} currentSlug={currentSlug} />
        </div>
        <article className="prose prose-neutral min-w-0 max-w-none flex-1 dark:prose-invert">
          {children}
        </article>
        <DocsToc headings={headings} />
      </div>
    </div>
  )
}
