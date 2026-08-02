import { basename } from 'path'
import { readFileContent } from '../file-service'
import type { scanFolder } from '../folder-service'
import { validateMarkdownPath } from '../path-validation'
import type {
  CompanionContextPacket,
  CompanionContextSource,
  CompanionContextTag,
  CompanionContextTraceItem,
} from '../../shared/types'
import type { ContextLedger } from './context-ledger'
import { selectInitialMarkdown } from './context-selection'

const MAX_INITIAL_SOURCE_BYTES = 16_384
const MAX_INITIAL_TOTAL_BYTES = 16_384

export interface BuildContextInput {
  activePath: string | null
  openFolderPath: string | null
  tags: CompanionContextTag[]
  question?: string
  ledger?: ContextLedger
  readFile?: (path: string) => Promise<string>
  scan?: typeof scanFolder
}

function sourceIdFor(path: string, headingId?: string): string {
  return headingId ? `src:${path}#${headingId}` : `src:${path}`
}

function truncateToBytes(text: string, maxBytes: number): string {
  if (Buffer.byteLength(text, 'utf8') <= maxBytes) return text
  let end = Math.min(text.length, maxBytes)
  while (end > 0 && Buffer.byteLength(text.slice(0, end), 'utf8') > maxBytes) end -= 1
  return text.slice(0, end)
}

function formatUnchangedSource(
  path: string,
  hash: string,
  headings: Array<{ depth: number; text: string; line: number }>,
  links: Array<{ label: string; target: string }>,
): string {
  const headingMap = headings.map(
    (heading) =>
      `${'  '.repeat(Math.max(heading.depth - 1, 0))}- ${heading.text} (line ${heading.line})`,
  )
  const linkMap = links.map((link) => `- ${link.label}: ${link.target}`)
  return [
    'Content unchanged from earlier in this session.',
    `Path: ${path}`,
    `SHA-256: ${hash}`,
    headingMap.length > 0 ? `Headings:\n${headingMap.join('\n')}` : '',
    linkMap.length > 0 ? `Linked documents:\n${linkMap.join('\n')}` : '',
  ]
    .filter(Boolean)
    .join('\n\n')
}

async function addSource(
  sources: CompanionContextSource[],
  warnings: string[],
  seen: Set<string>,
  path: string,
  budget: { used: number },
  readFile: (path: string) => Promise<string>,
  reason: CompanionContextTraceItem['reason'],
  traceItems: CompanionContextTraceItem[],
  question: string,
  ledger?: ContextLedger,
): Promise<void> {
  let resolved: string
  try {
    resolved = validateMarkdownPath(path)
  } catch {
    warnings.push(`Skipped non-markdown or invalid path: ${path}`)
    return
  }
  if (seen.has(resolved)) return
  if (budget.used >= MAX_INITIAL_TOTAL_BYTES) {
    warnings.push('Context budget reached; additional files omitted')
    return
  }

  try {
    const raw = await readFile(resolved)
    const remaining = Math.min(
      MAX_INITIAL_SOURCE_BYTES,
      MAX_INITIAL_TOTAL_BYTES - budget.used,
    )
    const selected = selectInitialMarkdown(raw, question, remaining)
    const ledgerRecord = ledger?.record(resolved, raw)
    const excerpt = ledgerRecord?.alreadySent
      ? truncateToBytes(
          formatUnchangedSource(
            resolved,
            ledgerRecord.hash,
            selected.headings,
            selected.links,
          ),
          remaining,
        )
      : selected.excerpt
    const bytes = Buffer.byteLength(excerpt, 'utf8')
    budget.used += bytes
    seen.add(resolved)
    sources.push({
      sourceId: sourceIdFor(resolved),
      path: resolved,
      excerpt,
      bytes,
    })
    traceItems.push({ path: resolved, reason, bytes })
    if (!ledgerRecord?.alreadySent && !selected.wholeDocument) {
      warnings.push(`Selected relevant sections from ${basename(resolved)}`)
    }
  } catch {
    warnings.push(`Could not read ${path}`)
  }
}

export async function buildCompanionContext(
  input: BuildContextInput,
): Promise<CompanionContextPacket> {
  const readFile = input.readFile ?? readFileContent
  const sources: CompanionContextSource[] = []
  const warnings: string[] = []
  const traceItems: CompanionContextTraceItem[] = []
  const seen = new Set<string>()
  const budget = { used: 0 }

  if (input.activePath) {
    await addSource(
      sources,
      warnings,
      seen,
      input.activePath,
      budget,
      readFile,
      'focused',
      traceItems,
      input.question ?? '',
      input.ledger,
    )
  }

  for (const tag of input.tags) {
    if (tag.kind === 'file') {
      await addSource(
        sources,
        warnings,
        seen,
        tag.path,
        budget,
        readFile,
        'attached',
        traceItems,
        input.question ?? '',
        input.ledger,
      )
    }
  }

  const names = sources.map((s) => basename(s.path))
  const summary =
    sources.length === 0
      ? 'No docs in context'
      : `Using ${names.slice(0, 3).join(', ')}${names.length > 3 ? ` + ${names.length - 3} more` : ''}`

  const injectedBytes = traceItems.reduce((total, item) => total + item.bytes, 0)
  return {
    sources,
    warnings,
    summary,
    trace: {
      focusedCount: traceItems.filter((item) => item.reason === 'focused').length,
      attachedCount: traceItems.filter((item) => item.reason === 'attached').length,
      searchedCount: 0,
      readRangeCount: 0,
      injectedBytes,
      estimatedTokens: Math.ceil(injectedBytes / 4),
      retrievalMode: 'focused-only',
      items: traceItems,
    },
  }
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
    'Search linked files or attached folders only when the question requires more context.',
    'Use only read-only context or search tools made available by Mdow.',
    'Do not edit files, use write tools, run terminal commands, or grant permissions.',
    '',
    '## Docs context',
    blocks || '(no sources)',
    '',
    '## User question',
    question,
  ].join('\n')
}
