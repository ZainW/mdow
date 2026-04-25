import { defaultMarkdownSerializer } from 'prosemirror-markdown'
import type { Node as ProseMirrorNode } from 'prosemirror-model'

export function serializeMarkdown(doc: ProseMirrorNode): string {
  return defaultMarkdownSerializer.serialize(doc)
}
