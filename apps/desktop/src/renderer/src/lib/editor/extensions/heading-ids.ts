export function slugifyHeading(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
}

export function dedupeSlug(base: string, counts: Map<string, number>): string {
  const count = counts.get(base) ?? 0
  counts.set(base, count + 1)
  return count === 0 ? base : `${base}-${count}`
}

export interface DocHeading {
  level: number
  text: string
  id: string
}

export function computeHeadingIds(rootEl: HTMLElement): DocHeading[] {
  const counts = new Map<string, number>()
  const headings: DocHeading[] = []
  const els = rootEl.querySelectorAll<HTMLElement>('h1, h2, h3, h4')
  for (const el of els) {
    const text = (el.textContent ?? '').trim()
    if (!text) continue
    const base = slugifyHeading(text)
    if (!base) continue
    const id = dedupeSlug(base, counts)
    el.id = id
    headings.push({ level: Number(el.tagName.slice(1)), text, id })
  }
  return headings
}
