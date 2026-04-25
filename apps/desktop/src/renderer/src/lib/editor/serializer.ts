import { defaultMarkdownSerializer } from 'prosemirror-markdown'
import type { Node as ProseMirrorNode } from '@tiptap/pm/model'

export function serializeMarkdown(doc: ProseMirrorNode): string {
  return defaultMarkdownSerializer.serialize(doc)
}
