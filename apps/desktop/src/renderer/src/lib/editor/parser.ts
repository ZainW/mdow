import {
  defaultMarkdownParser,
  schema as defaultSchema,
  MarkdownParser,
} from 'prosemirror-markdown'
import { Fragment, Schema, type Node as ProseMirrorNode } from '@tiptap/pm/model'
import { Transform } from '@tiptap/pm/transform'
import MarkdownIt from 'markdown-it'
import { serializeMarkdown } from './serializer'

export const schema: Schema = new Schema({
  nodes: defaultSchema.spec.nodes
    .addToStart('frontmatter', {
      group: 'block',
      atom: true,
      attrs: { source: { default: '' } },
      parseDOM: [{ tag: 'div[data-type="frontmatter"]' }],
      toDOM: () => ['div', { 'data-type': 'frontmatter' }],
    })
    .addToEnd('mermaidBlock', {
      group: 'block',
      atom: true,
      attrs: { source: { default: '' } },
      parseDOM: [{ tag: 'div[data-type="mermaid"]' }],
      toDOM: () => ['div', { 'data-type': 'mermaid' }],
    })
    .addToEnd('htmlBlock', {
      group: 'block',
      atom: true,
      attrs: { source: { default: '' } },
      parseDOM: [{ tag: 'div[data-type="html-block"]' }],
      toDOM: () => ['div', { 'data-type': 'html-block' }],
    }),
  marks: defaultSchema.spec.marks,
})

const tokenizer = MarkdownIt('commonmark', { html: true } as never)

const parser = new MarkdownParser(schema, tokenizer, {
  ...defaultMarkdownParser.tokens,
  html_block: {
    node: 'htmlBlock',
    getAttrs: (tok) => ({ source: (tok as { content: string }).content.replace(/\n+$/, '') }),
  },
})

const FRONTMATTER_RE = /^---\n([\s\S]*?)\n---\n?(\n)?/

export interface ParseResult {
  doc: ProseMirrorNode
  lossy: boolean
}

// Lossy means we can't safely round-trip the document — i.e., editing it
// risks losing content. We treat the serializer as canonical: harmless
// normalizations (bullet style, italic syntax, blank-line spacing) are NOT
// considered lossy. The CI fixture suite asserts strict round-trip equality;
// this runtime check only flags hard failures (serializer throws).
export function parseMarkdownChecked(text: string): ParseResult {
  const doc = parseMarkdown(text)
  let lossy = false
  try {
    serializeMarkdown(doc)
  } catch {
    lossy = true
  }
  return { doc, lossy }
}

export function parseMarkdown(text: string): ProseMirrorNode {
  let input = text
  let frontmatterSource: string | null = null
  const fmMatch = FRONTMATTER_RE.exec(input)
  if (fmMatch) {
    frontmatterSource = fmMatch[1]
    input = input.slice(fmMatch[0].length)
  }

  let result = parser.parse(input)
  if (!result) {
    throw new Error('Failed to parse markdown')
  }
  result = convertMermaidBlocks(result)

  if (frontmatterSource !== null) {
    const fmNode = schema.nodes.frontmatter.create({ source: frontmatterSource })
    const newContent = Fragment.from(fmNode).append(result.content)
    result = result.type.create(result.attrs, newContent, result.marks)
  }

  return result
}

function convertMermaidBlocks(doc: ProseMirrorNode): ProseMirrorNode {
  const replacements: { from: number; to: number; source: string }[] = []
  doc.descendants((node, pos) => {
    if (node.type.name === 'code_block' && node.attrs.params === 'mermaid') {
      replacements.push({ from: pos, to: pos + node.nodeSize, source: node.textContent })
    }
  })
  if (replacements.length === 0) return doc
  const tr = new Transform(doc)
  for (let i = replacements.length - 1; i >= 0; i--) {
    const r = replacements[i]
    const mermaidNode = schema.nodes.mermaidBlock.create({ source: r.source })
    tr.replaceWith(r.from, r.to, mermaidNode)
  }
  return tr.doc
}
