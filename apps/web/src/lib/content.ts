import { extractHeadings } from './extract-headings'
import type { SearchEntry } from './search-index'

export interface DocMeta {
  slug: string
  title: string
  description: string
  category: string
  order: number
}

export interface DocEntry {
  meta: DocMeta
  html: string
}

// Bundle all docs/*.md files at build time as raw strings. This eliminates
// runtime filesystem access — necessary for Cloudflare Workers, which have
// no real fs and where process.cwd() resolves to a virtual /bundle path.
const docFiles = import.meta.glob('../../content/docs/*.md', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

function parseFrontmatter(raw: string): {
  frontmatter: Record<string, string>
  body: string
} {
  const match = raw.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/)
  if (!match) return { frontmatter: {}, body: raw }

  const frontmatter: Record<string, string> = {}
  for (const line of match[1].split('\n')) {
    const [key, ...rest] = line.split(':')
    if (key && rest.length) {
      frontmatter[key.trim()] = rest.join(':').trim()
    }
  }
  return { frontmatter, body: match[2] }
}

function slugFromPath(path: string): string {
  const file = path.split('/').pop() ?? ''
  return file.replace(/\.md$/, '')
}

export async function getDoc(slug: string): Promise<DocEntry | null> {
  const entry = Object.entries(docFiles).find(([path]) => slugFromPath(path) === slug)
  if (!entry) return null

  const [, raw] = entry
  const { frontmatter, body } = parseFrontmatter(raw)
  const { renderToHtml, init } = await import('md4x')
  await init()
  const html = renderToHtml(body, { headingIds: true })
  return {
    meta: {
      slug,
      title: frontmatter.title || slug,
      description: frontmatter.description || '',
      category: frontmatter.category || 'General',
      order: parseInt(frontmatter.order || '99', 10),
    },
    html,
  }
}

export async function getAllDocs(): Promise<DocMeta[]> {
  const slugs = await getDocSlugs()
  const docs: DocMeta[] = []

  for (const slug of slugs) {
    const doc = await getDoc(slug)
    if (doc) docs.push(doc.meta)
  }

  return docs.sort((a, b) => a.order - b.order)
}

export async function getDocSlugs(): Promise<string[]> {
  return Object.keys(docFiles).map(slugFromPath)
}

/** Return the raw markdown file contents (including frontmatter) for a doc slug. */
export function getDocRaw(slug: string): string | null {
  const entry = Object.entries(docFiles).find(([path]) => slugFromPath(path) === slug)
  return entry ? entry[1] : null
}

export function getDocBody(slug: string): string | null {
  const raw = getDocRaw(slug)
  if (!raw) return null
  const { body } = parseFrontmatter(raw)
  return body
}

export function groupByCategory(docs: DocMeta[]): { category: string; docs: DocMeta[] }[] {
  const map = new Map<string, DocMeta[]>()
  for (const doc of docs) {
    const list = map.get(doc.category) || []
    list.push(doc)
    map.set(doc.category, list)
  }
  return [...map.entries()].map(([category, docs]) => ({ category, docs }))
}

function plainText(html: string): string {
  return html
    .replace(/<[^>]+>/g, ' ')
    .replace(/&[a-z#0-9]+;/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

/**
 * One entry per page plus one per h2 section, for client-side docs search.
 * Section entries carry their body text so queries match more than headings.
 */
export async function getSearchEntries(): Promise<SearchEntry[]> {
  const docs = await getAllDocs()
  const entries: SearchEntry[] = []
  for (const meta of docs) {
    const doc = await getDoc(meta.slug)
    if (!doc) continue
    const [intro, ...sections] = doc.html.split(/(?=<h2[\s>])/)
    entries.push({
      slug: meta.slug,
      title: meta.title,
      description: meta.description,
      category: meta.category,
      content: plainText(intro).slice(0, 600),
    })
    for (const section of sections) {
      const [heading] = extractHeadings(section)
      if (!heading || heading.level !== 2) continue
      entries.push({
        slug: meta.slug,
        title: meta.title,
        description: '',
        category: meta.category,
        section: { id: heading.id, text: heading.text },
        content: plainText(section.replace(/<h2[\s\S]*?<\/h2>/, '')).slice(0, 600),
      })
    }
  }
  return entries
}
