import { dirname, isAbsolute, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { Lexer, type Token, type Tokens } from 'marked'
import { htmlTitle, htmlToMarkdown } from './html'
import { dataImageSize, imageSize } from './image-size'

export type AlertKind = 'note' | 'tip' | 'important' | 'warning' | 'caution'

/**
 * One reader row. A long list, table or code fence is split across several rows so no single row
 * costs more than a frame to lay out; `joinNext` marks a row whose continuation follows directly.
 */
export type Block = BlockBody & { id: string; joinNext?: boolean }

export type BlockBody =
  | { kind: 'markdown'; source: string }
  | { kind: 'heading'; level: number; source: string; slug: string; text: string }
  | {
      kind: 'code'
      language: string
      code: string
      /** Where this row sits in a fence split across rows. */
      part: 'whole' | 'first' | 'middle' | 'last'
      /** The whole fence, for Copy. Set on the row that shows the header. */
      copy: string
    }
  | { kind: 'alert'; alert: AlertKind; source: string }
  /** A plain blockquote; `source` is its inner markdown. */
  | { kind: 'quote'; source: string }
  | { kind: 'image'; src: string | null; alt: string; width: number; height: number }
  | { kind: 'rule' }
  | { kind: 'footnotes'; items: Footnote[] }
  | {
      kind: 'table'
      /** Inline markdown per cell. Only the first row of a split table has a header. */
      header: string[] | null
      rows: string[][]
      align: (TableAlign | null)[]
      /** Column share of the reading width, summing to 1, identical across a table's rows. */
      widths: number[]
    }

export type TableAlign = 'left' | 'center' | 'right'

/** Plain search text is derived only when Find is open, not retained for every reader block. */
export function searchableText(block: Block): string {
  switch (block.kind) {
    case 'markdown':
    case 'quote':
      return sourceText(block.source)
    case 'heading':
      return block.text
    case 'code':
      return block.code
    case 'alert':
      return block.source
    case 'image':
      return block.alt
    case 'rule':
      return ''
    case 'footnotes':
      return block.items.map((note) => `${note.marker} ${note.source}`).join('\n')
    case 'table':
      return [...(block.header ? [block.header] : []), ...block.rows]
        .map((row) => row.map(inlineSearchText).join('\n'))
        .join('\n')
  }
}

/** A list is cut after this many items or characters, whichever comes first. */
export const LIST_CHUNK_ITEMS = 12
export const CHUNK_CHARS = 4000
/** Code fences longer than this are cut into rows, preferring blank lines as seams. */
export const CODE_CHUNK_LINES = 40
/** Tables are drawn by the reader, in slices of this many body rows. */
export const TABLE_CHUNK_ROWS = 24

export interface Footnote {
  label: string
  marker: string
  source: string
}

export interface OutlineEntry {
  level: number
  text: string
  slug: string
  blockIndex: number
}

export interface ParsedDocument {
  title: string | null
  blocks: Block[]
  outline: OutlineEntry[]
  slugs: Map<string, number>
}

const ALERT_RE = /^\[!(note|tip|important|warning|caution)\]\s*$/i
const SUPERSCRIPT = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹']

export function parseMarkdown(markdown: string, filePath: string): ParsedDocument {
  const { body, title } = stripFrontmatter(markdown)
  const { source, footnotes } = extractFootnotes(body)
  const blocks: Block[] = []
  const ctx: BuildContext = { baseDir: dirname(filePath), nextId: 0, blocks, depth: 0 }
  pushTokens(ctx, new Lexer({ gfm: true }).lex(source))

  if (footnotes.length > 0) {
    blocks.push({
      kind: 'footnotes',
      id: `b${ctx.nextId++}`,
      items: footnotes,
    })
  }

  const outline: OutlineEntry[] = []
  const slugs = new Map<string, number>()
  const seen = new Map<string, number>()
  blocks.forEach((block, index) => {
    if (block.kind !== 'heading') return
    const base = slugify(block.text)
    const count = seen.get(base) ?? 0
    seen.set(base, count + 1)
    block.slug = count === 0 ? base : `${base}-${count}`
    slugs.set(block.slug, index)
    outline.push({ level: block.level, text: block.text, slug: block.slug, blockIndex: index })
  })

  return { title, blocks, outline, slugs }
}

export function parseHtml(html: string, filePath: string): ParsedDocument {
  const parsed = parseMarkdown(htmlToMarkdown(html, dirname(filePath)), filePath)
  return { ...parsed, title: parsed.title ?? htmlTitle(html) }
}

interface BuildContext {
  baseDir: string
  nextId: number
  blocks: Block[]
  depth: number
}

function pushTokens(ctx: BuildContext, tokens: Token[]) {
  for (const token of tokens) pushToken(ctx, token)
}

function pushToken(ctx: BuildContext, token: Token) {
  const id = `b${ctx.nextId++}`
  switch (token.type) {
    case 'space':
    case 'def':
      return
    case 'heading': {
      const heading = token as Tokens.Heading
      ctx.blocks.push({
        kind: 'heading',
        id,
        level: heading.depth,
        source: heading.raw.trim(),
        text: inlineText(heading.tokens),
        slug: '',
      })
      return
    }
    case 'code': {
      const code = token as Tokens.Code
      const language = (code.lang ?? '').trim().split(/\s+/)[0]?.toLowerCase() ?? ''
      pushCode(ctx, id, language, code.text)
      return
    }
    case 'hr':
      ctx.blocks.push({ kind: 'rule', id })
      return
    case 'blockquote': {
      const quote = token as Tokens.Blockquote
      const alert = alertOf(quote)
      if (alert) {
        ctx.blocks.push({
          kind: 'alert',
          id,
          alert: alert.kind,
          source: alert.body,
        })
        return
      }
      const inner = quote.raw
        .split('\n')
        .map((line) => line.replace(/^\s*>\s?/, ''))
        .join('\n')
        .trim()
      ctx.blocks.push({ kind: 'quote', id, source: inner })
      return
    }
    case 'paragraph': {
      const images = standaloneImages(token as Tokens.Paragraph)
      if (images) {
        images.forEach((image, index) => {
          ctx.blocks.push(imageBlock(ctx, index === 0 ? id : `b${ctx.nextId++}`, image))
        })
        return
      }
      break
    }
    case 'html': {
      // Raw HTML (README banners, <details>, centered images) goes through the same lenient
      // converter as .html files, so it reads as content instead of tags.
      if (ctx.depth < 2) {
        const converted = htmlToMarkdown((token as Tokens.HTML).raw, ctx.baseDir)
        if (converted.trim()) {
          ctx.depth++
          pushTokens(ctx, new Lexer({ gfm: true }).lex(converted))
          ctx.depth--
        }
      }
      return
    }
    case 'list':
      pushList(ctx, id, token as Tokens.List)
      return
    case 'table':
      pushTable(ctx, id, token as Tokens.Table)
      return
  }
  const source = token.raw.trimEnd()
  if (!source.trim()) return
  ctx.blocks.push({ kind: 'markdown', id, source })
}

/** Mermaid is never split: the diagram needs the whole source. */
function pushCode(ctx: BuildContext, id: string, language: string, code: string) {
  const lines = code.split('\n')
  if (language === 'mermaid' || lines.length <= CODE_CHUNK_LINES * 1.5) {
    ctx.blocks.push({ kind: 'code', id, language, code, part: 'whole', copy: code })
    return
  }
  const pieces: string[] = []
  let start = 0
  while (start < lines.length) {
    let end = Math.min(lines.length, start + CODE_CHUNK_LINES)
    if (lines.length - end < CODE_CHUNK_LINES / 2) end = lines.length
    else {
      // A blank line keeps multi-line strings and comments from straddling the seam.
      for (let at = end; at > start + CODE_CHUNK_LINES / 2; at--) {
        if (!lines[at - 1]!.trim()) {
          end = at
          break
        }
      }
    }
    pieces.push(lines.slice(start, end).join('\n'))
    start = end
  }
  pieces.forEach((piece, index) => {
    const last = index === pieces.length - 1
    ctx.blocks.push({
      kind: 'code',
      id: index === 0 ? id : `b${ctx.nextId++}`,
      language,
      code: piece,
      part: index === 0 ? 'first' : last ? 'last' : 'middle',
      copy: index === 0 ? code : '',
      joinNext: !last,
    })
  })
}

function pushList(ctx: BuildContext, id: string, list: Tokens.List) {
  const groups: Tokens.ListItem[][] = [[]]
  let chars = 0
  for (const item of list.items) {
    const group = groups.at(-1)!
    if (
      group.length >= LIST_CHUNK_ITEMS ||
      (group.length > 0 && chars + item.raw.length > CHUNK_CHARS)
    ) {
      groups.push([item])
      chars = item.raw.length
    } else {
      group.push(item)
      chars += item.raw.length
    }
  }
  const first = typeof list.start === 'number' ? list.start : 1
  let before = 0
  groups.forEach((items, index) => {
    const last = index === groups.length - 1
    let raw = items.map((item) => item.raw).join('')
    // A slice numbers from its first marker, so give it the item's real position. Lists that
    // number every item `1.` would otherwise restart at 1 in each slice.
    if (list.ordered && index > 0) {
      const number = first + before
      raw = /^\s*\d+[.)]/.test(raw)
        ? raw.replace(/^(\s*)\d+([.)])/, `$1${number}$2`)
        : `${number}. ${raw}`
    }
    before += items.length
    ctx.blocks.push({
      kind: 'markdown',
      id: index === 0 ? id : `b${ctx.nextId++}`,
      source: taskMarkers(raw.trimEnd()),
      joinNext: !last,
    })
  })
}

