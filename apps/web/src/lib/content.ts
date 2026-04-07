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

async function getContentDir() {
  const { join } = await import('node:path')
  return join(process.cwd(), 'content', 'docs')
}

function injectHeadingIds(html: string): string {
  return html.replace(/<(h[23])>([\s\S]*?)<\/\1>/gi, (_, tag, inner) => {
    const text = inner.replace(/<[^>]+>/g, '').trim()
    const id = text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')
    return `<${tag} id="${id}">${inner}</${tag}>`
  })
}

export async function getDocSlugs(): Promise<string[]> {
  const { readdir } = await import('node:fs/promises')
  const { basename } = await import('node:path')
  const contentDir = await getContentDir()
  const files = await readdir(contentDir)
  return files.filter((f: string) => f.endsWith('.md')).map((f: string) => basename(f, '.md'))
}

export async function getDoc(slug: string): Promise<DocEntry | null> {
  try {
    const { readFile } = await import('node:fs/promises')
    const { join } = await import('node:path')
    const contentDir = await getContentDir()
    const raw = await readFile(join(contentDir, `${slug}.md`), 'utf-8')
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
