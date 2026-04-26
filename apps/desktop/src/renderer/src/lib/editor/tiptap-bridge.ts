// Bridge between prosemirror-markdown's schema (snake_case node/mark names)
// and Tiptap's schema (camelCase). The parser builds docs in the former; the
// Tiptap editor expects the latter. Without this translation, Tiptap silently
// drops unrecognized node types and the document renders as blank/partial.

import type { Node as ProseMirrorNode, Schema } from '@tiptap/pm/model'

const nodeNameToTiptap: Record<string, string> = {
  code_block: 'codeBlock',
  bullet_list: 'bulletList',
  ordered_list: 'orderedList',
  list_item: 'listItem',
  hard_break: 'hardBreak',
  horizontal_rule: 'horizontalRule',
}

const markNameToTiptap: Record<string, string> = {
  strong: 'bold',
  em: 'italic',
  s: 'strike',
}

const nodeNameFromTiptap: Record<string, string> = Object.fromEntries(
  Object.entries(nodeNameToTiptap).map(([k, v]) => [v, k]),
)

const markNameFromTiptap: Record<string, string> = Object.fromEntries(
  Object.entries(markNameToTiptap).map(([k, v]) => [v, k]),
)

interface PMJSONLike {
  type?: string
  attrs?: Record<string, unknown>
  marks?: { type: string; attrs?: Record<string, unknown> }[]
  content?: PMJSONLike[]
  text?: string
}

function rename(
  json: unknown,
  nodeMap: Record<string, string>,
  markMap: Record<string, string>,
): unknown {
  if (typeof json !== 'object' || json === null) return json
  if (Array.isArray(json)) return json.map((j) => rename(j, nodeMap, markMap))
  const node = json as PMJSONLike
  const out: PMJSONLike = { ...node }
  if (out.type && nodeMap[out.type]) out.type = nodeMap[out.type]
  if (out.marks) {
    out.marks = out.marks.map((m) => ({
      ...m,
      type: markMap[m.type] ?? m.type,
    }))
  }
  if (out.content) {
    out.content = out.content.map((c) => rename(c, nodeMap, markMap) as PMJSONLike)
  }
  return out
}

// Translate a ProseMirror doc (parser's schema) into JSON that Tiptap's
// editor can hydrate. Use as: editor content = toTiptapJSON(parseMarkdown(text))
export function toTiptapJSON(doc: ProseMirrorNode): unknown {
  return rename(doc.toJSON(), nodeNameToTiptap, markNameToTiptap)
}

// Inverse: take JSON from a Tiptap editor and produce a doc in the parser's
// schema for serialization. Use as:
//   const node = fromTiptapJSON(editor.state.doc.toJSON(), schema)
//   serializeMarkdown(node)
export function fromTiptapJSON(json: unknown, parserSchema: Schema): ProseMirrorNode {
  const renamed = rename(json, nodeNameFromTiptap, markNameFromTiptap)
  return parserSchema.nodeFromJSON(renamed)
}
