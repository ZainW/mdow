export interface TextSpan {
  start: number
  end: number
}

export interface WordDiff {
  /** Spans of `before` that were removed. */
  removed: TextSpan[]
  /** Spans of `after` that were inserted. */
  inserted: TextSpan[]
}

/** Above this many tokens on both sides the LCS table gets expensive; mark the whole text. */
const MAX_CELLS = 4_000_000

function tokenize(text: string): { token: string; start: number }[] {
  return Array.from(text.matchAll(/\s+|[\p{L}\p{N}_'’-]+|[^\s\p{L}\p{N}_]/gu), (match) => ({
    token: match[0],
    start: match.index,
  }))
}

function pushSpan(spans: TextSpan[], start: number, end: number): void {
  const last = spans.at(-1)
  if (last && last.end === start) last.end = end
  else spans.push({ start, end })
}

/**
 * Hugs each span to its words, then joins spans separated only by whitespace, so a rewritten
 * phrase reads as one highlight instead of a highlight per word.
 */
function trimSpans(text: string, spans: TextSpan[]): TextSpan[] {
  const trimmed = spans.flatMap(({ start, end }) => {
    let s = start
    let e = end
    while (s < e && /\s/.test(text[s])) s += 1
    while (e > s && /\s/.test(text[e - 1])) e -= 1
    return s < e ? [{ start: s, end: e }] : []
  })
  const merged: TextSpan[] = []
  for (const span of trimmed) {
    const last = merged.at(-1)
    if (last && /^\s*$/.test(text.slice(last.end, span.start))) last.end = span.end
    else merged.push({ ...span })
  }
  return merged
}

function spanOf(token: { token: string; start: number }): TextSpan {
  return { start: token.start, end: token.start + token.token.length }
}

/** Word-level diff between two texts, as character spans on each side. */
export function diffWords(before: string, after: string): WordDiff {
  const a = tokenize(before)
  const b = tokenize(after)

  let prefix = 0
  while (prefix < a.length && prefix < b.length && a[prefix].token === b[prefix].token) prefix += 1
  let suffix = 0
  while (
    suffix < a.length - prefix &&
    suffix < b.length - prefix &&
    a[a.length - 1 - suffix].token === b[b.length - 1 - suffix].token
  ) {
    suffix += 1
  }

  const midA = a.slice(prefix, a.length - suffix)
  const midB = b.slice(prefix, b.length - suffix)
  const removed: TextSpan[] = []
  const inserted: TextSpan[] = []

  if (midA.length * midB.length > MAX_CELLS) {
    if (midA.length) pushSpan(removed, spanOf(midA[0]).start, spanOf(midA.at(-1)!).end)
    if (midB.length) pushSpan(inserted, spanOf(midB[0]).start, spanOf(midB.at(-1)!).end)
    return { removed: trimSpans(before, removed), inserted: trimSpans(after, inserted) }
  }

  // Classic LCS table over the differing middle.
  const width = midB.length + 1
  const table = new Uint32Array((midA.length + 1) * width)
  for (let i = midA.length - 1; i >= 0; i -= 1) {
    for (let j = midB.length - 1; j >= 0; j -= 1) {
      table[i * width + j] =
        midA[i].token === midB[j].token
          ? table[(i + 1) * width + j + 1] + 1
          : Math.max(table[(i + 1) * width + j], table[i * width + j + 1])
    }
  }

  let i = 0
  let j = 0
  while (i < midA.length || j < midB.length) {
    if (i < midA.length && j < midB.length && midA[i].token === midB[j].token) {
      i += 1
      j += 1
    } else if (
      j < midB.length &&
      (i === midA.length || table[i * width + j + 1] >= table[(i + 1) * width + j])
    ) {
      const span = spanOf(midB[j])
      pushSpan(inserted, span.start, span.end)
      j += 1
    } else {
      const span = spanOf(midA[i])
      pushSpan(removed, span.start, span.end)
      i += 1
    }
  }

  return { removed: trimSpans(before, removed), inserted: trimSpans(after, inserted) }
}
