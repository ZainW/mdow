import { basename, isAbsolute, resolve } from 'path'
import { isDocumentPath } from '../../shared/types'
import type { CompanionFileChange, CompanionUpdate } from '../../shared/types'
import type { OpencodeEvent } from './opencode-client'

const MAX_DETAIL_CHARS = 4_000

/** Tools that change files. Mdow shows these as reviewable change cards, not tool rows. */
const WRITE_TOOLS = new Set(['edit', 'write', 'patch', 'apply_patch', 'multiedit'])

export interface TurnFinish {
  outcome: 'succeeded' | 'failed' | 'interrupted'
  error?: string
}

export interface TranslateResult {
  updates: CompanionUpdate[]
  /** A permission Mdow declines on the user's behalf, such as running shell commands. */
  autoReject?: { permissionId: string; reason: string }
  finish?: TurnFinish
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function str(value: unknown): string | undefined {
  return typeof value === 'string' ? value : undefined
}

function clip(text: string): string {
  return text.length > MAX_DETAIL_CHARS ? `${text.slice(0, MAX_DETAIL_CHARS)}\n…` : text
}

function formatInput(input: unknown): string | undefined {
  if (input === undefined) return undefined
  return clip(typeof input === 'string' ? input : JSON.stringify(input, null, 2))
}

function textFromContent(content: unknown): string {
  if (!Array.isArray(content)) return typeof content === 'string' ? content : ''
  return content
    .flatMap((block) => (isRecord(block) && typeof block.text === 'string' ? [block.text] : []))
    .join('\n')
}

function inputPath(input: unknown): string | undefined {
  if (!isRecord(input)) return undefined
  return str(input.path) ?? str(input.filePath) ?? str(input.file)
}

export function toolTitle(name: string, input?: unknown): string {
  const path = inputPath(input)
  const record = isRecord(input) ? input : {}
  switch (name) {
    case 'read':
      return path ? `Read ${basename(path)}` : 'Read a document'
    case 'grep':
    case 'search': {
      const pattern = str(record.pattern) ?? str(record.query)
      return pattern ? `Searched for “${pattern}”` : 'Searched the folder'
    }
    case 'glob':
    case 'list':
    case 'ls':
    case 'find':
      return 'Looked through the folder'
    case 'webfetch':
      return str(record.url) ? `Fetched ${str(record.url)}` : 'Fetched a page'
    case 'websearch':
      return str(record.query) ? `Searched the web for “${str(record.query)}”` : 'Searched the web'
    case 'todowrite':
    case 'todoread':
      return 'Updated its plan'
    default:
      return name.charAt(0).toUpperCase() + name.slice(1)
  }
}

export function fileChangesFrom(metadata: unknown, directory: string): CompanionFileChange[] {
  if (!isRecord(metadata) || !Array.isArray(metadata.files)) return []
  return metadata.files.flatMap((entry) => {
    if (!isRecord(entry)) return []
    const file = str(entry.file) ?? str(entry.path)
    if (!file) return []
    const status =
      entry.status === 'added' || entry.status === 'deleted' ? entry.status : 'modified'
    return [
      {
        path: isAbsolute(file) ? file : resolve(directory, file),
        displayPath: file,
        patch: str(entry.patch) ?? '',
        additions: typeof entry.additions === 'number' ? entry.additions : 0,
        deletions: typeof entry.deletions === 'number' ? entry.deletions : 0,
        status,
      } satisfies CompanionFileChange,
    ]
  })
}

/**
 * Translates the OpenCode event stream for one session into Companion updates. One instance
 * lives as long as the session, so tool names and pending permissions carry across turns.
 */
export class OpencodeTurnTranslator {
  private readonly toolNames = new Map<string, string>()
  private readonly toolInputs = new Map<string, unknown>()
  private readonly permissionTools = new Map<string, string>()
  private readonly rejectedPermissions = new Set<string>()
  private readonly cited = new Set<string>()
  private reasoningOpen = false

  constructor(
    readonly sessionId: string,
    readonly directory: string,
  ) {}

  /** Clears per-turn state; call before each prompt. */
  beginTurn(): void {
    this.cited.clear()
    this.reasoningOpen = false
  }

  translate(event: OpencodeEvent): TranslateResult {
    const data = event.data
    if (str(data.sessionID) !== this.sessionId) return { updates: [] }

    switch (event.type) {
      case 'session.text.delta': {
        const delta = str(data.delta)
        if (!delta) return { updates: [] }
        return { updates: [...this.closeReasoning(), { kind: 'delta', text: delta }] }
      }
      case 'session.reasoning.delta': {
        const delta = str(data.delta)
        if (!delta) return { updates: [] }
        this.reasoningOpen = true
        return { updates: [{ kind: 'thinking', text: delta }] }
      }
      case 'session.reasoning.ended':
        return { updates: this.closeReasoning() }
      case 'session.tool.input.started':
        return this.toolStarted(str(data.id), str(data.name))
      case 'session.tool.called':
        return this.toolCalled(str(data.id), data.input)
      case 'session.tool.success':
        return this.toolSucceeded(str(data.id), data.content, data.metadata)
      case 'session.tool.failed':
        return this.toolFailed(str(data.id), data.error)
      case 'permission.asked':
        return this.permissionAsked(data)
      case 'permission.replied':
        return this.permissionReplied(str(data.requestID), str(data.reply))
      case 'session.execution.succeeded':
        return { updates: this.closeReasoning(), finish: { outcome: 'succeeded' } }
      case 'session.execution.failed': {
        const error = isRecord(data.error) ? str(data.error.message) : undefined
        return {
          updates: this.closeReasoning(),
          finish: { outcome: 'failed', error: error ?? 'OpenCode could not finish the response.' },
        }
      }
      default:
        if (
          event.type.startsWith('session.execution.') &&
          event.type !== 'session.execution.started'
        ) {
          return { updates: this.closeReasoning(), finish: { outcome: 'interrupted' } }
        }
        return { updates: [] }
    }
  }

