import {
  createMarkdownParser,
  type ComarkPlugin,
  type ElementNode,
  type MarkdownDocument,
  type Node,
} from 'comark'
import { memoizeAsync } from './cache-storage'
import { normalizeLanguage } from './highlight'
import { SECTION_TAG } from './markdown-sections'

type ParseFn = ReturnType<typeof createMarkdownParser>

interface ParserFeatures {
  math: boolean
  mermaid: boolean
}

const parserPromises = new Map<string, Promise<ParseFn>>()

// Convert soft line breaks (`\n` inside text nodes) to <br>, GitHub-flavored.
// Skips <pre>/<code> subtrees so code keeps its literal newlines.
function walkBreaks(node: unknown): void {
  if (!Array.isArray(node) || node.length <= 2) return
  const arr = node as unknown[]
  const tag = arr[0]
  if (tag === 'pre' || tag === 'code') return
  let modified = false
  const next: unknown[] = []
  for (let i = 2; i < arr.length; i++) {
    const child = arr[i]
    if (typeof child === 'string' && /\n/.test(child)) {
      modified = true
      const lines = child.split('\n')
      for (let li = 0; li < lines.length; li++) {
        if (lines[li].length > 0) next.push(lines[li])
        if (li < lines.length - 1) next.push(['br', {}])
      }
    } else {
      next.push(child)
    }
  }
  if (modified) {
    arr.length = 2
    arr.push(...next)
  }
  for (let i = 2; i < arr.length; i++) walkBreaks(arr[i])
}

const breaksOutsideCode: ComarkPlugin = {
  name: 'breaks-outside-code',
  post(state) {
    for (const n of state.tree.nodes) walkBreaks(n)
  },
}

const fencePattern = /^ {0,3}(`{3,}|~{3,})([^\n]*)$/gm

function detectParserFeatures(text: string): ParserFeatures {
  const infos: string[] = []
  for (const match of text.matchAll(fencePattern)) {
    infos.push(match[2]?.trim() ?? '')
  }

  return {
    math: /(^|[^\\])\$/.test(text),
    mermaid: infos.some((info) => normalizeLanguage(info) === 'mermaid'),
  }
}

function parserKey(features: ParserFeatures): string {
  return [features.math ? 'math' : 'no-math', features.mermaid ? 'mermaid' : 'no-mermaid'].join(':')
}

async function createParser(features: ParserFeatures): Promise<ParseFn> {
  const plugins: ComarkPlugin[] = []

  if (features.math) {
    const { default: math } = await import('comark/plugins/math')
    plugins.push(math({ throwOnError: false }))
  }
  if (features.mermaid) {
    const { default: mermaid } = await import('comark/plugins/mermaid')
    plugins.push(mermaid())
  }

  plugins.push(breaksOutsideCode)

  return createMarkdownParser({ plugins })
}

async function getParser(text: string): Promise<ParseFn> {
  const features = detectParserFeatures(text)
  const key = parserKey(features)

  let promise = parserPromises.get(key)
  if (!promise) {
    promise = createParser(features)
    parserPromises.set(key, promise)
  }
  return promise
}

export async function initMarkdown(): Promise<void> {
  const parse = await getParser('plain')
  await parse('# Warm\n\n- list\n\n```ts\n//\n```')
}

export interface DocHeading {
  level: number
  text: string
  id: string
}

export interface RenderResult {
  tree: MarkdownDocument
  mermaidBlocks: { id: string; code: string }[]
  headings: DocHeading[]
  frontmatter: Record<string, unknown>
}

function slugifyHeading(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
}

function isElement(node: Node): node is ElementNode {
  return Array.isArray(node) && typeof node[0] === 'string'
}

function isNode(value: unknown): value is Node {
  return typeof value === 'string' || Array.isArray(value)
}

function getNodeText(node: Node): string {
  if (typeof node === 'string') return node
  if (!isElement(node)) return ''
  return getChildren(node).map(getNodeText).join('')
}

function getChildren(node: ElementNode): Node[] {
  const children: Node[] = []
  for (let i = 2; i < node.length; i++) {
    const child = node[i]
    if (isNode(child)) children.push(child)
  }
  return children
}

function appendClassName(node: ElementNode, className: string): void {
  const attrs = node[1]
  const existing = attrs.class
  if (typeof existing === 'string' && existing.length > 0) {
    attrs.class = `${existing} ${className}`
  } else {
    attrs.class = className
  }
}

// Documents with many top-level blocks are grouped into heading-aligned sections. Each section
// is one content-visibility box, so Chromium tracks a few hundred boxes instead of tens of
// thousands, and the per-section height estimate keeps the scrollbar honest before layout.
const SECTION_MIN_BLOCKS = 160
const SECTION_TARGET_BLOCKS = 24
const SECTION_MAX_BLOCKS = 200
// Rough characters per rendered line in the default reading column.
const CHARS_PER_LINE = 85

function countDescendants(node: Node, tag: string): number {
  if (!isElement(node)) return 0
  let count = node[0] === tag ? 1 : 0
  for (const child of getChildren(node)) count += countDescendants(child, tag)
  return count
}

function textLines(node: Node): number {
  return Math.max(1, Math.ceil(getNodeText(node).length / CHARS_PER_LINE))
}

/** Estimated rendered height of a top-level block, in em. Only needs to be roughly right. */
export function estimateBlockHeight(node: Node): number {
  if (!isElement(node)) return 0
  const tag = node[0]
  switch (tag) {
    case 'h1':
      return 4.5
    case 'h2':
    case 'h3':
      return 3.5
    case 'h4':
    case 'h5':
    case 'h6':
      return 2.5
    case 'hr':
      return 4
    case 'pre': {
      const code = typeof node[1].code === 'string' ? node[1].code : getNodeText(node)
      return 4 + code.split('\n').length * 1.4
    }
    case 'table':
      return 3 + countDescendants(node, 'tr') * 2.3
    case 'ul':
    case 'ol':
      return 1 + countDescendants(node, 'li') * 1.75
    case 'mermaid':
      return 16
    default:
      return 1 + textLines(node) * 1.65
  }
}

