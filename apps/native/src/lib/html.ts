import { isAbsolute, resolve } from 'node:path'
import { parse, NodeType, type HTMLElement, type Node } from 'node-html-parser'

/**
 * Converts an HTML document into markdown so it flows through the same reader pipeline.
 *
 * Sanitizer contract: script, style, iframe, object, embed and document-head elements never
 * reach the output, and only `alt`, `src`, `href`, `class` and `start` are read, so event
 * handler attributes are dropped by construction. Relative targets are rewritten against the
 * document's folder.
 */
export function htmlToMarkdown(html: string, baseDir: string): string {
  const root = parse(html.replace(/^\s*<!doctype[^>]*>/i, ''), {
    comment: false,
    blockTextElements: { script: true, style: true },
  })
  const out: string[] = []
  new Converter(baseDir).blocks(root.childNodes, out, '')
  return (
    out
      .join('\n\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim() + '\n'
  )
}

const STRIPPED = new Set([
  'script',
  'style',
  'iframe',
  'object',
  'embed',
  'head',
  'noscript',
  'template',
  'title',
  'meta',
  'link',
  'base',
])

const INLINE = new Set([
  'a',
  'abbr',
  'b',
  'br',
  'cite',
  'code',
  'del',
  'em',
  'i',
  'img',
  'kbd',
  'mark',
  'q',
  's',
  'samp',
  'small',
  'span',
  'strike',
  'strong',
  'sub',
  'sup',
  'time',
  'u',
  'var',
])

class Converter {
  constructor(private baseDir: string) {}

  blocks(nodes: Node[], out: string[], indent: string) {
    let pending: Node[] = []
    const flush = () => {
      const text = this.inline(pending).trim()
      if (text) out.push(indent + text)
      pending = []
    }

    for (const node of nodes) {
      if (node.nodeType === NodeType.TEXT_NODE || isInline(node)) {
        pending.push(node)
        continue
      }
      if (node.nodeType !== NodeType.ELEMENT_NODE) continue
      flush()
      const element = node as HTMLElement
      const tag = element.rawTagName?.toLowerCase() ?? ''
      if (STRIPPED.has(tag)) continue

      if (tag === 'svg') {
        out.push(indent + this.svg(element))
      } else if (isMermaid(element)) {
        out.push(this.fence('mermaid', decode(element.rawText).trim(), indent))
      } else if (/^h[1-6]$/.test(tag)) {
        const text = this.inline(element.childNodes).trim()
        if (text) out.push(`${indent}${'#'.repeat(Number(tag[1]))} ${text}`)
      } else if (tag === 'p') {
        const text = this.inline(element.childNodes).trim()
        if (text) out.push(indent + text)
      } else if (tag === 'hr') {
        out.push(`${indent}---`)
      } else if (tag === 'pre') {
        out.push(this.codeFence(element, indent))
      } else if (tag === 'ul' || tag === 'ol') {
        out.push(this.list(element, indent))
      } else if (tag === 'nav') {
        // Site navigation reads as one line of links rather than a pile of runs.
        const links = element
          .querySelectorAll('a')
          .map((link) => this.inline([link]).trim())
          .filter(Boolean)
        if (links.length > 0) out.push(indent + links.join(' · '))
      } else if (tag === 'summary' || tag === 'dt') {
        const text = this.inline(element.childNodes).trim()
        if (text) out.push(indent + wrap('**', text))
      } else if (tag === 'figcaption') {
        const text = this.inline(element.childNodes).trim()
        if (text) out.push(indent + wrap('*', text))
      } else if (tag === 'blockquote' || tag === 'aside') {
        const inner: string[] = []
        this.blocks(element.childNodes, inner, '')
        out.push(
          inner
            .join('\n\n')
            .split('\n')
            .map((line) => `${indent}> ${line}`.trimEnd())
            .join('\n'),
        )
      } else if (tag === 'table') {
        const table = this.table(element)
        if (table) out.push(table)
      } else {
        // Unknown containers (div, section, article, details, …) flatten to their contents.
        this.blocks(element.childNodes, out, indent)
      }
    }
    flush()
  }

  private list(element: HTMLElement, indent: string) {
    const ordered = element.rawTagName.toLowerCase() === 'ol'
    let number = Number(element.getAttribute('start') ?? '1') || 1
    const lines: string[] = []
    for (const item of element.childNodes) {
      if (item.nodeType !== NodeType.ELEMENT_NODE) continue
      const li = item as HTMLElement
      if (li.rawTagName.toLowerCase() !== 'li') continue
      const marker = ordered ? `${number++}. ` : '- '
      const inner: string[] = []
      this.blocks(li.childNodes, inner, '')
      // Tight list: an item's paragraph and its nested list stay on adjacent lines.
      const body = inner.join('\n').split('\n')
      const pad = ' '.repeat(marker.length)
      lines.push(
        body
          .map((line, index) =>
            index === 0 ? indent + marker + line : line ? indent + pad + line : '',
          )
          .join('\n'),
      )
    }
    return lines.join('\n')
  }

  private codeFence(pre: HTMLElement, indent: string) {
    const code = pre.querySelector('code')
    const classes = `${pre.getAttribute('class') ?? ''} ${code?.getAttribute('class') ?? ''}`
    const language = /\bmermaid\b/.test(classes)
      ? 'mermaid'
      : (/(?:language|lang)-([\w+#-]+)/.exec(classes)?.[1] ?? '')
    const text = decode((code ?? pre).rawText).replace(/\n$/, '')
    return this.fence(language, text, indent)
  }

  private fence(language: string, text: string, indent: string) {
    const fence = text.includes('```') ? '~~~~' : '```'
    return [`${indent}${fence}${language}`, ...text.split('\n'), fence]
      .map((line, index) => (index === 0 ? line : indent + line))
      .join('\n')
  }

  /**
   * Inline SVG (charts, diagrams, logos) becomes an image the reader draws as-is. It is a data
   * URL, so nothing is fetched, and scripts inside it never run in GPUI's rasterizer.
   */
  private svg(element: HTMLElement) {
    let markup = element.toString()
    if (!/\sxmlns=/.test(markup.slice(0, markup.indexOf('>')))) {
      markup = markup.replace(/^<svg/i, '<svg xmlns="http://www.w3.org/2000/svg"')
    }
    const label = element.getAttribute('aria-label') ?? element.querySelector('title')?.text ?? ''
    const data = `data:image/svg+xml;base64,${Buffer.from(markup).toString('base64')}`
    return `![${escapeText(label.trim())}](<${data}>)`
  }

  private table(table: HTMLElement) {
    const rows = table
      .querySelectorAll('tr')
      .map((row) =>
        row.childNodes
          .filter(
            (cell): cell is HTMLElement =>
              cell.nodeType === NodeType.ELEMENT_NODE &&
              ['td', 'th'].includes((cell as HTMLElement).rawTagName.toLowerCase()),
          )
          .map((cell) => this.inline(cell.childNodes).trim().replace(/\|/g, '\\|') || ' '),
      )
      .filter((row) => row.length > 0)
    if (rows.length === 0) return null
    const width = Math.max(...rows.map((row) => row.length))
    const line = (row: string[]) =>
      `| ${Array.from({ length: width }, (_, i) => row[i] ?? ' ').join(' | ')} |`
    return [line(rows[0]!), line(Array(width).fill('---')), ...rows.slice(1).map(line)].join('\n')
  }

  inline(nodes: Node[]): string {
    return nodes
      .map((node) => {
        if (node.nodeType === NodeType.TEXT_NODE) {
          return escapeText(decode(node.rawText.replace(/[ \t\r\n]+/g, ' ')))
        }
        if (node.nodeType !== NodeType.ELEMENT_NODE) return ''
        const element = node as HTMLElement
        const tag = element.rawTagName?.toLowerCase() ?? ''
        if (STRIPPED.has(tag)) return ''
        const inner = () => this.inline(element.childNodes)
        switch (tag) {
          case 'br':
            return '  \n'
          case 'strong':
          case 'b':
            return wrap('**', inner())
          case 'em':
          case 'i':
            return wrap('*', inner())
          case 'del':
          case 's':
          case 'strike':
            return wrap('~~', inner())
          case 'code':
          case 'kbd':
          case 'samp': {
            const text = decode(element.rawText)
            const ticks = text.includes('`') ? '``' : '`'
            return text ? `${ticks}${text}${ticks}` : ''
          }
          case 'a': {
            const text = inner().trim()
            const href = element.getAttribute('href')
            if (!href) return text
            return `[${text || href}](<${this.target(href)}>)`
          }
          case 'svg':
            return this.svg(element)
          case 'img': {
            const src = element.getAttribute('src')
            if (!src) return ''
            return `![${escapeText(element.getAttribute('alt') ?? '')}](<${this.target(src)}>)`
          }
          default:
            return isInline(element) ? inner() : ` ${inner()} `
        }
      })
      .join('')
  }

  private target(target: string) {
    const trimmed = target.trim()
    if (/^[a-z][a-z0-9+.-]*:/i.test(trimmed) || trimmed.startsWith('#')) return trimmed
    const [path, suffix = ''] = splitSuffix(trimmed)
    let decoded = path
    try {
      decoded = decodeURI(path)
    } catch {}
    return (isAbsolute(decoded) ? decoded : resolve(this.baseDir, decoded)) + suffix
  }
}

function splitSuffix(target: string): [string, string?] {
  const index = target.search(/[?#]/)
  return index === -1 ? [target] : [target.slice(0, index), target.slice(index)]
}

function isMermaid(element: HTMLElement) {
  const tag = element.rawTagName?.toLowerCase()
  return (tag === 'div' || tag === 'pre') && /\bmermaid\b/.test(element.getAttribute('class') ?? '')
}

/** The document's `<title>`, when it has one. */
export function htmlTitle(html: string): string | null {
  const match = /<title[^>]*>([\s\S]*?)<\/title>/i.exec(html)
  const title = match ? decode(match[1]!).replace(/\s+/g, ' ').trim() : ''
  return title || null
}

function isInline(node: Node) {
  return (
    node.nodeType === NodeType.ELEMENT_NODE &&
    INLINE.has((node as HTMLElement).rawTagName?.toLowerCase() ?? '')
  )
}

function wrap(marker: string, text: string) {
  const trimmed = text.trim()
  return trimmed ? `${marker}${trimmed}${marker}` : ''
}

function escapeText(text: string) {
  return text.replace(/([\\`*_[\]<>])/g, '\\$1')
}

const ENTITIES: Record<string, string> = {
  amp: '&',
  lt: '<',
  gt: '>',
  quot: '"',
  apos: "'",
  nbsp: ' ',
  mdash: '—',
  ndash: '–',
  hellip: '…',
  copy: '©',
  reg: '®',
  trade: '™',
  rsquo: '’',
  lsquo: '‘',
  rdquo: '”',
  ldquo: '“',
}

function decode(text: string) {
  return text.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (match, entity: string) => {
    if (entity[0] === '#') {
      const code =
        entity[1] === 'x' || entity[1] === 'X'
          ? parseInt(entity.slice(2), 16)
          : parseInt(entity.slice(1), 10)
      return Number.isFinite(code) ? String.fromCodePoint(code) : match
    }
    return ENTITIES[entity.toLowerCase()] ?? match
  })
}
