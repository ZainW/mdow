import { useCallback, useState, type ReactNode } from 'react'
import type { DocMeta } from '~/lib/content'
import type { SearchEntry } from '~/lib/search-index'
import { DocsMobileNav } from './docs-mobile-nav'
import { DocsSearchDialog, DocsSearchTrigger } from './docs-search'
import { DocsSidebar } from './docs-sidebar'
import { DocsToc, type TocItem } from './docs-toc'

interface DocsLayoutProps {
  docs: DocMeta[]
  searchEntries: SearchEntry[]
  currentSlug: string
  headings: TocItem[]
  children: ReactNode
}

export function DocsLayout({
  docs,
  searchEntries,
  currentSlug,
  headings,
  children,
}: DocsLayoutProps) {
  const [searchOpen, setSearchOpen] = useState(false)
  const openSearch = useCallback(() => setSearchOpen(true), [])

  return (
    <div className="mx-auto max-w-6xl px-5 sm:px-6">
      <div className="sticky top-[calc(4rem+env(safe-area-inset-top))] z-dropdown -mx-5 flex items-center gap-2 border-b border-border-subtle bg-background/90 px-5 py-2.5 backdrop-blur-lg sm:-mx-6 sm:px-6 lg:hidden">
        <DocsMobileNav docs={docs} currentSlug={currentSlug} />
        <DocsSearchTrigger onOpen={openSearch} className="h-9 w-auto flex-1" />
      </div>

      <div className="flex gap-10">
        <aside className="hidden w-52 shrink-0 lg:block">
          <div className="sticky top-16 max-h-[calc(100dvh-4rem)] space-y-6 overflow-y-auto pb-10 pt-10">
            <DocsSearchTrigger onOpen={openSearch} />
            <DocsSidebar docs={docs} currentSlug={currentSlug} />
          </div>
        </aside>

        <div className="min-w-0 flex-1 pb-24 pt-8 lg:pt-10">{children}</div>

        <DocsToc headings={headings} />
      </div>

      <DocsSearchDialog entries={searchEntries} open={searchOpen} onOpenChange={setSearchOpen} />
    </div>
  )
}