function pushTable(ctx: BuildContext, id: string, table: Tokens.Table) {
  const cell = (value: Tokens.TableCell) => value.text.trim()
  const header = table.header.map(cell)
  const rows = table.rows.map((row) => row.map(cell))
  const columns = Math.max(header.length, ...rows.map((row) => row.length))
  // Weight columns roughly like auto table layout: typical content width plus the cell's
  // padding, and never narrower than the (smaller, uppercase) header label.
  const weights = Array.from({ length: columns }, (_, column) => {
    const lengths = rows.map((row) => plainLength(row[column] ?? ''))
    const sorted = lengths.toSorted((a, b) => a - b)
    const typical = sorted[Math.floor(sorted.length * 0.9)] ?? 0
    const label = plainLength(header[column] ?? '') * 0.85
    return Math.max(3, label, Math.min(60, typical)) + 4
  })
  const total = weights.reduce((sum, weight) => sum + weight, 0)
  const widths = weights.map((weight) => weight / total)
  const align = Array.from({ length: columns }, (_, column) => table.align[column] ?? null)
  // A header-only table still gets one (empty) slice.
  for (let start = 0; start === 0 || start < rows.length; start += TABLE_CHUNK_ROWS) {
    const slice = rows.slice(start, start + TABLE_CHUNK_ROWS)
    const last = start + TABLE_CHUNK_ROWS >= rows.length
    ctx.blocks.push({
      kind: 'table',
      id: start === 0 ? id : `b${ctx.nextId++}`,
      header: start === 0 ? header : null,
      rows: slice,
      align,
      widths,
      joinNext: !last,
    })
  }
}

