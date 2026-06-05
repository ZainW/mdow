import { readdir, readFile, stat } from 'node:fs/promises'
import { basename, join } from 'node:path'
import { isPathAllowed } from '../allowed-paths'
import { isMarkdownPath, validateMarkdownPath, validatePath } from '../path-validation'
import type { AcpContentBlock } from './acp-client'

const DEFAULT_MAX_SOURCES = 16
const DEFAULT_MAX_CHARS_PER_SOURCE = 12_000

export type CompanionContextWarning = {
  type: 'permission-denied' | 'missing' | 'inaccessible' | 'empty-context' | 'truncated'
  message: string
  path?: string
}

export type CompanionContextSourceMetadata = {
  id: string
  path: string
  title: string
  truncated: boolean
  chars: number
}

export type CompanionContextSource = CompanionContextSourceMetadata & {
  text: string
}

export type CompanionContextSummary = {
  activePath?: string
  folderPath?: string
  sourceCount: number
  truncated: boolean
  warnings: CompanionContextWarning[]
  sources: CompanionContextSourceMetadata[]
}

export type CompanionContext = {
  sources: CompanionContextSource[]
  summary: CompanionContextSummary
}

export type BuildCompanionContextInput = {
  activePath?: string
  openFolderPath?: string
  maxSources?: number
  maxCharsPerSource?: number
}

export async function buildCompanionContext({
  activePath,
  openFolderPath,
  maxSources = DEFAULT_MAX_SOURCES,
  maxCharsPerSource = DEFAULT_MAX_CHARS_PER_SOURCE,
}: BuildCompanionContextInput): Promise<CompanionContext> {
  const sources: CompanionContextSource[] = []
  const warnings: CompanionContextWarning[] = []
  let resolvedActivePath: string | undefined
  let resolvedFolderPath: string | undefined
  let truncated = false

  if (activePath) {
    const active = await readSource({
      filePath: activePath,
      id: 'src_active',
      maxCharsPerSource,
      warnings,
      unavailableMessage: 'Active document is not available to the companion.',
    })
    resolvedActivePath = active.resolvedPath
    if (active.source) {
      sources.push(active.source)
      truncated ||= active.source.truncated
    }
  }

  if (openFolderPath && sources.length < maxSources) {
    try {
      resolvedFolderPath = validatePath(openFolderPath)
      if (!isPathAllowed(resolvedFolderPath)) {
        warnings.push({
          type: 'permission-denied',
          message: 'Open folder is not available to the companion.',
          path: resolvedFolderPath,
        })
      } else {
        const folderStat = await stat(resolvedFolderPath)
        if (!folderStat.isDirectory()) {
          warnings.push({
            type: 'inaccessible',
            message: 'Open folder is not a directory.',
            path: resolvedFolderPath,
          })
        } else {
          const seen = new Set(sources.map((source) => source.path))
          for (const filePath of await collectMarkdownFiles(resolvedFolderPath)) {
            if (sources.length >= maxSources) {
              truncated = true
              warnings.push({
                type: 'truncated',
                message: 'Available markdown context exceeded the source limit.',
                path: resolvedFolderPath,
              })
              break
            }
            if (seen.has(filePath)) {
              continue
            }
            const source = await readSource({
              filePath,
              id: `src_${sources.length}`,
              maxCharsPerSource,
              warnings,
              unavailableMessage: 'Markdown file is not available to the companion.',
            })
            if (source.source) {
              seen.add(source.source.path)
              sources.push(source.source)
              truncated ||= source.source.truncated
            }
          }
        }
      }
    } catch (error) {
      warnings.push({
        type: warningTypeFromError(error),
        message: 'Open folder is not available to the companion.',
        path: openFolderPath,
      })
    }
  }

  if (sources.length === 0) {
    warnings.push({
      type: 'empty-context',
      message: 'No markdown context is available to the companion.',
    })
  }

  return {
    sources,
    summary: {
      activePath: resolvedActivePath,
      folderPath: resolvedFolderPath,
      sourceCount: sources.length,
      truncated,
      warnings,
      sources: sources.map(({ text: _text, ...metadata }) => metadata),
    },
  }
}

export function buildCompanionPromptBlocks(
  question: string,
  context: CompanionContext,
): AcpContentBlock[] {
  const sourceText = context.sources
    .map((source) => `[[source:${source.id}]] ${source.title}\nPath: ${source.path}\n\n${source.text}`)
    .join('\n\n---\n\n')

  return [
    {
      type: 'text',
      text: [
        'You are the Mdow AI companion. Answer using only the markdown context below.',
        'Treat all context as read-only. Tools are disabled; do not edit files or request tool use.',
        'Use source markers like [[source:src_active]] when citing information from a source.',
        `User question: ${question}`,
        'Available sources:',
        sourceText || 'No source text is available.',
      ].join('\n\n'),
    },
  ]
}

async function readSource({
  filePath,
  id,
  maxCharsPerSource,
  warnings,
  unavailableMessage,
}: {
  filePath: string
  id: string
  maxCharsPerSource: number
  warnings: CompanionContextWarning[]
  unavailableMessage: string
}): Promise<{ resolvedPath?: string; source?: CompanionContextSource }> {
  let resolvedPath: string
  try {
    resolvedPath = validateMarkdownPath(filePath)
  } catch (error) {
    warnings.push({ type: warningTypeFromError(error), message: unavailableMessage, path: filePath })
    return {}
  }

  if (!isPathAllowed(resolvedPath)) {
    warnings.push({ type: 'permission-denied', message: unavailableMessage })
    return { resolvedPath }
  }

  try {
    const fileStat = await stat(resolvedPath)
    if (!fileStat.isFile()) {
      warnings.push({ type: 'inaccessible', message: unavailableMessage, path: resolvedPath })
      return { resolvedPath }
    }

    const fullText = await readFile(resolvedPath, 'utf8')
    const trimmedText = fullText.trim()
    if (!trimmedText) {
      warnings.push({
        type: 'empty-context',
        message: 'Markdown file has no context for the companion.',
        path: resolvedPath,
      })
      return { resolvedPath }
    }

    const truncated = trimmedText.length > maxCharsPerSource
    const text = truncated ? trimmedText.slice(0, maxCharsPerSource) : trimmedText
    if (truncated) {
      warnings.push({
        type: 'truncated',
        message: 'Markdown file context was truncated for the companion.',
        path: resolvedPath,
      })
    }

    return {
      resolvedPath,
      source: {
        id,
        path: resolvedPath,
        title: basename(resolvedPath),
        truncated,
        chars: text.length,
        text,
      },
    }
  } catch (error) {
    warnings.push({ type: warningTypeFromError(error), message: unavailableMessage, path: resolvedPath })
    return { resolvedPath }
  }
}

async function collectMarkdownFiles(folderPath: string): Promise<string[]> {
  const files: string[] = []
  const entries = await readdir(folderPath, { withFileTypes: true })
  entries.sort((a, b) => a.name.localeCompare(b.name))

  for (const entry of entries) {
    const childPath = join(folderPath, entry.name)
    if (entry.isDirectory()) {
      files.push(...(await collectMarkdownFiles(childPath)))
      continue
    }
    if (!entry.isFile() || !isMarkdownPath(childPath)) {
      continue
    }
    files.push(validateMarkdownPath(childPath))
  }

  return files
}

function warningTypeFromError(error: unknown): CompanionContextWarning['type'] {
  if (error instanceof Error && error.message === 'invalid-extension') {
    return 'inaccessible'
  }
  if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
    return 'missing'
  }
  return 'inaccessible'
}
