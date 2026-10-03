import { parseUnifiedDiff } from './unified-diff'

/** How far a hunk may have drifted from its recorded line number and still apply. */
const MAX_DRIFT = 200

function matchesAt(lines: string[], expected: string[], start: number): boolean {
  if (start < 0 || start + expected.length > lines.length) return false
  for (let index = 0; index < expected.length; index += 1) {
    if (lines[start + index] !== expected[index]) return false
  }
  return true
}

/**
 * Applies a unified diff to `text`, returning the patched text, or null when the patch no longer
 * matches (the document changed since the patch was made). Hunks may drift a little, as they do
 * when earlier hunks add or remove lines.
 */
export function applyUnifiedPatch(text: string, patch: string): string | null {
  const hunks = parseUnifiedDiff(patch)
  if (hunks.length === 0) return null

  const newline = text.includes('\r\n') ? '\r\n' : '\n'
  const lines = text.split(/\r?\n/)
  const output: string[] = []
  let cursor = 0

  for (const hunk of hunks) {
    const before = hunk.lines.filter((line) => line.kind !== 'add').map((line) => line.text)
    const after = hunk.lines.filter((line) => line.kind !== 'remove').map((line) => line.text)
    const firstOld = hunk.lines.find((line) => line.oldLine !== null)?.oldLine
    // A hunk that only adds lines records where they go through its new-side numbering.
    const expected = Math.max(
      cursor,
      (firstOld ?? hunk.lines.find((line) => line.newLine !== null)?.newLine ?? 1) - 1,
    )

    let start = -1
    for (let drift = 0; drift <= MAX_DRIFT && start === -1; drift += 1) {
      if (matchesAt(lines, before, expected + drift)) start = expected + drift
      else if (expected - drift >= cursor && matchesAt(lines, before, expected - drift)) {
        start = expected - drift
      }
    }
    if (start === -1) return null

    output.push(...lines.slice(cursor, start), ...after)
    cursor = start + before.length
  }

  output.push(...lines.slice(cursor))
  return output.join(newline)
}