function plainLength(source: string) {
  return source.replace(/[`*_~]|\]\([^)]*\)|!?\[/g, '').length
}

function imageBlock(ctx: BuildContext, id: string, image: Tokens.Image): Block {
  const inline = /^data:image\//i.test(image.href)
  const src = inline ? image.href : resolveLocalImage(image.href, ctx.baseDir)
  const size = inline ? dataImageSize(image.href) : src ? imageSize(src) : null
  return {
    kind: 'image',
    id,
    src: size ? src : null,
    alt: image.text,
    width: size?.width ?? 0,
    height: size?.height ?? 0,
  }
}

export function resolveLocalImage(href: string, baseDir: string): string | null {
  if (!href || /^(https?:|data:|mailto:)/i.test(href)) return null
  try {
    const path = href.startsWith('file:') ? fileURLToPath(href) : decodeURI(href.split(/[?#]/)[0]!)
    return isAbsolute(path) ? path : resolve(baseDir, path)
  } catch {
    return null
  }
}

function standaloneImages(paragraph: Tokens.Paragraph): Tokens.Image[] | null {
  const images: Tokens.Image[] = []
  for (const token of paragraph.tokens) {
    if (token.type === 'image') images.push(token as Tokens.Image)
    else if (token.type === 'link' && isImageLink(token as Tokens.Link)) {
      images.push((token as Tokens.Link).tokens[0] as Tokens.Image)
    } else if (!(token.type === 'br' || (token.type === 'text' && !token.raw.trim()))) return null
  }
  return images.length > 0 ? images : null
}

function isImageLink(link: Tokens.Link) {
  return link.tokens.length === 1 && link.tokens[0]?.type === 'image'
}

function alertOf(quote: Tokens.Blockquote): { kind: AlertKind; body: string } | null {
  const lines = quote.raw.split('\n').map((line) => line.replace(/^\s*>\s?/, ''))
  const match = ALERT_RE.exec(lines[0]?.trim() ?? '')
  if (!match) return null
  return {
    kind: match[1]!.toLowerCase() as AlertKind,
    body: lines.slice(1).join('\n').trim(),
  }
}

/** The native <markdown> element prints `[x]` literally, so render the box as a glyph. */
function taskMarkers(source: string) {
  return source.replace(/^(\s*(?:[-*+]|\d+[.)])\s+)\[([ xX])\]\s/gm, (_, bullet: string, mark) =>
    mark === ' ' ? `${bullet}☐ ` : `${bullet}☑ `,
  )
}

export function stripFrontmatter(markdown: string): { body: string; title: string | null } {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\s*(?:\r?\n|$)/.exec(markdown)
  if (!match) return { body: markdown, title: null }
  const titleLine = /^title:\s*(.+)$/m.exec(match[1]!)
  const title = titleLine ? titleLine[1]!.trim().replace(/^(['"])(.*)\1$/, '$2') || null : null
  return { body: markdown.slice(match[0].length), title }
}

function extractFootnotes(markdown: string): { source: string; footnotes: Footnote[] } {
  const definitions = new Map<string, string>()
  const kept: string[] = []
  let fence: string | null = null
  let current: string | null = null

  for (const line of markdown.split('\n')) {
    const fenceMatch = /^\s*(`{3,}|~{3,})/.exec(line)
    if (fenceMatch) {
      if (fence === null) fence = fenceMatch[1]!.charAt(0)
      else if (fenceMatch[1]!.startsWith(fence)) fence = null
    }
    if (fence === null) {
      const def = /^\[\^([^\]\s]+)\]:\s?(.*)$/.exec(line)
      if (def) {
        current = def[1]!
        definitions.set(current, def[2]!)
        continue
      }
      if (current !== null && /^( {2,}|\t)\S/.test(line)) {
        definitions.set(current, `${definitions.get(current)} ${line.trim()}`)
        continue
      }
    }
    current = null
    kept.push(line)
  }

  if (definitions.size === 0) return { source: markdown, footnotes: [] }

  const order: string[] = []
  const source = kept
    .join('\n')
    .replace(/(`+)[\s\S]*?\1|\[\^([^\]\s]+)\]/g, (match, ticks: string | undefined, label) => {
      if (ticks || !definitions.has(label)) return match
      if (!order.includes(label)) order.push(label)
      return footnoteMarker(label, order.indexOf(label) + 1)
    })
  for (const label of definitions.keys()) if (!order.includes(label)) order.push(label)

  return {
    source,
    footnotes: order.map((label, index) => ({
      label,
      marker: footnoteMarker(label, index + 1),
      source: definitions.get(label) ?? '',
    })),
  }
}

function footnoteMarker(label: string, ordinal: number) {
  return /^\d+$/.test(label)
    ? String(ordinal)
        .split('')
        .map((digit) => SUPERSCRIPT[Number(digit)])
        .join('')
    : `[${label}]`
}

export function slugify(text: string) {
  return text
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, '')
    .replace(/\s/g, '-')
}

function inlineText(tokens: Token[] | undefined): string {
  if (!tokens) return ''
  return tokens
    .map((token) => {
      if ('tokens' in token && token.tokens) return inlineText(token.tokens)
      if (token.type === 'br') return '\n'
      return 'text' in token ? decodeEntities(String(token.text)) : ''
    })
    .join('')
}

/** Plain text of a block in painted order, one entry per line, used for find counts. */
function blockText(token: Token): string {
  switch (token.type) {
    case 'list':
      return (token as Tokens.List).items
        .map((item) => item.tokens.map(blockText).join('\n'))
        .join('\n')
    case 'table': {
      const table = token as Tokens.Table
      return [table.header, ...table.rows]
        .map((row) => row.map((cell) => inlineText(cell.tokens)).join('\n'))
        .join('\n')
    }
    case 'blockquote':
      return (token as Tokens.Blockquote).tokens.map(blockText).join('\n')
    case 'code':
      return (token as Tokens.Code).text
    default:
      if ('tokens' in token && token.tokens) return inlineText(token.tokens)
      return 'text' in token ? String(token.text) : ''
  }
}

function sourceText(source: string): string {
  return new Lexer({ gfm: true }).lex(source).map(blockText).join('\n')
}

function inlineSearchText(source: string): string {
  return inlineText(new Lexer({ gfm: true }).inlineTokens(source))
}

function decodeEntities(text: string) {
  return text
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
}
