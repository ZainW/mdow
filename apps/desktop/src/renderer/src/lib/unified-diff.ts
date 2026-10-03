export interface DiffLine {
  kind: 'context' | 'add' | 'remove'
  text: string
  oldLine: number | null
  newLine: number | null
}

export interface DiffHunk {
  header: string
  lines: DiffLine[]
}

const HUNK_HEADER = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/

/** Parses the hunks of a unified diff, skipping file headers and "no newline" markers. */
export function parseUnifiedDiff(patch: string): DiffHunk[] {
  const hunks: DiffHunk[] = []
  let current: DiffHunk | null = null
  let oldLine = 0
  let newLine = 0

  for (const raw of patch.replace(/\r\n/g, '\n').split('\n')) {
    const header = HUNK_HEADER.exec(raw)
    if (header) {
      oldLine = Number(header[1])
      newLine = Number(header[2])
      current = { header: header[3].trim(), lines: [] }
      hunks.push(current)
      continue
    }
    if (!current || raw.startsWith('\\')) continue

    const marker = raw.charAt(0)
    const text = raw.slice(1)
    if (marker === '+') {
      current.lines.push({ kind: 'add', text, oldLine: null, newLine: newLine++ })
    } else if (marker === '-') {
      current.lines.push({ kind: 'remove', text, oldLine: oldLine++, newLine: null })
    } else if (marker === ' ') {
      current.lines.push({ kind: 'context', text, oldLine: oldLine++, newLine: newLine++ })
    }
  }

  return hunks.filter((hunk) => hunk.lines.length > 0)
}
