import { defaultMarkdownSerializer, MarkdownSerializer } from 'prosemirror-markdown'
import type { Node as ProseMirrorNode } from '@tiptap/pm/model'

// Override prosemirror-markdown's default node serializers and marks to
// match oxfmt's markdown normalization (bullets as `-`, italic as `_..._`).
// Otherwise the formatter would rewrite our output and we'd never round-trip.
const serializer = new MarkdownSerializer(
  {
    ...defaultMarkdownSerializer.nodes,
    bullet_list(state, node) {
      state.renderList(node, '  ', () => '- ')
    },
    frontmatter(state, node) {
      state.write('---\n')
      state.text(node.attrs.source as string, false)
      state.ensureNewLine()
      state.write('---')
      state.closeBlock(node)
    },
    mermaidBlock(state, node) {
      state.write('```mermaid\n')
      state.text(node.attrs.source as string, false)
      state.ensureNewLine()
      state.write('```')
      state.closeBlock(node)
    },
    htmlBlock(state, node) {
      state.text(node.attrs.source as string, false)
      state.closeBlock(node)
    },
  },
  {
    ...defaultMarkdownSerializer.marks,
    em: { open: '_', close: '_', mixable: true, expelEnclosingWhitespace: true },
  },
)

export function serializeMarkdown(doc: ProseMirrorNode): string {
  return serializer.serialize(doc)
}
