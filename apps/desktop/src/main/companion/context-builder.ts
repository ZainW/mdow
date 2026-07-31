/* oxlint-disable eslint/no-await-in-loop, react-doctor/async-await-in-loop -- Source order is semantic and each read updates the shared byte budget. */
import { basename } from 'path'
import { readFileContent } from '../file-service'
import { scanFolder } from '../folder-service'
import { validateMarkdownPath, validatePath } from '../path-validation'
import { isMarkdownPath } from '../../shared/types'
import type {
  CompanionContextPacket,
  CompanionContextSource,
  CompanionContextTag,
} from '../../shared/types'

const MAX_SOURCE_BYTES = 24_000
const MAX_TOTAL_BYTES = 120_000
const MAX_FOLDER_FILES = 20

export interface BuildContextInput {
  activePath: string | null
  openFolderPath: string | null
  tags: CompanionContextTag[]
  readFile?: (path: string) => Promise<string>
  scan?: typeof scanFolder
}

function sourceIdFor(path: string, headingId?: string): string {
  return headingId ? `src:${path}#${headingId}` : `src:${path}`
}

function truncate(text: string, maxBytes: number): { text: string; truncated: boolean } {
  if (Buffer.byteLength(text, 'utf8') <= maxBytes) return { text, truncated: false }
  let end = Math.min(text.length, maxBytes)
  while (end > 0 && Buffer.byteLength(text.slice(0, end), 'utf8') > maxBytes) end -= 1
  return { text: `${text.slice(0, end)}\n\n[truncated]`, truncated: true }
}

async function addSource(
  sources: CompanionContextSource[],
  warnings: string[],
  seen: Set<string>,
  path: string,
  budget: { used: number },
  readFile: (path: string) => Promise<string>,
): Promise<void> {
  let resolved: string
  try {
    resolved = validateMarkdownPath(path)
  } catch {
    warnings.push(`Skipped non-markdown or invalid path: ${path}`)
    return
  }
  if (seen.has(resolved)) return
  if (budget.used >= MAX_TOTAL_BYTES) {
    warnings.push('Context budget reached; additional files omitted')
    return
  }

  try {
    const raw = await readFile(resolved)
    const remaining = Math.min(MAX_SOURCE_BYTES, MAX_TOTAL_BYTES - budget.used)
    const { text, truncated } = truncate(raw, remaining)
    const bytes = Buffer.byteLength(text, 'utf8')
    budget.used += bytes
    seen.add(resolved)
    sources.push({
      sourceId: sourceIdFor(resolved),
      path: resolved,
      excerpt: text,
      bytes,
    })
    if (truncated) warnings.push(`Truncated ${basename(resolved)}`)
  } catch {
    warnings.push(`Could not read ${path}`)
  }
}

function collectMarkdownPaths(
  nodes: { path: string; isDirectory: boolean; children?: unknown[] }[],
  out: string[],
): void {
  for (const node of nodes) {
    if (node.isDirectory && Array.isArray(node.children)) {
      collectMarkdownPaths(node.children as typeof nodes, out)
    } else if (!node.isDirectory && isMarkdownPath(node.path)) {
      out.push(node.path)
    }
  }
}

export async function buildCompanionContext(
  input: BuildContextInput,
): Promise<CompanionContextPacket> {
  const readFile = input.readFile ?? readFileContent
  const scan = input.scan ?? scanFolder
  const sources: CompanionContextSource[] = []
  const warnings: string[] = []
  const seen = new Set<string>()
  const budget = { used: 0 }

  if (input.activePath) {
    await addSource(sources, warnings, seen, input.activePath, budget, readFile)
  }

  for (const tag of input.tags) {
    if (tag.kind === 'file') {
      await addSource(sources, warnings, seen, tag.path, budget, readFile)
    } else if (tag.kind === 'folder') {
      try {
        const resolved = validatePath(tag.path)
        const { tree, truncated } = await scan(resolved)
        if (truncated) warnings.push(`Folder scan truncated for ${tag.path}`)
        const paths: string[] = []
        collectMarkdownPaths(tree, paths)
        for (const path of paths.slice(0, MAX_FOLDER_FILES)) {
          await addSource(sources, warnings, seen, path, budget, readFile)
        }
      } catch {
        warnings.push(`Could not scan folder tag ${tag.path}`)
      }
    }
  }

  if (input.openFolderPath) {
    try {
      const resolved = validatePath(input.openFolderPath)
      const { tree, truncated } = await scan(resolved)
      if (truncated) warnings.push('Open folder tree was truncated')
      const paths: string[] = []
      collectMarkdownPaths(tree, paths)
      let added = 0
      for (const path of paths) {
        if (added >= MAX_FOLDER_FILES) {
          warnings.push(`Included first ${MAX_FOLDER_FILES} folder docs only`)
          break
        }
        const before = sources.length
        await addSource(sources, warnings, seen, path, budget, readFile)
        if (sources.length > before) added += 1
      }
    } catch {
      warnings.push('Could not scan open folder for context')
    }
  }

  const names = sources.map((s) => basename(s.path))
  const summary =
    sources.length === 0
      ? 'No docs in context'
      : `Using ${names.slice(0, 3).join(', ')}${names.length > 3 ? ` + ${names.length - 3} more` : ''}`

  return { sources, warnings, summary }
}

export function formatContextPrompt(packet: CompanionContextPacket, question: string): string {
  const blocks = packet.sources
    .map((s) => `### Source ${s.sourceId}\nPath: ${s.path}\n\n\`\`\`markdown\n${s.excerpt}\n\`\`\``)
    .join('\n\n')

  return [
    'You are Mdow Companion, a read-only docs assistant.',
    'Answer using the provided markdown sources.',
    'Cite source IDs like src:/absolute/path.md when making doc-specific claims.',
    'If the docs do not contain enough information, say so.',
    'Do not edit files, run commands, or request tools.',
    '',
    '## Docs context',
    blocks || '(no sources)',
    '',
    '## User question',
    question,
  ].join('\n')
}
