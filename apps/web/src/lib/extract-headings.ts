import { slugify } from './slugify'

export interface ExtractedHeading {
  id: string
  text: string
  level: 2 | 3
}

/**
 * Extracts h2 and h3 headings from a rendered markdown HTML string.
 * Pure regex parsing — runs on the server during loader execution.
 * Assigns slugified ids when the heading lacks one.
 */
export function extractHeadings(html: string): ExtractedHeading[] {
  const headings: ExtractedHeading[] = []
  const re = /<h([23])(?:\s+([^>]*))?>([\s\S]*?)<\/h\1>/gi
  let match: RegExpExecArray | null
  while ((match = re.exec(html)) !== null) {
    const level = parseInt(match[1], 10) as 2 | 3
    const attrs = match[2] || ''
    const text = match[3].replace(/<[^>]+>/g, '').trim()
    const idMatch = attrs.match(/\bid="([^"]+)"/)
    const id = idMatch ? idMatch[1] : slugify(text)
    headings.push({ id, text, level })
  }
  return headings
}
