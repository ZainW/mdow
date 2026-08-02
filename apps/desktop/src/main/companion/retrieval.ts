import { basename } from 'path'
import { readFileContent } from '../file-service'
import { scanFolder } from '../folder-service'
import { validatePath } from '../path-validation'
import { isMarkdownPath } from '../../shared/types'
import type { CompanionContextTag, TreeNode } from '../../shared/types'

const MAX_FILES_SCANNED = 200
const MAX_RANGES = 3
const MAX_RANGE_BYTES = 4_096

const STOP_WORDS = new Set([
  'about',
  'does',
  'from',
  'have',
  'into',
  'that',
  'this',
  'what',
  'when',
  'where',
  'which',
  'with',
])

export interface RetrievedRange {
  path: string
  excerpt: string
  startLine: number
  endLine: number
  bytes: number
  score: number
}

export interface RetrieveMarkdownInput {
  question: string
  roots: string[]
  excludedPaths: string[]
  readFile?: (path: string) => Promise<string>
  scan?: typeof scanFolder
}

function termsFor(value: string): string[] {
  const raw =
    value
      .toLowerCase()
      .match(/[\p{L}\p{N}_-]{3,}/gu)
      ?.filter((term) => !STOP_WORDS.has(term)) ?? []
  const expanded = raw.flatMap((term) => {
    const stem = term.replace(/(ations?|ments?|ingly|edly|ing|ed|es|s)$/u, '')
    return stem.length >= 3 && stem !== term ? [term, stem] : [term]
  })
  return [...new Set(expanded)]
}

function collectMarkdownPaths(nodes: TreeNode[], output: string[]): void {
  for (const node of nodes) {
    if (output.length >= MAX_FILES_SCANNED) return
    if (node.isDirectory) {
      if (node.children) collectMarkdownPaths(node.children, output)
    } else if (isMarkdownPath(node.path)) {
      output.push(node.path)
    }
  }
}

function lineScore(line: string, terms: string[]): number {
  const normalized = line.toLowerCase()
  return terms.reduce((score, term) => score + (normalized.includes(term) ? 1 : 0), 0)
}

function rangeForContent(path: string, content: string, terms: string[]): RetrievedRange | null {
  const lines = content.split(/\r?\n/)
  let bestLine = 0
  let bestLineScore = 0
  for (let index = 0; index < lines.length; index += 1) {
    const score = lineScore(lines[index], terms)
    if (score > bestLineScore) {
      bestLine = index
      bestLineScore = score
    }
  }

  const normalizedPath = basename(path).toLowerCase()
  const pathScore = terms.reduce(
    (score, term) => score + (normalizedPath.includes(term) ? 6 : 0),
    0,
  )
  const contentScore = terms.reduce((score, term) => {
    const matches = content.toLowerCase().split(term).length - 1
    return score + Math.min(matches, 6)
  }, 0)
  const score = pathScore + contentScore + bestLineScore * 3
  if (score === 0) return null

  let start = bestLine
  for (let index = bestLine; index >= 0; index -= 1) {
    if (/^#{1,6}\s+/.test(lines[index])) {
      start = index
      break
    }
    if (bestLine - index >= 8) break
    start = index
  }

  const selected: string[] = []
  let bytes = 0
  let end = start
  for (let index = start; index < lines.length; index += 1) {
    const line = lines[index]
    const separatorBytes = selected.length > 0 ? 1 : 0
    const lineBytes = Buffer.byteLength(line, 'utf8')
    if (bytes + separatorBytes + lineBytes > MAX_RANGE_BYTES) break
    selected.push(line)
    bytes += separatorBytes + lineBytes
    end = index
    if (index > bestLine + 24 && /^#{1,6}\s+/.test(lines[index + 1] ?? '')) break
  }

  return {
    path,
    excerpt: selected.join('\n'),
    startLine: start + 1,
    endLine: end + 1,
    bytes,
    score,
  }
}

export function shouldRetrieve(
  question: string,
  activePath: string | null,
  tags: CompanionContextTag[],
): boolean {
  const normalized = question.toLowerCase()
  const activeName = activePath ? basename(activePath).toLowerCase() : ''
  const namedMarkdown = normalized.match(/[\w./-]+\.(?:md|markdown|mdx)\b/g) ?? []
  if (namedMarkdown.some((name) => basename(name) !== activeName)) return true
  if (/\b(compare|contrast|difference|across|other|related|references?)\b/.test(normalized)) {
    return true
  }
  const hasFolderScope = tags.some((tag) => tag.kind === 'folder')
  return hasFolderScope && /\b(all|docs?|documents?|files?|folder|collection)\b/.test(normalized)
}

export async function retrieveMarkdownRanges(
  input: RetrieveMarkdownInput,
): Promise<RetrievedRange[]> {
  const scan = input.scan ?? scanFolder
  const readFile = input.readFile ?? readFileContent
  const excluded = new Set(input.excludedPaths.map((path) => validatePath(path)))
  const paths: string[] = []

  for (const root of [...new Set(input.roots)]) {
    try {
      const result = await scan(validatePath(root))
      collectMarkdownPaths(result.tree, paths)
      if (paths.length >= MAX_FILES_SCANNED) break
    } catch {
      // Invalid, missing, or unreadable roots contribute no retrieval candidates.
    }
  }

  const candidates = [...new Set(paths)].filter((path) => !excluded.has(validatePath(path)))
  const terms = termsFor(input.question)
  const ranges = await Promise.all(
    candidates.map(async (path) => {
      try {
        return rangeForContent(path, await readFile(path), terms)
      } catch {
        return null
      }
    }),
  )

  return ranges
    .filter((range): range is RetrievedRange => range !== null)
    .toSorted((a, b) => b.score - a.score || a.path.localeCompare(b.path))
    .slice(0, MAX_RANGES)
}
