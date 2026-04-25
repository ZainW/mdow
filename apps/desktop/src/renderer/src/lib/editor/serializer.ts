import { defaultMarkdownSerializer, MarkdownSerializer } from 'prosemirror-markdown'
import type { Node as ProseMirrorNode } from '@tiptap/pm/model'

const serializer = new MarkdownSerializer(
  {
    ...defaultMarkdownSerializer.nodes,
    mermaidBlock(state, node) {
      state.write('```mermaid\n')
      state.text(node.attrs.source as string, false)
      state.ensureNewLine()
      state.write('```')
      state.closeBlock(node)
    },
  },
  defaultMarkdownSerializer.marks,
)

export function serializeMarkdown(doc: ProseMirrorNode): string {
  return serializer.serialize(doc)
}
