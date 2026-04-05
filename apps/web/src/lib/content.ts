import { readdir, readFile } from 'node:fs/promises'
import { join, basename } from 'node:path'

export interface DocMeta {
  slug: string
  title: string
  description: string
  category: string
  order: number
}

export interface DocEntry {
  meta: DocMeta
  raw: string
}

const CONTENT_DIR = join(process.cwd(), 'content', 'docs')

export async function getDocSlugs(): Promise<string[]> {
  const files = await readdir(CONTENT_DIR)
  return files.filter((f) => f.endsWith('.md')).map((f) => basename(f, '.md'))
}

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

export async function getDoc(slug: string): Promise<DocEntry | null> {
  try {
    const raw = await readFile(join(CONTENT_DIR, `${slug}.md`), 'utf-8')
    const { frontmatter, body } = parseFrontmatter(raw)
    return {
      meta: {
        slug,
        title: frontmatter.title || slug,
        description: frontmatter.description || '',
        category: frontmatter.category || 'General',
        order: parseInt(frontmatter.order || '99', 10),
      },
      raw: body,
    }
  } catch {
    return null
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

export function groupByCategory(docs: DocMeta[]): { category: string; docs: DocMeta[] }[] {
  const map = new Map<string, DocMeta[]>()
  for (const doc of docs) {
    const list = map.get(doc.category) || []
    list.push(doc)
    map.set(doc.category, list)
  }
  return [...map.entries()].map(([category, docs]) => ({ category, docs }))
}
