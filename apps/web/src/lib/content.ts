import { slugify } from './slugify'

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

function injectHeadingIds(html: string): string {
  return html.replace(/<(h[23])>([\s\S]*?)<\/\1>/gi, (_, tag, inner) => {
    const text = inner.replace(/<[^>]+>/g, '').trim()
    const id = slugify(text)
    return `<${tag} id="${id}">${inner}</${tag}>`
  })
}

function slugFromPath(path: string): string {
  const file = path.split('/').pop() ?? ''
  return file.replace(/\.md$/, '')
}

export async function getDocSlugs(): Promise<string[]> {
  return Object.keys(docFiles).map(slugFromPath)
}

export async function getDoc(slug: string): Promise<DocEntry | null> {
  const entry = Object.entries(docFiles).find(([path]) => slugFromPath(path) === slug)
  if (!entry) return null

  const [, raw] = entry
  const { frontmatter, body } = parseFrontmatter(raw)
  const { renderToHtml, init } = await import('md4x')
  await init()
  const html = injectHeadingIds(renderToHtml(body))
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

export function groupByCategory(docs: DocMeta[]): { category: string; docs: DocMeta[] }[] {
  const map = new Map<string, DocMeta[]>()
  for (const doc of docs) {
    const list = map.get(doc.category) || []
    list.push(doc)
    map.set(doc.category, list)
  }
  return [...map.entries()].map(([category, docs]) => ({ category, docs }))
}
