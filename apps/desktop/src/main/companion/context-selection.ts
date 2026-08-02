const DEFAULT_MAX_BYTES = 16_384
const MAX_MAP_BYTES = 4_096
const MAX_LINK_MANIFEST_BYTES = 2_048

const STOP_WORDS = new Set([
  'about',
  'also',
  'does',
  'from',
  'have',
  'into',
  'that',
  'their',
  'this',
  'what',
  'when',
  'where',
  'which',
  'with',
  'would',
])

export interface MarkdownHeading {
  depth: number
  text: string
  line: number
}

export interface MarkdownLink {
  label: string
  target: string
}

export interface SelectedMarkdown {
  excerpt: string
  bytes: number
  wholeDocument: boolean
  headings: MarkdownHeading[]
  links: MarkdownLink[]
}

interface HeadingPosition extends MarkdownHeading {
  lineIndex: number
}

interface MarkdownSection {
  start: number
  text: string
  score: number
}

function byteLength(value: string): number {
  return Buffer.byteLength(value, 'utf8')
}

function truncateUtf8(value: string, maxBytes: number): string {
  if (maxBytes <= 0) return ''
  if (byteLength(value) <= maxBytes) return value
  let end = Math.min(value.length, maxBytes)
  while (end > 0 && byteLength(value.slice(0, end)) > maxBytes) end -= 1
  return value.slice(0, end)
}

function parseHeadings(lines: string[]): HeadingPosition[] {
  const headings: HeadingPosition[] = []
  lines.forEach((line, lineIndex) => {
    const match = /^(#{1,6})\s+(.+?)\s*#*\s*$/.exec(line)
    if (!match) return
    headings.push({
      depth: match[1].length,
      text: match[2].trim(),
      line: lineIndex + 1,
      lineIndex,
    })
  })
  return headings
}

function isLocalLink(target: string): boolean {
  const normalized = target.trim().toLowerCase()
  return Boolean(
    normalized &&
    !normalized.startsWith('#') &&
    !normalized.startsWith('http:') &&
    !normalized.startsWith('https:') &&
    !normalized.startsWith('mailto:') &&
    !normalized.startsWith('data:'),
  )
}

function extractLinks(content: string): MarkdownLink[] {
  const links: MarkdownLink[] = []
  const seen = new Set<string>()
  const references = new Map<string, string>()

  for (const match of content.matchAll(/^\s*\[([^\]]+)\]:\s*(\S+)/gm)) {
    references.set(match[1].trim().toLowerCase(), match[2].replace(/^<|>$/g, ''))
  }

  const add = (label: string, target: string): void => {
    if (!isLocalLink(target)) return
    const key = `${label}\u0000${target}`
    if (seen.has(key)) return
    seen.add(key)
    links.push({ label, target })
  }

  for (const match of content.matchAll(/(?<!!)\[([^\]]+)\]\(([^)\s]+)(?:\s+['"][^'"]*['"])?\)/g)) {
    add(match[1].trim(), match[2].replace(/^<|>$/g, ''))
  }

  for (const match of content.matchAll(/(?<!!)\[([^\]]+)\]\[([^\]]*)\]/g)) {
    const label = match[1].trim()
    const reference = (match[2].trim() || label).toLowerCase()
    const target = references.get(reference)
    if (target) add(label, target)
  }

  return links
}

function questionTerms(question: string): string[] {
  return [
    ...new Set(
      question
        .toLowerCase()
        .match(/[\p{L}\p{N}_-]{3,}/gu)
        ?.filter((term) => !STOP_WORDS.has(term)) ?? [],
    ),
  ]
}

function countOccurrences(text: string, term: string): number {
  let count = 0
  let offset = 0
  while (offset < text.length) {
    const index = text.indexOf(term, offset)
    if (index === -1) break
    count += 1
    offset = index + term.length
  }
  return count
}

function buildSections(lines: string[], headings: HeadingPosition[], terms: string[]): MarkdownSection[] {
  const sections: MarkdownSection[] = []
  for (let index = 0; index < headings.length; index += 1) {
    const heading = headings[index]
    let end = lines.length
    for (let next = index + 1; next < headings.length; next += 1) {
      if (headings[next].depth <= heading.depth) {
        end = headings[next].lineIndex
        break
      }
    }
    const text = lines.slice(heading.lineIndex, end).join('\n').trim()
    const normalizedHeading = heading.text.toLowerCase()
    const normalizedText = text.toLowerCase()
    const score = terms.reduce(
      (total, term) =>
        total +
        (normalizedHeading.includes(term) ? 8 : 0) +
        Math.min(countOccurrences(normalizedText, term), 4),
      0,
    )
    sections.push({ start: heading.lineIndex, text, score })
  }
  return sections
}

function formatHeadingMap(headings: MarkdownHeading[]): string {
  if (headings.length === 0) return ''
  const rows = headings.map(
    (heading) => `${'  '.repeat(Math.max(heading.depth - 1, 0))}- ${heading.text} (line ${heading.line})`,
  )
  return truncateUtf8(`## Document map\n${rows.join('\n')}`, MAX_MAP_BYTES)
}

function formatLinkManifest(links: MarkdownLink[]): string {
  if (links.length === 0) return ''
  const rows = links.map((link) => `- ${link.label}: ${link.target}`)
  return truncateUtf8(`## Linked document manifest\n${rows.join('\n')}`, MAX_LINK_MANIFEST_BYTES)
}

export function selectInitialMarkdown(
  content: string,
  question: string,
  maxBytes = DEFAULT_MAX_BYTES,
): SelectedMarkdown {
  const lines = content.split(/\r?\n/)
  const headingPositions = parseHeadings(lines)
  const headings = headingPositions.map(({ depth, text, line }) => ({ depth, text, line }))
  const links = extractLinks(content)

  if (byteLength(content) <= maxBytes) {
    return {
      excerpt: content,
      bytes: byteLength(content),
      wholeDocument: true,
      headings,
      links,
    }
  }

  const terms = questionTerms(question)
  const sections = buildSections(lines, headingPositions, terms)
  const candidates = sections.length > 1 ? sections.slice(1) : sections
  const relevant = candidates.filter((section) => section.score > 0)
  const ranked = (relevant.length > 0 ? relevant : candidates)
    .toSorted((a, b) => b.score - a.score || a.start - b.start)
    .slice(0, 3)
    .toSorted((a, b) => a.start - b.start)

  const blocks = [formatHeadingMap(headings), formatLinkManifest(links), '## Selected sections']
    .filter(Boolean)
    .join('\n\n')
  let excerpt = truncateUtf8(blocks, maxBytes)

  for (const section of ranked) {
    const separator = excerpt ? '\n\n' : ''
    const remaining = maxBytes - byteLength(excerpt) - byteLength(separator)
    if (remaining <= 0) break
    const selected = truncateUtf8(section.text, remaining)
    if (!selected) break
    excerpt += `${separator}${selected}`
  }

  return {
    excerpt,
    bytes: byteLength(excerpt),
    wholeDocument: false,
    headings,
    links,
  }
}
