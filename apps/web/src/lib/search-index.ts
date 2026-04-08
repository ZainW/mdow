import Fuse from 'fuse.js'
import type { DocMeta } from './content'

export interface SearchEntry {
  slug: string
  title: string
  description: string
  category: string
}

let fuse: Fuse<SearchEntry> | null = null

export function buildSearchIndex(docs: DocMeta[]) {
  const entries: SearchEntry[] = docs.map((d) => ({
    slug: d.slug,
    title: d.title,
    description: d.description,
    category: d.category,
  }))

  fuse = new Fuse(entries, {
    keys: [
      { name: 'title', weight: 2 },
      { name: 'description', weight: 1 },
      { name: 'category', weight: 0.5 },
    ],
    threshold: 0.4,
  })

  return fuse
}

export function search(query: string): SearchEntry[] {
  if (!fuse || !query.trim()) return []
  return fuse.search(query).map((r) => r.item)
}
