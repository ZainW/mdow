import Fuse from 'fuse.js'

export interface SearchEntry {
  slug: string
  /** Page title. */
  title: string
  description: string
  category: string
  /** Set when the entry is a section within a page. */
  section?: { id: string; text: string }
  /** Plain body text, matched with a low weight. */
  content?: string
}

let fuse: Fuse<SearchEntry> | null = null
let indexed: SearchEntry[] | null = null

export function buildSearchIndex(entries: SearchEntry[]) {
  if (indexed === entries && fuse) return fuse
  indexed = entries
  fuse = new Fuse(entries, {
    keys: [
      { name: 'title', weight: 2 },
      { name: 'section.text', weight: 1.5 },
      { name: 'description', weight: 1 },
      { name: 'category', weight: 0.5 },
      { name: 'content', weight: 0.4 },
    ],
    threshold: 0.35,
    ignoreLocation: true,
  })
  return fuse
}

export function search(query: string, limit = 8): SearchEntry[] {
  if (!fuse || !query.trim()) return []
  return fuse.search(query, { limit }).map((r) => r.item)
}
