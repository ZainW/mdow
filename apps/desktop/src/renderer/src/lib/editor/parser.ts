import { defaultMarkdownParser } from 'prosemirror-markdown'
import type { Node as ProseMirrorNode } from '@tiptap/pm/model'

export function parseMarkdown(text: string): ProseMirrorNode {
  const doc = defaultMarkdownParser.parse(text)
  if (!doc) {
    throw new Error('Failed to parse markdown')
  }
  return doc
}
