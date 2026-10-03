import type { ElementNode, MarkdownDocument, Node } from 'comark'
import { SECTION_TAG } from './markdown-sections'

export type ReviewChangeKind = 'added' | 'removed' | 'modified'

export interface ReviewTree {
  tree: MarkdownDocument
  changeCount: number
}

/** Above this many cells the block LCS gets expensive; the differing middle becomes one change. */
const MAX_CELLS = 4_000_000

function isElement(node: Node): node is ElementNode {
  return Array.isArray(node) && typeof node[0] === 'string'
}

/** Top-level blocks, with the long-document section wrappers removed. */
function blocks(nodes: Node[]): Node[] {
  return nodes.flatMap((node) =>
    isElement(node) && node[0] === SECTION_TAG ? (node.slice(2) as Node[]) : [node],
  )
}

/** A block's identity for diffing: its content, ignoring source positions and generated ids. */
function blockKey(node: Node): string {
  return JSON.stringify(node, (key, value: unknown) =>
    key === '$' || key === 'id' ? undefined : value,
  )
}

/** Copies a removed block so its anchors and diagram ids never collide with the new version. */
function retire(node: Node): Node {
  if (!isElement(node)) return node
  const [tag, attrs, ...children] = node
  const nextAttrs = { ...attrs }
  if (typeof nextAttrs.id === 'string') {
    if (tag === 'mermaid') nextAttrs.id = `${nextAttrs.id}-previous`
    else delete nextAttrs.id
  }
  return [tag, nextAttrs, ...children.map(retire)]
}

type Op = { kind: 'same' | 'remove' | 'add'; node: Node }

function diffBlocks(before: Node[], after: Node[]): Op[] {
  const a = before.map(blockKey)
  const b = after.map(blockKey)
  let prefix = 0
  while (prefix < a.length && prefix < b.length && a[prefix] === b[prefix]) prefix += 1
  let suffix = 0
  while (
    suffix < a.length - prefix &&
    suffix < b.length - prefix &&
    a[a.length - 1 - suffix] === b[b.length - 1 - suffix]
  ) {
    suffix += 1
  }

  const ops: Op[] = before.slice(0, prefix).map((node, index) => ({
    kind: 'same',
    node: after[index] ?? node,
  }))
  const midA = a.slice(prefix, a.length - suffix)
  const midB = b.slice(prefix, b.length - suffix)

  if (midA.length * midB.length > MAX_CELLS) {
    ops.push(
      ...before.slice(prefix, a.length - suffix).map((node) => ({ kind: 'remove' as const, node })),
    )
    ops.push(
      ...after.slice(prefix, b.length - suffix).map((node) => ({ kind: 'add' as const, node })),
    )
  } else {
    const width = midB.length + 1
    const table = new Uint32Array((midA.length + 1) * width)
    for (let i = midA.length - 1; i >= 0; i -= 1) {
      for (let j = midB.length - 1; j >= 0; j -= 1) {
        table[i * width + j] =
          midA[i] === midB[j]
            ? table[(i + 1) * width + j + 1] + 1
            : Math.max(table[(i + 1) * width + j], table[i * width + j + 1])
      }
    }
    let i = 0
    let j = 0
    while (i < midA.length || j < midB.length) {
      if (i < midA.length && j < midB.length && midA[i] === midB[j]) {
        ops.push({ kind: 'same', node: after[prefix + j] })
        i += 1
        j += 1
      } else if (
        i < midA.length &&
        (j === midB.length || table[(i + 1) * width + j] >= table[i * width + j + 1])
      ) {
        ops.push({ kind: 'remove', node: before[prefix + i] })
        i += 1
      } else {
        ops.push({ kind: 'add', node: after[prefix + j] })
        j += 1
      }
    }
  }

  ops.push(...after.slice(b.length - suffix).map((node) => ({ kind: 'same' as const, node })))
  return ops
}

function withClass(node: ElementNode, className: string, extra: Record<string, unknown> = {}) {
  const [tag, attrs, ...children] = node
  const existing = typeof attrs.class === 'string' && attrs.class ? `${attrs.class} ` : ''
  return [tag, { ...attrs, ...extra, class: `${existing}${className}` }, ...children] as ElementNode
}

/**
 * A list edited in place stays one list: only the items that changed are marked, instead of
 * the whole list appearing twice. Returns null when the change is not a single list edit.
 */
function refineList(removed: Node[], added: Node[], index: number): ElementNode | null {
  const [before] = removed
  const [after] = added
  if (removed.length !== 1 || added.length !== 1 || !isElement(before) || !isElement(after)) {
    return null
  }
  if (before[0] !== after[0] || (before[0] !== 'ul' && before[0] !== 'ol')) return null

  const items: Node[] = []
  let gone: Node[] = []
  let fresh: Node[] = []
  const flushItems = () => {
    const struck = fresh.length === 0
    for (const item of gone) {
      if (!isElement(item)) continue
      items.push(
        withClass(
          retire(item) as ElementNode,
          struck ? 'md-change-old md-change-struck' : 'md-change-old',
          {
            'aria-label': 'Current text',
          },
        ),
      )
    }
    for (const item of fresh) {
      if (isElement(item))
        items.push(withClass(item, 'md-change-new', { 'aria-label': 'Suggested text' }))
    }
    gone = []
    fresh = []
  }
  for (const op of diffBlocks(before.slice(2) as Node[], after.slice(2) as Node[])) {
    if (op.kind === 'same') {
      flushItems()
      items.push(op.node)
    } else if (op.kind === 'remove') {
      gone.push(op.node)
    } else {
      fresh.push(op.node)
    }
  }
  flushItems()

  const [tag, attrs] = after
  return [
    tag,
    { ...attrs, class: 'md-change md-change-list', 'data-change-index': String(index) },
    ...items,
  ] as ElementNode
}

/**
 * Builds a document that shows the proposed version with each change wrapped for review:
 * `.md-change` holds the replaced blocks (`.md-change-old`) above their replacement
 * (`.md-change-new`), so the reader renders both with its usual components.
 */
export function buildReviewTree(before: MarkdownDocument, after: MarkdownDocument): ReviewTree {
  const ops = diffBlocks(blocks(before.nodes), blocks(after.nodes))
  const nodes: Node[] = []
  let changeCount = 0
  let removed: Node[] = []
  let added: Node[] = []

  const flush = () => {
    if (removed.length === 0 && added.length === 0) return
    const list = refineList(removed, added, changeCount)
    if (list) {
      nodes.push(list)
    } else {
      const kind: ReviewChangeKind =
        removed.length && added.length ? 'modified' : removed.length ? 'removed' : 'added'
      const parts: Node[] = []
      if (removed.length) {
        parts.push([
          'div',
          { class: 'md-change-old', role: 'group', 'aria-label': 'Current text' },
          ...removed.map(retire),
        ] as ElementNode)
      }
      if (added.length) {
        parts.push([
          'div',
          { class: 'md-change-new', role: 'group', 'aria-label': 'Suggested text' },
          ...added,
        ] as ElementNode)
      }
      nodes.push([
        'div',
        { class: `md-change md-change-${kind}`, 'data-change-index': String(changeCount) },
        ...parts,
      ] as ElementNode)
    }
    changeCount += 1
    removed = []
    added = []
  }

  for (const op of ops) {
    if (op.kind === 'same') {
      flush()
      nodes.push(op.node)
    } else if (op.kind === 'remove') {
      removed.push(op.node)
    } else {
      added.push(op.node)
    }
  }
  flush()

  return { tree: { ...after, nodes }, changeCount }
}