function isSectionBreak(node: Node): boolean {
  return isElement(node) && (node[0] === 'h1' || node[0] === 'h2' || node[0] === 'h3')
}

// Two-lane FNV-1a over a block's tags, attributes, and text. Sections carry the combined value
// as a signature so a live reload only re-renders the sections whose content changed.
interface HashState {
  a: number
  b: number
}

function mixString(state: HashState, value: string): void {
  let { a, b } = state
  for (let i = 0; i < value.length; i++) {
    const code = value.charCodeAt(i)
    a = Math.imul(a ^ code, 0x01000193)
    b = Math.imul(b ^ code, 0x5bd1e995)
  }
  a = Math.imul(a ^ value.length, 0x01000193)
  state.a = a
  state.b = b
}

function mixAttributes(state: HashState, attrs: Record<string, unknown>): void {
  for (const key in attrs) {
    // `code` repeats the fence body, which is already hashed as the <code> child's text.
    if (key === 'code') continue
    const value = attrs[key]
    mixString(state, key)
    mixString(state, typeof value === 'string' ? value : (JSON.stringify(value) ?? ''))
  }
}

export function groupIntoSections(nodes: Node[], signatures?: string[]): Node[] {
  if (nodes.length < SECTION_MIN_BLOCKS) return nodes

  const sections: Node[] = []
  let current: Node[] = []
  let currentSignatures: string[] = []
  let estimate = 0
  const flush = () => {
    if (current.length === 0) return
    const attrs: Record<string, unknown> = { estimate: Math.round(estimate) }
    if (signatures) attrs.signature = currentSignatures.join(',')
    sections.push([SECTION_TAG, attrs, ...current] as ElementNode)
    current = []
    currentSignatures = []
    estimate = 0
  }

  for (let index = 0; index < nodes.length; index++) {
    const node = nodes[index]
    // Break before a heading so each section opens with the block whose top margin sets the
    // gap; margins cannot collapse across a content-visibility boundary.
    if (
      (current.length >= SECTION_TARGET_BLOCKS && isSectionBreak(node)) ||
      current.length >= SECTION_MAX_BLOCKS
    ) {
      flush()
    }
    current.push(node)
    if (signatures) currentSignatures.push(signatures[index])
    estimate += estimateBlockHeight(node)
  }
  flush()
  return sections
}

/** Parses and post-processes a document on the current thread (main thread or worker). */
export async function renderMarkdownInThread(text: string): Promise<RenderResult> {
  const parse = await getParser(text)
  const tree = await parse(text)

  const mermaidBlocks: { id: string; code: string }[] = []
  let mermaidCounter = 0
  const headings: DocHeading[] = []
  const slugCounts = new Map<string, number>()

  const hash: HashState = { a: 0x811c9dc5, b: 0x9747b28c }

  function visit(node: Node): void {
    if (typeof node === 'string') {
      mixString(hash, node)
      return
    }
    if (!isElement(node)) return

    const tag = node[0]
    const attrs = node[1]
    mixString(hash, tag)

    if (/^h[1-6]$/.test(tag)) {
      const headingText = getNodeText(node).trim()
      if (headingText) {
        let id = typeof attrs.id === 'string' ? attrs.id : ''
        if (!id) {
          const base = slugifyHeading(headingText)
          if (base) {
            const count = slugCounts.get(base) ?? 0
            slugCounts.set(base, count + 1)
            id = count === 0 ? base : `${base}-${count}`
            attrs.id = id
          }
        }
        if (id) headings.push({ level: Number(tag.slice(1)), text: headingText, id })
      }
    }

    if (tag === 'pre') {
      // Hand the raw fence body to CodeBlock, which highlights it lazily near the viewport.
      const code = node[2]
      if (isElement(code) && code[0] === 'code' && typeof code[2] === 'string') {
        attrs.code = code[2]
      }
    }

    if (tag === 'mermaid') {
      const code = typeof attrs.content === 'string' ? attrs.content : ''
      const id =
        typeof attrs.id === 'string' && attrs.id.length > 0
          ? attrs.id
          : `mermaid-${mermaidCounter++}`
      attrs.id = id
      appendClassName(node, 'mermaid mermaid-container')
      mermaidBlocks.push({ id, code })
    }

    // Attributes are hashed after the passes above have assigned ids and classes.
    mixAttributes(hash, attrs)
    for (const child of getChildren(node)) visit(child)
  }

  const signatures: string[] = []
  for (const node of tree.nodes) {
    hash.a = 0x811c9dc5
    hash.b = 0x9747b28c
    visit(node)
    signatures.push(`${(hash.a >>> 0).toString(36)}.${(hash.b >>> 0).toString(36)}`)
  }
  tree.nodes = groupIntoSections(tree.nodes, signatures)

  return {
    tree,
    mermaidBlocks,
    headings,
    frontmatter: tree.frontmatter ?? {},
  }
}

// Keyed by document text. Kept small: a render of a very large document holds its whole tree.
export const renderMarkdown = memoizeAsync<
  [text: string, options?: { bypassCache?: boolean }],
  RenderResult
>((text) => renderMarkdownInThread(text), {
  maxEntries: 24,
  getKey: (text) => text,
  shouldBypassCache: (_text, options?: { bypassCache?: boolean }) => options?.bypassCache === true,
})
