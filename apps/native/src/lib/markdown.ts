import { dirname, isAbsolute, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { Lexer, type Token, type Tokens } from 'marked'
import { htmlToMarkdown } from './html'
import { imageSize } from './image-size'

export type AlertKind = 'note' | 'tip' | 'important' | 'warning' | 'caution'

export type Block =
  | { kind: 'markdown'; id: string; source: string; text: string }
  | { kind: 'heading'; id: string; level: number; source: string; text: string; slug: string }
  | { kind: 'code'; id: string; language: string; code: string; text: string }
  | { kind: 'alert'; id: string; alert: AlertKind; source: string; text: string }
  | {
      kind: 'image'
      id: string
      src: string | null
      alt: string
      width: number
      height: number
      text: string
    }
  | { kind: 'rule'; id: string; text: string }
  | { kind: 'footnotes'; id: string; items: Footnote[]; text: string }

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
      text: footnotes.map((note) => `${note.marker} ${note.source}`).join('\n'),
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
  return parseMarkdown(htmlToMarkdown(html, dirname(filePath)), filePath)
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
      ctx.blocks.push({ kind: 'code', id, language, code: code.text, text: code.text })
      return
    }
    case 'hr':
      ctx.blocks.push({ kind: 'rule', id, text: '' })
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
          text: alert.body,
        })
        return
      }
      break
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
      ctx.blocks.push({
        kind: 'markdown',
        id,
        source: taskMarkers(token.raw.trimEnd()),
        text: blockText(token),
      })
      return
  }
  const source = token.raw.trimEnd()
  if (!source.trim()) return
  ctx.blocks.push({ kind: 'markdown', id, source, text: blockText(token) })
}

function imageBlock(ctx: BuildContext, id: string, image: Tokens.Image): Block {
  const src = resolveLocalImage(image.href, ctx.baseDir)
  const size = src ? imageSize(src) : null
  return {
    kind: 'image',
    id,
    src: size ? src : null,
    alt: image.text,
    width: size?.width ?? 0,
    height: size?.height ?? 0,
    text: image.text,
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

function decodeEntities(text: string) {
  return text
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
}