  private closeReasoning(): CompanionUpdate[] {
    if (!this.reasoningOpen) return []
    this.reasoningOpen = false
    return [{ kind: 'thinking-done' }]
  }

  private toolStarted(id: string | undefined, name: string | undefined): TranslateResult {
    if (!id || !name) return { updates: [] }
    this.toolNames.set(id, name)
    if (WRITE_TOOLS.has(name)) return { updates: this.closeReasoning() }
    return {
      updates: [
        ...this.closeReasoning(),
        { kind: 'tool', toolCallId: id, name: toolTitle(name), state: 'pending' },
      ],
    }
  }

  private toolCalled(id: string | undefined, input: unknown): TranslateResult {
    if (!id) return { updates: [] }
    const name = this.toolNames.get(id) ?? 'tool'
    if (WRITE_TOOLS.has(name)) return { updates: [] }
    this.toolInputs.set(id, input)
    const updates: CompanionUpdate[] = [
      {
        kind: 'tool',
        toolCallId: id,
        name: toolTitle(name, input),
        state: 'running',
        input: formatInput(input),
      },
    ]
    const path = inputPath(input)
    if (name === 'read' && path && isDocumentPath(path)) {
      const absolute = isAbsolute(path) ? path : resolve(this.directory, path)
      if (!this.cited.has(absolute)) {
        this.cited.add(absolute)
        updates.push({
          kind: 'citation',
          citation: { sourceId: `read:${absolute}`, path: absolute, label: basename(absolute) },
        })
      }
    }
    return { updates }
  }

  private toolSucceeded(
    id: string | undefined,
    content: unknown,
    metadata: unknown,
  ): TranslateResult {
    if (!id) return { updates: [] }
    const name = this.toolNames.get(id) ?? 'tool'
    const files = fileChangesFrom(metadata, this.directory)
    if (WRITE_TOOLS.has(name) || files.length > 0) {
      return {
        updates: [
          { kind: 'change', toolCallId: id, status: 'applied', ...(files.length ? { files } : {}) },
        ],
      }
    }
    return {
      updates: [
        {
          kind: 'tool',
          toolCallId: id,
          name: toolTitle(name, this.toolInputs.get(id)),
          state: 'completed',
          output: clip(textFromContent(content)),
        },
      ],
    }
  }

  private toolFailed(id: string | undefined, error: unknown): TranslateResult {
    if (!id) return { updates: [] }
    const name = this.toolNames.get(id) ?? 'tool'
    const message = (isRecord(error) ? str(error.message) : str(error)) ?? 'Tool failed'
    if (WRITE_TOOLS.has(name)) {
      const rejected = [...this.permissionTools].some(
        ([permissionId, toolId]) => toolId === id && this.rejectedPermissions.has(permissionId),
      )
      return {
        updates: [
          rejected
            ? { kind: 'change', toolCallId: id, status: 'rejected' }
            : { kind: 'change', toolCallId: id, status: 'failed', error: message },
        ],
      }
    }
    return {
      updates: [
        {
          kind: 'tool',
          toolCallId: id,
          name: toolTitle(name, this.toolInputs.get(id)),
          state: 'error',
          error: message,
        },
      ],
    }
  }

  private permissionAsked(data: Record<string, unknown>): TranslateResult {
    const permissionId = str(data.id)
    if (!permissionId) return { updates: [] }
    const action = str(data.action) ?? 'unknown'
    const toolCallId = (isRecord(data.source) ? str(data.source.id) : undefined) ?? permissionId
    this.permissionTools.set(permissionId, toolCallId)

    if (action === 'edit' || action === 'write') {
      const files = fileChangesFrom(data.metadata, this.directory)
      return {
        updates: [
          ...this.closeReasoning(),
          { kind: 'change', toolCallId, permissionId, status: 'pending', files },
        ],
      }
    }

    // Mdow is a document companion: anything beyond reading and editing documents in the
    // open folder (shell commands, other folders, the network) is declined automatically.
    const resources = Array.isArray(data.resources)
      ? data.resources.filter((item): item is string => typeof item === 'string')
      : []
    const target = resources.length ? ` (${resources.join(', ')})` : ''
    return {
      updates: [],
      autoReject: {
        permissionId,
        reason: `Mdow does not allow the ${action} permission${target}. Work only with the documents in this folder.`,
      },
    }
  }

  private permissionReplied(
    permissionId: string | undefined,
    reply: string | undefined,
  ): TranslateResult {
    if (!permissionId) return { updates: [] }
    const toolCallId = this.permissionTools.get(permissionId)
    if (reply !== 'reject' || !toolCallId) return { updates: [] }
    this.rejectedPermissions.add(permissionId)
    const name = this.toolNames.get(toolCallId)
    if (name && !WRITE_TOOLS.has(name)) return { updates: [] }
    return { updates: [{ kind: 'change', toolCallId, status: 'rejected' }] }
  }
}
