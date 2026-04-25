import {
  defaultMarkdownParser,
  schema as defaultSchema,
  MarkdownParser,
} from 'prosemirror-markdown'
import { Schema, type Node as ProseMirrorNode } from '@tiptap/pm/model'
import { Transform } from '@tiptap/pm/transform'
import MarkdownIt from 'markdown-it'

export const schema = new Schema({
  nodes: defaultSchema.spec.nodes.addToEnd('mermaidBlock', {
    group: 'block',
    atom: true,
    attrs: { source: { default: '' } },
    parseDOM: [{ tag: 'div[data-type="mermaid"]' }],
    toDOM: () => ['div', { 'data-type': 'mermaid' }],
  }),
  marks: defaultSchema.spec.marks,
})

const tokenizer = MarkdownIt('commonmark', { html: true } as never)

const parser = new MarkdownParser(schema, tokenizer, defaultMarkdownParser.tokens)

export function parseMarkdown(text: string): ProseMirrorNode {
  const doc = parser.parse(text)
  if (!doc) {
    throw new Error('Failed to parse markdown')
  }
  return convertMermaidBlocks(doc)
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
