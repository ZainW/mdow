# AI Companion Chat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a read-only AI companion chat that connects Mdow to local ACP providers, answers from the active markdown document plus open-folder docs, and presents one shared conversation in a right panel and full-screen view.

**Architecture:** Keep provider execution in the Electron main process. The main process owns provider detection, newline-delimited ACP JSON-RPC over subprocess stdio, read-only context building, and streamed IPC updates. The renderer owns session-only chat state and UI, using copied AI Elements/shadcn primitives adapted to Mdow's Tailwind v4 design tokens.

**Tech Stack:** Electron, React 19, TypeScript, Zustand, Vitest, Testing Library, Tailwind CSS v4, shadcn/ui, Vercel AI Elements registry components, Agent Client Protocol v1 over stdio.

---

## File Structure

Create these main-process files:

- `apps/desktop/src/main/companion/provider-detection.ts`: provider candidate definitions, custom command parsing, and safe availability checks that never trigger `npx` installs.
- `apps/desktop/src/main/companion/provider-detection.test.ts`: provider detection tests with mocked subprocesses.
- `apps/desktop/src/main/companion/acp-client.ts`: newline-delimited JSON-RPC client for ACP initialize, session creation, prompt turns, streaming updates, cancellation, and shutdown.
- `apps/desktop/src/main/companion/acp-client.test.ts`: ACP framing, lifecycle, streaming, cancellation, and unsupported tool request tests.
- `apps/desktop/src/main/companion/context-builder.ts`: safe markdown context collection, source IDs, truncation warnings, and ACP prompt text/resource construction.
- `apps/desktop/src/main/companion/context-builder.test.ts`: context builder tests with temporary markdown files.
- `apps/desktop/src/main/companion/service.ts`: orchestration layer used by IPC handlers.
- `apps/desktop/src/main/companion/service.test.ts`: service tests using mocked provider detection, ACP client, and context builder.

Modify these existing main/preload/shared files:

- `apps/desktop/src/shared/types.ts`: companion types and IPC constants.
- `apps/desktop/src/main/store.ts`: persisted companion provider settings.
- `apps/desktop/src/main/store.test.ts`: store default and persistence tests.
- `apps/desktop/src/main/ipc.ts`: register companion IPC handlers and stream events to the renderer.
- `apps/desktop/src/preload/index.ts`: expose typed companion methods and event subscriptions.

Create these renderer files:

- `apps/desktop/src/renderer/src/store/slices/companion-slice.ts`: session-only panel/full-screen/message/provider state.
- `apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`: store behavior tests.
- `apps/desktop/src/renderer/src/hooks/useCompanionController.ts`: connects Zustand state to `window.api` companion methods and updates.
- `apps/desktop/src/renderer/src/hooks/useCompanionController.test.tsx`: hook tests with mocked `window.api`.
- `apps/desktop/src/renderer/src/lib/companion-citations.ts`: parse and validate `[[source:id]]` markers from assistant text.
- `apps/desktop/src/renderer/src/lib/companion-citations.test.ts`: citation extraction tests.
- `apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx`: right-side compact companion surface.
- `apps/desktop/src/renderer/src/components/companion/CompanionFullscreen.tsx`: expanded shared-session surface.
- `apps/desktop/src/renderer/src/components/companion/CompanionMessages.tsx`: message rendering, streaming state, sources, warnings.
- `apps/desktop/src/renderer/src/components/companion/CompanionComposer.tsx`: prompt input, send, cancel, keyboard behavior.
- `apps/desktop/src/renderer/src/components/companion/CompanionSetup.tsx`: provider empty/setup state.
- `apps/desktop/src/renderer/src/components/companion/CompanionStatus.tsx`: provider chip and context summary.
- `apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`: panel, setup, composer, message, and full-screen integration tests.

Create or adapt these AI Elements files through the shadcn registry:

- `apps/desktop/src/renderer/src/components/ai-elements/conversation.tsx`
- `apps/desktop/src/renderer/src/components/ai-elements/message.tsx`
- `apps/desktop/src/renderer/src/components/ai-elements/prompt-input.tsx`
- `apps/desktop/src/renderer/src/components/ai-elements/sources.tsx`

Modify these renderer integration files:

- `apps/desktop/src/renderer/src/store/app-store.ts`: include the companion slice and exported types.
- `apps/desktop/src/renderer/src/App.tsx`: render `CompanionPanel` and `CompanionFullscreen` beside existing document chrome.
- `apps/desktop/src/renderer/src/components/SettingsDialog.tsx`: add companion provider preference and custom command settings.
- `apps/desktop/src/renderer/src/assets/styles/index.css`: add Streamdown source scanning if AI Elements `MessageResponse` requires it, plus companion width variables.
- `apps/desktop/src/renderer/src/test/setup.ts`: add `EventSource` or clipboard shims only if the copied AI Elements components require them during tests.

## Task 1: Shared Companion Types And Persisted Settings

**Files:**
- Modify: `apps/desktop/src/shared/types.ts`
- Modify: `apps/desktop/src/main/store.ts`
- Create or modify: `apps/desktop/src/main/store.test.ts`

- [ ] **Step 1: Write failing store tests**

Add `apps/desktop/src/main/store.test.ts` with this test harness, or extend the file if it already exists:

```ts
import { beforeEach, describe, expect, it, vi } from 'vitest'

const memory = vi.hoisted(() => new Map<string, unknown>())

vi.mock('electron-store', () => ({
  default: class MockStore<TSchema extends Record<string, unknown>> {
    constructor(options: { defaults: TSchema }) {
      for (const [key, value] of Object.entries(options.defaults)) {
        if (!memory.has(key)) memory.set(key, value)
      }
    }

    get<TKey extends keyof TSchema>(key: TKey): TSchema[TKey] {
      return memory.get(String(key)) as TSchema[TKey]
    }

    set<TKey extends keyof TSchema>(key: TKey, value: TSchema[TKey]): void {
      memory.set(String(key), value)
    }
  },
}))

vi.mock('fs', () => ({ existsSync: () => true }))

describe('store companion settings', () => {
  beforeEach(() => {
    memory.clear()
    vi.resetModules()
  })

  it('returns default companion settings with app state', async () => {
    const { getAppState } = await import('./store')

    expect(getAppState().companionProvider).toBe('auto')
    expect(getAppState().companionCustomCommand).toBe('')
  })

  it('persists companion provider and custom command through saveAppState', async () => {
    const { getAppState, saveAppState } = await import('./store')

    saveAppState({
      companionProvider: 'custom',
      companionCustomCommand: '/usr/local/bin/custom-acp --stdio',
    })

    expect(getAppState().companionProvider).toBe('custom')
    expect(getAppState().companionCustomCommand).toBe('/usr/local/bin/custom-acp --stdio')
  })
})
```

- [ ] **Step 2: Run the failing tests**

Run:

```bash
pnpm run --filter desktop test -- src/main/store.test.ts
```

Expected: FAIL because `companionProvider` and `companionCustomCommand` do not exist on the store state.

- [ ] **Step 3: Add shared companion types and IPC constants**

Modify `apps/desktop/src/shared/types.ts` to add these types near the existing app-state types:

```ts
export type CompanionProviderId = 'auto' | 'opencode' | 'codex' | 'custom'

export type CompanionProviderStatusState = 'available' | 'missing' | 'failed'

export interface CompanionProviderStatus {
  id: Exclude<CompanionProviderId, 'auto'>
  label: string
  command: string
  status: CompanionProviderStatusState
  installHint?: string
  error?: string
}

export interface CompanionSettings {
  provider: CompanionProviderId
  customCommand: string
}

export type CompanionMessageRole = 'user' | 'assistant' | 'system'
export type CompanionMessageStatus = 'complete' | 'streaming' | 'error'

export interface CompanionCitation {
  sourceId: string
  title: string
  path: string
  heading?: string
}

export interface CompanionContextSource {
  id: string
  title: string
  path: string
  heading?: string
}

export interface CompanionContextWarning {
  type: 'missing-file' | 'permission-denied' | 'truncated' | 'no-context'
  message: string
}

export interface CompanionContextSummary {
  activePath: string | null
  folderPath: string | null
  sourceCount: number
  truncated: boolean
  warnings: CompanionContextWarning[]
  sources: CompanionContextSource[]
}

export interface CompanionMessage {
  id: string
  role: CompanionMessageRole
  content: string
  status: CompanionMessageStatus
  citations: CompanionCitation[]
  createdAt: number
}

export type CompanionUpdate =
  | { type: 'status'; status: 'starting' | 'ready' | 'streaming' | 'complete' | 'cancelled' }
  | { type: 'assistant-delta'; messageId: string; text: string }
  | { type: 'context'; summary: CompanionContextSummary }
  | { type: 'warning'; warning: CompanionContextWarning }
  | { type: 'tool-refused'; title: string }
  | { type: 'error'; message: string }

export interface CompanionSendRequest {
  messageId: string
  text: string
  provider: CompanionProviderId
  activePath: string | null
  openFolderPath: string | null
}
```

Extend `AppState` in the same file:

```ts
export interface AppState {
  recents?: string[]
  zoomLevel: number
  lastFolder: string | null
  windowBounds: { x: number; y: number; width: number; height: number } | null
  sessionTabs: { path: string }[]
  sessionActiveTabPath: string | null
  contentFont: string
  codeFont: string
  theme: string
  autoUpdateEnabled: boolean
  wideMode: boolean
  interfaceScale: InterfaceScale
  readingWidth: ReadingWidth
  sidebarMode: SidebarMode
  companionProvider: CompanionProviderId
  companionCustomCommand: string
}
```

Add these IPC constants to `IPC`:

```ts
COMPANION_DETECT_PROVIDERS: 'companion:detect-providers',
COMPANION_GET_SETTINGS: 'companion:get-settings',
COMPANION_SAVE_SETTINGS: 'companion:save-settings',
COMPANION_START_SESSION: 'companion:start-session',
COMPANION_SEND: 'companion:send',
COMPANION_CANCEL: 'companion:cancel',
COMPANION_SHUTDOWN: 'companion:shutdown',
COMPANION_UPDATE: 'companion:update',
```

- [ ] **Step 4: Persist companion fields in the main store**

Modify `apps/desktop/src/main/store.ts`:

```ts
import Store from 'electron-store'
import { existsSync } from 'fs'
import type { CompanionProviderId } from '../shared/types'
```

Add fields to `StoreSchema`:

```ts
  companionProvider: CompanionProviderId
  companionCustomCommand: string
```

Add defaults:

```ts
    companionProvider: 'auto',
    companionCustomCommand: '',
```

Return fields from `getAppState()`:

```ts
    companionProvider: store.get('companionProvider'),
    companionCustomCommand: store.get('companionCustomCommand'),
```

Persist fields in `saveAppState()`:

```ts
  if (state.companionProvider !== undefined) store.set('companionProvider', state.companionProvider)
  if (state.companionCustomCommand !== undefined)
    store.set('companionCustomCommand', state.companionCustomCommand)
```

- [ ] **Step 5: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/main/store.test.ts
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/shared/types.ts apps/desktop/src/main/store.ts apps/desktop/src/main/store.test.ts
git commit -m "feat: persist companion settings"
```

## Task 2: Provider Detection

**Files:**
- Create: `apps/desktop/src/main/companion/provider-detection.ts`
- Create: `apps/desktop/src/main/companion/provider-detection.test.ts`

- [ ] **Step 1: Write failing provider detection tests**

Create `apps/desktop/src/main/companion/provider-detection.test.ts`:

```ts
import { EventEmitter } from 'node:events'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const spawnMock = vi.hoisted(() => vi.fn())

vi.mock('node:child_process', () => ({ spawn: spawnMock }))

import { detectCompanionProviders } from './provider-detection'

function mockProcess({ exitCode, delay = 0 }: { exitCode?: number; delay?: number }) {
  const proc = new EventEmitter() as EventEmitter & { kill: ReturnType<typeof vi.fn> }
  proc.kill = vi.fn(() => true)
  setTimeout(() => {
    if (exitCode !== undefined) proc.emit('exit', exitCode, null)
  }, delay)
  return proc
}

describe('provider detection', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    spawnMock.mockReset()
  })

  it('parses quoted custom commands', () => {
    expect(parseProviderCommand('"/Applications/My Agent/acp" --stdio --flag')).toEqual({
      command: '/Applications/My Agent/acp',
      args: ['--stdio', '--flag'],
    })
  })

  it('marks opencode available when the process starts and stays alive briefly', async () => {
    spawnMock.mockImplementation((command: string) => {
      if (command === 'opencode') return mockProcess({ delay: 10_000 })
      return mockProcess({ exitCode: 1 })
    })

    const promise = detectCompanionProviders('')
    await vi.advanceTimersByTimeAsync(900)
    const statuses = await promise

    expect(statuses.find((s) => s.id === 'opencode')?.status).toBe('available')
    expect(spawnMock).toHaveBeenCalledWith('opencode', ['acp'], expect.any(Object))
  })

  it('detects the Codex adapter without allowing npx installs', async () => {
    spawnMock.mockImplementation((command: string, args: string[]) => {
      if (command === 'npx' && args[0] === '--no-install') return mockProcess({ delay: 10_000 })
      return mockProcess({ exitCode: 1 })
    })

    const promise = detectCompanionProviders('')
    await vi.advanceTimersByTimeAsync(900)
    const statuses = await promise

    const codex = statuses.find((s) => s.id === 'codex')
    expect(codex?.status).toBe('available')
    expect(codex?.command).toBe('npx --no-install @zed-industries/codex-acp')
  })

  it('returns missing for failed built-in commands and includes a custom provider when configured', async () => {
    spawnMock.mockImplementation(() => mockProcess({ exitCode: 127 }))

    const promise = detectCompanionProviders('/usr/local/bin/acp --docs')
    await vi.advanceTimersByTimeAsync(900)
    const statuses = await promise

    expect(statuses.find((s) => s.id === 'opencode')?.status).toBe('missing')
    expect(statuses.find((s) => s.id === 'codex')?.status).toBe('missing')
    expect(statuses.find((s) => s.id === 'custom')?.command).toBe('/usr/local/bin/acp --docs')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- src/main/companion/provider-detection.test.ts
```

Expected: FAIL because `provider-detection.ts` does not exist.

- [ ] **Step 3: Implement provider detection**

Create `apps/desktop/src/main/companion/provider-detection.ts`:

```ts
import { spawn } from 'node:child_process'
import type { CompanionProviderStatus } from '../../shared/types'

interface ProviderCommand {
  command: string
  args: string[]
}

interface ProviderCandidate extends ProviderCommand {
  id: CompanionProviderStatus['id']
  label: string
  displayCommand: string
  installHint?: string
}

const DETECTION_TIMEOUT_MS = 750

const BUILT_IN_CANDIDATES: ProviderCandidate[] = [
  {
    id: 'opencode',
    label: 'opencode',
    command: 'opencode',
    args: ['acp'],
    displayCommand: 'opencode acp',
    installHint: 'Install opencode, then make sure the opencode command is on PATH.',
  },
  {
    id: 'codex',
    label: 'Codex ACP',
    command: 'npx',
    args: ['--no-install', '@zed-industries/codex-acp'],
    displayCommand: 'npx --no-install @zed-industries/codex-acp',
    installHint: 'Install @zed-industries/codex-acp before using the Codex ACP adapter.',
  },
]

export function parseProviderCommand(input: string): ProviderCommand {
  const parts: string[] = []
  const pattern = /"([^"]+)"|'([^']+)'|\S+/g
  for (const match of input.matchAll(pattern)) {
    parts.push(match[1] ?? match[2] ?? match[0])
  }
  const [command = '', ...args] = parts
  return { command, args }
}

async function detectCandidate(candidate: ProviderCandidate): Promise<CompanionProviderStatus> {
  return new Promise((resolve) => {
    let settled = false
    let child: ReturnType<typeof spawn> | null = null

    const finish = (status: CompanionProviderStatus['status'], error?: string) => {
      if (settled) return
      settled = true
      if (child && status === 'available') child.kill()
      resolve({
        id: candidate.id,
        label: candidate.label,
        command: candidate.displayCommand,
        installHint: candidate.installHint,
        status,
        error,
      })
    }

    try {
      child = spawn(candidate.command, candidate.args, {
        stdio: ['ignore', 'ignore', 'ignore'],
        env: { ...process.env, npm_config_yes: 'false' },
      })
    } catch (err) {
      finish('failed', err instanceof Error ? err.message : 'Failed to start provider')
      return
    }

    child.once('error', (err) => finish('missing', err.message))
    child.once('exit', (code) => finish(code === 0 ? 'available' : 'missing', `Exited with ${code}`))
    setTimeout(() => finish('available'), DETECTION_TIMEOUT_MS)
  })
}

export async function detectCompanionProviders(
  customCommand: string,
): Promise<CompanionProviderStatus[]> {
  const candidates = [...BUILT_IN_CANDIDATES]
  const trimmed = customCommand.trim()
  if (trimmed) {
    const parsed = parseProviderCommand(trimmed)
    candidates.push({
      id: 'custom',
      label: 'Custom',
      command: parsed.command,
      args: parsed.args,
      displayCommand: trimmed,
    })
  }
  return Promise.all(candidates.map((candidate) => detectCandidate(candidate)))
}
```

- [ ] **Step 4: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/main/companion/provider-detection.test.ts
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/main/companion/provider-detection.ts apps/desktop/src/main/companion/provider-detection.test.ts
git commit -m "feat: detect local companion providers"
```

## Task 3: ACP Client Over Newline-Delimited JSON-RPC

**Files:**
- Create: `apps/desktop/src/main/companion/acp-client.ts`
- Create: `apps/desktop/src/main/companion/acp-client.test.ts`

- [ ] **Step 1: Write failing ACP client tests**

Create `apps/desktop/src/main/companion/acp-client.test.ts`:

```ts
import { EventEmitter } from 'node:events'
import { PassThrough, Writable } from 'node:stream'
import { describe, expect, it, vi } from 'vitest'
import { createAcpClient, type AcpProcessFactory } from './acp-client'

function createHarness() {
  const stdout = new PassThrough()
  const stderr = new PassThrough()
  const writes: string[] = []
  const stdin = new Writable({
    write(chunk, _encoding, callback) {
      writes.push(chunk.toString())
      callback()
    },
  })
  const child = new EventEmitter() as EventEmitter & {
    stdin: Writable
    stdout: PassThrough
    stderr: PassThrough
    kill: ReturnType<typeof vi.fn>
  }
  child.stdin = stdin
  child.stdout = stdout
  child.stderr = stderr
  child.kill = vi.fn(() => true)
  const factory: AcpProcessFactory = () => child
  return { child, stdout, writes, factory }
}

function writeJson(stdout: PassThrough, message: unknown) {
  stdout.write(`${JSON.stringify(message)}\n`)
}

describe('ACP client', () => {
  it('initializes, creates a session, and sends text prompts', async () => {
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'opencode',
      args: ['acp'],
      cwd: '/tmp/docs',
      processFactory: factory,
      onUpdate: vi.fn(),
    })

    const start = client.start()
    const initialize = JSON.parse(writes[0])
    expect(initialize.method).toBe('initialize')
    expect(initialize.params.clientCapabilities).toEqual({})
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: initialize.id,
      result: { protocolVersion: 1, agentCapabilities: {}, authMethods: [] },
    })

    const sessionNew = JSON.parse(writes[1])
    expect(sessionNew.method).toBe('session/new')
    expect(sessionNew.params).toEqual({ cwd: '/tmp/docs', mcpServers: [] })
    writeJson(stdout, { jsonrpc: '2.0', id: sessionNew.id, result: { sessionId: 'sess_1' } })
    await start

    const prompt = client.sendPrompt([{ type: 'text', text: 'hello' }])
    const promptRequest = JSON.parse(writes[2])
    expect(promptRequest.method).toBe('session/prompt')
    expect(promptRequest.params.sessionId).toBe('sess_1')
    expect(promptRequest.params.prompt).toEqual([{ type: 'text', text: 'hello' }])
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: promptRequest.id,
      result: { stopReason: 'end_turn' },
    })
    await prompt
  })

  it('streams agent message chunks and refuses permission requests', async () => {
    const onUpdate = vi.fn()
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'opencode',
      args: ['acp'],
      cwd: '/tmp/docs',
      processFactory: factory,
      onUpdate,
    })

    const start = client.start()
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: JSON.parse(writes[0]).id,
      result: { protocolVersion: 1, agentCapabilities: {}, authMethods: [] },
    })
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: JSON.parse(writes[1]).id,
      result: { sessionId: 'sess_1' },
    })
    await start

    writeJson(stdout, {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        sessionId: 'sess_1',
        update: {
          sessionUpdate: 'agent_message_chunk',
          content: { type: 'text', text: 'Answer part' },
        },
      },
    })
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: 99,
      method: 'session/request_permission',
      params: { sessionId: 'sess_1', toolCall: { title: 'Write file' } },
    })

    expect(onUpdate).toHaveBeenCalledWith({ type: 'assistant-delta', text: 'Answer part' })
    expect(onUpdate).toHaveBeenCalledWith({ type: 'tool-refused', title: 'Write file' })
    const refusal = writes.map((line) => JSON.parse(line)).find((msg) => msg.id === 99)
    expect(refusal.result).toEqual({ outcome: 'rejected' })
  })

  it('sends session/cancel notifications and kills the process on stop', async () => {
    const { child, stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'opencode',
      args: ['acp'],
      cwd: '/tmp/docs',
      processFactory: factory,
      onUpdate: vi.fn(),
    })
    const start = client.start()
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: JSON.parse(writes[0]).id,
      result: { protocolVersion: 1, agentCapabilities: {}, authMethods: [] },
    })
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: JSON.parse(writes[1]).id,
      result: { sessionId: 'sess_1' },
    })
    await start

    client.cancel()
    expect(JSON.parse(writes.at(-1)!).method).toBe('session/cancel')

    client.stop()
    expect(child.kill).toHaveBeenCalled()
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- src/main/companion/acp-client.test.ts
```

Expected: FAIL because `acp-client.ts` does not exist.

- [ ] **Step 3: Implement the ACP client**

Create `apps/desktop/src/main/companion/acp-client.ts` with newline-delimited JSON-RPC. Use the ACP v1 stdio transport rule: one UTF-8 JSON-RPC message per line, no embedded newlines in each envelope.

```ts
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'

type JsonRpcId = number

interface JsonRpcRequest {
  jsonrpc: '2.0'
  id: JsonRpcId
  method: string
  params?: unknown
}

interface JsonRpcNotification {
  jsonrpc: '2.0'
  method: string
  params?: unknown
}

interface JsonRpcResponse {
  jsonrpc: '2.0'
  id: JsonRpcId
  result?: unknown
  error?: { code: number; message: string }
}

export type AcpContentBlock =
  | { type: 'text'; text: string }
  | { type: 'resource'; resource: { uri: string; mimeType?: string; text: string } }

export type AcpClientUpdate =
  | { type: 'assistant-delta'; text: string }
  | { type: 'tool-refused'; title: string }
  | { type: 'error'; message: string }

export type AcpProcessFactory = (
  command: string,
  args: string[],
  options: { cwd: string; env: NodeJS.ProcessEnv },
) => ChildProcessWithoutNullStreams

interface AcpClientOptions {
  command: string
  args: string[]
  cwd: string
  processFactory?: AcpProcessFactory
  onUpdate: (update: AcpClientUpdate) => void
}

export interface AcpClient {
  start: () => Promise<void>
  sendPrompt: (prompt: AcpContentBlock[]) => Promise<void>
  cancel: () => void
  stop: () => void
}

export function createAcpClient(options: AcpClientOptions): AcpClient {
  const processFactory = options.processFactory ?? ((command, args, opts) => spawn(command, args, opts))
  let child: ChildProcessWithoutNullStreams | null = null
  let nextId = 1
  let buffer = ''
  let sessionId: string | null = null
  const pending = new Map<JsonRpcId, (response: JsonRpcResponse) => void>()

  function write(message: JsonRpcRequest | JsonRpcNotification | JsonRpcResponse): void {
    child?.stdin.write(`${JSON.stringify(message)}\n`)
  }

  function request(method: string, params?: unknown): Promise<unknown> {
    const id = nextId++
    write({ jsonrpc: '2.0', id, method, params })
    return new Promise((resolve, reject) => {
      pending.set(id, (response) => {
        if (response.error) reject(new Error(response.error.message))
        else resolve(response.result)
      })
    })
  }

  function handleRequest(message: JsonRpcRequest): void {
    if (message.method === 'session/request_permission') {
      const params = message.params as { toolCall?: { title?: string } } | undefined
      options.onUpdate({ type: 'tool-refused', title: params?.toolCall?.title ?? 'Tool request' })
      write({ jsonrpc: '2.0', id: message.id, result: { outcome: 'rejected' } })
      return
    }
    write({ jsonrpc: '2.0', id: message.id, error: { code: -32601, message: 'Unsupported method' } })
  }

  function handleNotification(message: JsonRpcNotification): void {
    if (message.method !== 'session/update') return
    const params = message.params as
      | { update?: { sessionUpdate?: string; content?: { type?: string; text?: string }; title?: string } }
      | undefined
    const update = params?.update
    if (update?.sessionUpdate === 'agent_message_chunk' && update.content?.type === 'text') {
      options.onUpdate({ type: 'assistant-delta', text: update.content.text ?? '' })
    }
    if (update?.sessionUpdate === 'tool_call') {
      options.onUpdate({ type: 'tool-refused', title: update.title ?? 'Tool request' })
    }
  }

  function handleLine(line: string): void {
    if (!line.trim()) return
    const message = JSON.parse(line) as JsonRpcRequest | JsonRpcNotification | JsonRpcResponse
    if ('id' in message && ('result' in message || 'error' in message)) {
      const callback = pending.get(message.id)
      pending.delete(message.id)
      callback?.(message)
      return
    }
    if ('id' in message && 'method' in message) {
      handleRequest(message)
      return
    }
    if ('method' in message) handleNotification(message)
  }

  function attachStdout(): void {
    child?.stdout.on('data', (chunk) => {
      buffer += chunk.toString('utf8')
      let newlineIndex = buffer.indexOf('\n')
      while (newlineIndex !== -1) {
        const line = buffer.slice(0, newlineIndex)
        buffer = buffer.slice(newlineIndex + 1)
        handleLine(line)
        newlineIndex = buffer.indexOf('\n')
      }
    })
  }

  return {
    async start() {
      child = processFactory(options.command, options.args, {
        cwd: options.cwd,
        env: { ...process.env, npm_config_yes: 'false' },
      })
      attachStdout()
      const init = (await request('initialize', {
        protocolVersion: 1,
        clientCapabilities: {},
        clientInfo: { name: 'mdow', title: 'Mdow', version: '1.0.0' },
      })) as { protocolVersion?: number }
      if (init.protocolVersion !== 1) throw new Error('Unsupported ACP protocol version')
      const session = (await request('session/new', {
        cwd: options.cwd,
        mcpServers: [],
      })) as { sessionId?: string }
      if (!session.sessionId) throw new Error('ACP provider did not create a session')
      sessionId = session.sessionId
    },
    async sendPrompt(prompt) {
      if (!sessionId) throw new Error('ACP session has not started')
      await request('session/prompt', { sessionId, prompt })
    },
    cancel() {
      if (!sessionId) return
      write({ jsonrpc: '2.0', method: 'session/cancel', params: { sessionId } })
    },
    stop() {
      child?.kill()
      child = null
      sessionId = null
      pending.clear()
    },
  }
}
```

- [ ] **Step 4: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/main/companion/acp-client.test.ts
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/main/companion/acp-client.ts apps/desktop/src/main/companion/acp-client.test.ts
git commit -m "feat: add read-only ACP client"
```

## Task 4: Read-Only Context Builder And Source Markers

**Files:**
- Create: `apps/desktop/src/main/companion/context-builder.ts`
- Create: `apps/desktop/src/main/companion/context-builder.test.ts`

- [ ] **Step 1: Write failing context builder tests**

Create `apps/desktop/src/main/companion/context-builder.test.ts`:

```ts
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { beforeEach, describe, expect, it } from 'vitest'
import { clearAllowedPaths, registerAllowedFile, registerAllowedPath } from '../allowed-paths'
import { buildCompanionContext, buildCompanionPromptBlocks } from './context-builder'

async function createDocs() {
  const root = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
  await mkdir(join(root, 'guide'))
  const active = join(root, 'README.md')
  const guide = join(root, 'guide', 'install.md')
  const text = join(root, 'notes.txt')
  await writeFile(active, '# Mdow\n\nOpen markdown files and read quietly.')
  await writeFile(guide, '# Install\n\nDownload the release for your platform.')
  await writeFile(text, 'not markdown')
  return { root, active, guide, text }
}

describe('companion context builder', () => {
  beforeEach(() => clearAllowedPaths())

  it('prioritizes the active markdown document and includes open-folder markdown files', async () => {
    const docs = await createDocs()
    registerAllowedFile(docs.active)
    registerAllowedPath(docs.root)

    const context = await buildCompanionContext({
      activePath: docs.active,
      openFolderPath: docs.root,
      maxSources: 8,
      maxCharsPerSource: 1_000,
    })

    expect(context.sources.map((source) => source.path)).toEqual([docs.active, docs.guide])
    expect(context.sources[0].id).toBe('src_active')
    expect(context.summary.sourceCount).toBe(2)
    expect(context.summary.truncated).toBe(false)
  })

  it('omits paths outside allowed roots and reports a context warning', async () => {
    const docs = await createDocs()

    const context = await buildCompanionContext({
      activePath: docs.active,
      openFolderPath: docs.root,
      maxSources: 8,
      maxCharsPerSource: 1_000,
    })

    expect(context.sources).toHaveLength(0)
    expect(context.summary.warnings[0]).toEqual({
      type: 'permission-denied',
      message: 'Active document is not available to the companion.',
    })
  })

  it('builds text-only prompt blocks with explicit source marker instructions', async () => {
    const docs = await createDocs()
    registerAllowedFile(docs.active)
    registerAllowedPath(docs.root)
    const context = await buildCompanionContext({
      activePath: docs.active,
      openFolderPath: docs.root,
      maxSources: 8,
      maxCharsPerSource: 1_000,
    })

    const blocks = buildCompanionPromptBlocks('How do I install it?', context)

    expect(blocks).toHaveLength(1)
    expect(blocks[0].type).toBe('text')
    expect(blocks[0].text).toContain('Use source markers like [[source:src_active]]')
    expect(blocks[0].text).toContain('# Install')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- src/main/companion/context-builder.test.ts
```

Expected: FAIL because `context-builder.ts` does not exist.

- [ ] **Step 3: Implement context builder**

Create `apps/desktop/src/main/companion/context-builder.ts`:

```ts
import { readdir, readFile, stat } from 'node:fs/promises'
import { basename, join } from 'node:path'
import { isPathAllowed } from '../allowed-paths'
import { validateMarkdownPath, validatePath } from '../path-validation'
import type { AcpContentBlock } from './acp-client'
import type {
  CompanionContextSource,
  CompanionContextSummary,
  CompanionContextWarning,
} from '../../shared/types'

interface BuildCompanionContextInput {
  activePath: string | null
  openFolderPath: string | null
  maxSources?: number
  maxCharsPerSource?: number
}

interface CompanionContextSourceWithText extends CompanionContextSource {
  text: string
}

export interface CompanionContextPacket {
  sources: CompanionContextSourceWithText[]
  summary: CompanionContextSummary
}

const DEFAULT_MAX_SOURCES = 16
const DEFAULT_MAX_CHARS_PER_SOURCE = 12_000

async function readMarkdownSource(
  id: string,
  path: string,
  maxChars: number,
): Promise<CompanionContextSourceWithText | null> {
  const resolved = validateMarkdownPath(path)
  if (!isPathAllowed(resolved)) return null
  const text = await readFile(resolved, 'utf-8')
  return {
    id,
    title: basename(resolved),
    path: resolved,
    text: text.slice(0, maxChars),
  }
}

async function collectMarkdownFiles(folderPath: string, limit: number): Promise<string[]> {
  const resolvedFolder = validatePath(folderPath)
  if (!isPathAllowed(resolvedFolder)) return []
  const files: string[] = []

  async function walk(dir: string): Promise<void> {
    if (files.length >= limit) return
    const entries = await readdir(dir, { withFileTypes: true })
    for (const entry of entries.toSorted((a, b) => a.name.localeCompare(b.name))) {
      if (files.length >= limit) return
      if (entry.name.startsWith('.') || entry.name === 'node_modules') continue
      const fullPath = join(dir, entry.name)
      if (entry.isDirectory()) {
        await walk(fullPath)
      } else {
        try {
          validateMarkdownPath(fullPath)
          files.push(fullPath)
        } catch {
          continue
        }
      }
    }
  }

  await walk(resolvedFolder)
  return files
}

export async function buildCompanionContext(
  input: BuildCompanionContextInput,
): Promise<CompanionContextPacket> {
  const maxSources = input.maxSources ?? DEFAULT_MAX_SOURCES
  const maxCharsPerSource = input.maxCharsPerSource ?? DEFAULT_MAX_CHARS_PER_SOURCE
  const sources: CompanionContextSourceWithText[] = []
  const warnings: CompanionContextWarning[] = []
  let truncated = false

  if (input.activePath) {
    try {
      const active = await readMarkdownSource('src_active', input.activePath, maxCharsPerSource)
      if (active) sources.push(active)
      else warnings.push({ type: 'permission-denied', message: 'Active document is not available to the companion.' })
    } catch {
      warnings.push({ type: 'missing-file', message: 'Active document could not be read.' })
    }
  }

  if (input.openFolderPath && sources.length < maxSources) {
    try {
      const folderStat = await stat(validatePath(input.openFolderPath))
      if (folderStat.isDirectory()) {
        const folderFiles = await collectMarkdownFiles(input.openFolderPath, maxSources + 1)
        let index = 1
        for (const file of folderFiles) {
          if (sources.length >= maxSources) {
            truncated = true
            break
          }
          if (sources.some((source) => source.path === file)) continue
          const source = await readMarkdownSource(`src_${index}`, file, maxCharsPerSource)
          index += 1
          if (source) sources.push(source)
        }
      }
    } catch {
      warnings.push({ type: 'missing-file', message: 'Open folder could not be read.' })
    }
  }

  if (truncated) {
    warnings.push({ type: 'truncated', message: `Context was limited to ${maxSources} markdown files.` })
  }
  if (sources.length === 0) {
    warnings.push({ type: 'no-context', message: 'Open a markdown file or folder before asking doc-specific questions.' })
  }

  return {
    sources,
    summary: {
      activePath: input.activePath,
      folderPath: input.openFolderPath,
      sourceCount: sources.length,
      truncated,
      warnings,
      sources: sources.map(({ text: _text, ...source }) => source),
    },
  }
}

export function buildCompanionPromptBlocks(
  question: string,
  context: CompanionContextPacket,
): AcpContentBlock[] {
  const sourceText = context.sources
    .map((source) => `SOURCE ${source.id}\nTITLE: ${source.title}\nPATH: ${source.path}\n\n${source.text}`)
    .join('\n\n---\n\n')

  return [
    {
      type: 'text',
      text: `You are Mdow's read-only documentation companion. Answer from the provided markdown sources. Do not edit files, run commands, or request tools. Use source markers like [[source:src_active]] or [[source:src_1]] for doc-specific claims. Only use source IDs listed below. If the docs do not contain the answer, say that.\n\nUSER QUESTION:\n${question}\n\nAVAILABLE SOURCES:\n${sourceText}`,
    },
  ]
}
```

- [ ] **Step 4: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/main/companion/context-builder.test.ts
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/main/companion/context-builder.ts apps/desktop/src/main/companion/context-builder.test.ts
git commit -m "feat: build read-only companion context"
```

## Task 5: Companion Main Service And IPC

**Files:**
- Create: `apps/desktop/src/main/companion/service.ts`
- Create: `apps/desktop/src/main/companion/service.test.ts`
- Modify: `apps/desktop/src/main/ipc.ts`
- Modify: `apps/desktop/src/preload/index.ts`

- [ ] **Step 1: Write service tests**

Create `apps/desktop/src/main/companion/service.test.ts`:

```ts
import { describe, expect, it, vi } from 'vitest'
import type { CompanionSendRequest } from '../../shared/types'
import { createCompanionService } from './service'

describe('companion service', () => {
  it('detects providers with the persisted custom command', async () => {
    const detectProviders = vi.fn(async () => [
      { id: 'opencode', label: 'opencode', command: 'opencode acp', status: 'available' as const },
    ])
    const service = createCompanionService({
      detectProviders,
      createClient: vi.fn(),
      buildContext: vi.fn(),
      getSettings: () => ({ provider: 'auto', customCommand: 'custom acp' }),
      emitUpdate: vi.fn(),
    })

    await service.detectProviders()

    expect(detectProviders).toHaveBeenCalledWith('custom acp')
  })

  it('builds context, starts an ACP client, streams status, and sends prompt blocks', async () => {
    const sendPrompt = vi.fn(async () => {})
    const start = vi.fn(async () => {})
    const emitUpdate = vi.fn()
    const request: CompanionSendRequest = {
      messageId: 'msg_1',
      text: 'What is Mdow?',
      provider: 'opencode',
      activePath: '/docs/README.md',
      openFolderPath: '/docs',
    }
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({ start, sendPrompt, cancel: vi.fn(), stop: vi.fn() })),
      buildContext: vi.fn(async () => ({
        sources: [{ id: 'src_active', title: 'README.md', path: '/docs/README.md', text: '# Mdow' }],
        summary: {
          activePath: '/docs/README.md',
          folderPath: '/docs',
          sourceCount: 1,
          truncated: false,
          warnings: [],
          sources: [{ id: 'src_active', title: 'README.md', path: '/docs/README.md' }],
        },
      })),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    await service.send(request)

    expect(start).toHaveBeenCalledOnce()
    expect(sendPrompt).toHaveBeenCalledWith(expect.arrayContaining([{ type: 'text', text: expect.stringContaining('What is Mdow?') }]))
    expect(emitUpdate).toHaveBeenCalledWith({ type: 'status', status: 'streaming' })
    expect(emitUpdate).toHaveBeenCalledWith(expect.objectContaining({ type: 'context' }))
    expect(emitUpdate).toHaveBeenCalledWith({ type: 'status', status: 'complete' })
  })

  it('chooses the first available provider when the request provider is auto', async () => {
    const createClient = vi.fn(() => ({
      start: vi.fn(async () => {}),
      sendPrompt: vi.fn(async () => {}),
      cancel: vi.fn(),
      stop: vi.fn(),
    }))
    const service = createCompanionService({
      detectProviders: vi.fn(async () => [
        { id: 'opencode', label: 'opencode', command: 'opencode acp', status: 'missing' as const },
        {
          id: 'codex',
          label: 'Codex ACP',
          command: 'npx --no-install @zed-industries/codex-acp',
          status: 'available' as const,
        },
      ]),
      createClient,
      buildContext: vi.fn(async () => ({
        sources: [],
        summary: {
          activePath: null,
          folderPath: null,
          sourceCount: 0,
          truncated: false,
          warnings: [],
          sources: [],
        },
      })),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate: vi.fn(),
    })

    await service.send({
      messageId: 'msg_1',
      text: 'Hello',
      provider: 'auto',
      activePath: null,
      openFolderPath: null,
    })

    expect(createClient).toHaveBeenCalledWith(
      expect.objectContaining({ command: 'npx', args: ['--no-install', '@zed-industries/codex-acp'] }),
    )
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- src/main/companion/service.test.ts
```

Expected: FAIL because `service.ts` does not exist.

- [ ] **Step 3: Implement the service**

Create `apps/desktop/src/main/companion/service.ts`:

```ts
import { dirname } from 'node:path'
import type { CompanionProviderStatus, CompanionSendRequest, CompanionSettings, CompanionUpdate } from '../../shared/types'
import { detectCompanionProviders } from './provider-detection'
import { createAcpClient, type AcpClient, type AcpClientUpdate } from './acp-client'
import { buildCompanionContext, buildCompanionPromptBlocks, type CompanionContextPacket } from './context-builder'

interface CompanionServiceDeps {
  detectProviders: (customCommand: string) => Promise<CompanionProviderStatus[]>
  createClient: (options: {
    command: string
    args: string[]
    cwd: string
    onUpdate: (update: AcpClientUpdate) => void
  }) => AcpClient
  buildContext: typeof buildCompanionContext
  getSettings: () => CompanionSettings
  emitUpdate: (update: CompanionUpdate) => void
}

export interface CompanionService {
  detectProviders: () => Promise<CompanionProviderStatus[]>
  send: (request: CompanionSendRequest) => Promise<void>
  cancel: () => void
  shutdown: () => void
}

async function resolveProviderCommand(
  provider: CompanionSendRequest['provider'],
  settings: CompanionSettings,
  detectProviders: (customCommand: string) => Promise<CompanionProviderStatus[]>,
) {
  const requested = provider === 'auto' ? settings.provider : provider
  if (requested === 'auto') {
    const statuses = await detectProviders(settings.customCommand)
    const available = statuses.find((status) => status.status === 'available')
    if (!available) throw new Error('No local companion provider is available')
    return resolveProviderCommand(available.id, settings, detectProviders)
  }
  if (requested === 'custom') {
    const parts = settings.customCommand.trim().match(/"([^"]+)"|'([^']+)'|\S+/g) ?? []
    const [command = '', ...args] = parts.map((part) => part.replace(/^['"]|['"]$/g, ''))
    return { command, args }
  }
  if (requested === 'codex') {
    return { command: 'npx', args: ['--no-install', '@zed-industries/codex-acp'] }
  }
  return { command: 'opencode', args: ['acp'] }
}

export function createCompanionService(deps: CompanionServiceDeps): CompanionService {
  let client: AcpClient | null = null
  let activeAssistantMessageId: string | null = null

  function forwardAcpUpdate(update: AcpClientUpdate): void {
    if (update.type === 'assistant-delta' && activeAssistantMessageId) deps.emitUpdate({ type: 'assistant-delta', messageId: activeAssistantMessageId, text: update.text })
    if (update.type === 'tool-refused') deps.emitUpdate({ type: 'tool-refused', title: update.title })
    if (update.type === 'error') deps.emitUpdate({ type: 'error', message: update.message })
  }

  return {
    detectProviders() {
      return deps.detectProviders(deps.getSettings().customCommand)
    },
    async send(request) {
      activeAssistantMessageId = request.messageId
      const settings = deps.getSettings()
      const context: CompanionContextPacket = await deps.buildContext({
        activePath: request.activePath,
        openFolderPath: request.openFolderPath,
      })
      deps.emitUpdate({ type: 'context', summary: context.summary })
      for (const warning of context.summary.warnings) deps.emitUpdate({ type: 'warning', warning })

      if (!client) {
        const command = await resolveProviderCommand(request.provider, settings, deps.detectProviders)
        const cwd = request.openFolderPath ?? (request.activePath ? dirname(request.activePath) : process.cwd())
        client = deps.createClient({ ...command, cwd, onUpdate: forwardAcpUpdate })
        deps.emitUpdate({ type: 'status', status: 'starting' })
        await client.start()
      }

      deps.emitUpdate({ type: 'status', status: 'streaming' })
      await client.sendPrompt(buildCompanionPromptBlocks(request.text, context))
      deps.emitUpdate({ type: 'status', status: 'complete' })
    },
    cancel() {
      client?.cancel()
      deps.emitUpdate({ type: 'status', status: 'cancelled' })
    },
    shutdown() {
      client?.stop()
      client = null
    },
  }
}

export function createDefaultCompanionService(options: {
  getSettings: () => CompanionSettings
  emitUpdate: (update: CompanionUpdate) => void
}): CompanionService {
  return createCompanionService({
    detectProviders: detectCompanionProviders,
    createClient: ({ command, args, cwd, onUpdate }) => createAcpClient({ command, args, cwd, onUpdate }),
    buildContext: buildCompanionContext,
    getSettings: options.getSettings,
    emitUpdate: options.emitUpdate,
  })
}
```

- [ ] **Step 4: Add IPC and preload API**

Modify `apps/desktop/src/main/ipc.ts` inside `registerIpcHandlers` after store handlers:

```ts
  const companionService = createDefaultCompanionService({
    getSettings: () => {
      const state = getAppState()
      return {
        provider: state.companionProvider,
        customCommand: state.companionCustomCommand,
      }
    },
    emitUpdate: (update) => {
      const win = getMainWindow()
      if (!win || win.isDestroyed()) return
      win.webContents.send(IPC.COMPANION_UPDATE, update)
    },
  })

  ipcMain.handle(IPC.COMPANION_DETECT_PROVIDERS, () => companionService.detectProviders())
  ipcMain.handle(IPC.COMPANION_GET_SETTINGS, () => {
    const state = getAppState()
    return { provider: state.companionProvider, customCommand: state.companionCustomCommand }
  })
  ipcMain.handle(IPC.COMPANION_SAVE_SETTINGS, (_, settings: CompanionSettings) => {
    saveAppState({ companionProvider: settings.provider, companionCustomCommand: settings.customCommand })
  })
  ipcMain.handle(IPC.COMPANION_SEND, (_, request: CompanionSendRequest) => companionService.send(request))
  ipcMain.handle(IPC.COMPANION_CANCEL, () => companionService.cancel())
  ipcMain.handle(IPC.COMPANION_SHUTDOWN, () => companionService.shutdown())
```

Add imports at the top of `ipc.ts`:

```ts
import { IPC, type CompanionSendRequest, type CompanionSettings } from '../shared/types'
import { createDefaultCompanionService } from './companion/service'
```

Replace direct string channel usage in existing handlers with `IPC` constants only where touched by this task.

Modify `apps/desktop/src/preload/index.ts` imports and `ElectronAPI`:

```ts
  type CompanionProviderStatus,
  type CompanionSendRequest,
  type CompanionSettings,
  type CompanionUpdate,
```

```ts
  detectCompanionProviders: () => Promise<CompanionProviderStatus[]>
  getCompanionSettings: () => Promise<CompanionSettings>
  saveCompanionSettings: (settings: CompanionSettings) => Promise<void>
  sendCompanionMessage: (request: CompanionSendRequest) => Promise<void>
  cancelCompanionMessage: () => Promise<void>
  shutdownCompanion: () => Promise<void>
  onCompanionUpdate: (callback: (update: CompanionUpdate) => void) => Unsubscribe
```

Add API methods:

```ts
  detectCompanionProviders: () => ipcRenderer.invoke(IPC.COMPANION_DETECT_PROVIDERS),
  getCompanionSettings: () => ipcRenderer.invoke(IPC.COMPANION_GET_SETTINGS),
  saveCompanionSettings: (settings) => ipcRenderer.invoke(IPC.COMPANION_SAVE_SETTINGS, settings),
  sendCompanionMessage: (request) => ipcRenderer.invoke(IPC.COMPANION_SEND, request),
  cancelCompanionMessage: () => ipcRenderer.invoke(IPC.COMPANION_CANCEL),
  shutdownCompanion: () => ipcRenderer.invoke(IPC.COMPANION_SHUTDOWN),
  onCompanionUpdate: (callback) => createIpcListener(IPC.COMPANION_UPDATE, callback),
```

- [ ] **Step 5: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/main/companion/service.test.ts
```

Expected: PASS.

Run:

```bash
pnpm run --filter desktop typecheck:node
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/main/companion/service.ts apps/desktop/src/main/companion/service.test.ts apps/desktop/src/main/ipc.ts apps/desktop/src/preload/index.ts
git commit -m "feat: expose companion IPC service"
```

## Task 6: Renderer Companion Store And Citation Parsing

**Files:**
- Create: `apps/desktop/src/renderer/src/store/slices/companion-slice.ts`
- Create: `apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`
- Create: `apps/desktop/src/renderer/src/lib/companion-citations.ts`
- Create: `apps/desktop/src/renderer/src/lib/companion-citations.test.ts`
- Modify: `apps/desktop/src/renderer/src/store/app-store.ts`

- [ ] **Step 1: Write failing store and citation tests**

Create `apps/desktop/src/renderer/src/lib/companion-citations.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { extractCompanionCitations } from './companion-citations'

describe('extractCompanionCitations', () => {
  it('strips valid source markers and returns citation chips', () => {
    const result = extractCompanionCitations('Mdow reads markdown. [[source:src_active]]', [
      { id: 'src_active', title: 'README.md', path: '/docs/README.md' },
    ])

    expect(result.text).toBe('Mdow reads markdown.')
    expect(result.citations).toEqual([
      { sourceId: 'src_active', title: 'README.md', path: '/docs/README.md' },
    ])
  })

  it('removes invalid source markers without creating trusted citations', () => {
    const result = extractCompanionCitations('Claim [[source:unknown]]', [])

    expect(result.text).toBe('Claim')
    expect(result.citations).toEqual([])
  })
})
```

Create `apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { useAppStore } from '../app-store'

describe('companion slice', () => {
  it('opens the panel and appends session-only messages', () => {
    useAppStore.getState().resetCompanion()

    useAppStore.getState().setCompanionOpen(true)
    const user = useAppStore.getState().appendCompanionMessage('user', 'What is Mdow?')
    const assistant = useAppStore.getState().appendCompanionMessage('assistant', '', 'streaming')
    useAppStore.getState().appendCompanionAssistantDelta(assistant.id, 'A reader.')

    expect(useAppStore.getState().companionOpen).toBe(true)
    expect(useAppStore.getState().companionMessages).toEqual([
      expect.objectContaining({ id: user.id, role: 'user', content: 'What is Mdow?' }),
      expect.objectContaining({ id: assistant.id, role: 'assistant', content: 'A reader.' }),
    ])
  })

  it('shares messages between compact and fullscreen state', () => {
    useAppStore.getState().resetCompanion()
    useAppStore.getState().appendCompanionMessage('user', 'Hello')
    useAppStore.getState().setCompanionFullscreen(true)

    expect(useAppStore.getState().companionFullscreen).toBe(true)
    expect(useAppStore.getState().companionMessages).toHaveLength(1)
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/lib/companion-citations.test.ts src/renderer/src/store/slices/companion-slice.test.ts
```

Expected: FAIL because the new modules do not exist.

- [ ] **Step 3: Implement citation parsing**

Create `apps/desktop/src/renderer/src/lib/companion-citations.ts`:

```ts
import type { CompanionCitation, CompanionContextSource } from '../../../shared/types'

export function extractCompanionCitations(
  content: string,
  sources: CompanionContextSource[],
): { text: string; citations: CompanionCitation[] } {
  const byId = new Map(sources.map((source) => [source.id, source]))
  const citations: CompanionCitation[] = []
  const text = content
    .replace(/\[\[source:([a-zA-Z0-9_-]+)\]\]/g, (_match, sourceId: string) => {
      const source = byId.get(sourceId)
      if (source && !citations.some((citation) => citation.sourceId === sourceId)) {
        citations.push({
          sourceId,
          title: source.title,
          path: source.path,
          heading: source.heading,
        })
      }
      return ''
    })
    .replace(/[ \t]+\n/g, '\n')
    .trim()
  return { text, citations }
}
```

- [ ] **Step 4: Implement companion slice and wire it into the app store**

Create `apps/desktop/src/renderer/src/store/slices/companion-slice.ts`:

```ts
import type { StateCreator } from 'zustand'
import type {
  CompanionContextSummary,
  CompanionMessage,
  CompanionMessageRole,
  CompanionMessageStatus,
  CompanionProviderId,
  CompanionProviderStatus,
} from '../../../../shared/types'

export interface CompanionSlice {
  companionOpen: boolean
  companionFullscreen: boolean
  companionStreaming: boolean
  companionProvider: CompanionProviderId
  companionCustomCommand: string
  companionProviders: CompanionProviderStatus[]
  companionMessages: CompanionMessage[]
  companionContext: CompanionContextSummary | null
  companionError: string | null
  setCompanionOpen: (open: boolean) => void
  setCompanionFullscreen: (open: boolean) => void
  setCompanionStreaming: (streaming: boolean) => void
  setCompanionProvider: (provider: CompanionProviderId) => void
  setCompanionCustomCommand: (command: string) => void
  setCompanionProviders: (providers: CompanionProviderStatus[]) => void
  setCompanionContext: (context: CompanionContextSummary | null) => void
  setCompanionError: (error: string | null) => void
  appendCompanionMessage: (
    role: CompanionMessageRole,
    content: string,
    status?: CompanionMessageStatus,
  ) => CompanionMessage
  appendCompanionAssistantDelta: (messageId: string, delta: string) => void
  updateCompanionMessage: (message: CompanionMessage) => void
  resetCompanion: () => void
}

const initialCompanionState = {
  companionOpen: false,
  companionFullscreen: false,
  companionStreaming: false,
  companionProvider: 'auto' as CompanionProviderId,
  companionCustomCommand: '',
  companionProviders: [] as CompanionProviderStatus[],
  companionMessages: [] as CompanionMessage[],
  companionContext: null as CompanionContextSummary | null,
  companionError: null as string | null,
}

export const createCompanionSlice: StateCreator<CompanionSlice, [], [], CompanionSlice> = (set) => ({
  ...initialCompanionState,
  setCompanionOpen: (open) => set({ companionOpen: open }),
  setCompanionFullscreen: (open) => set({ companionFullscreen: open }),
  setCompanionStreaming: (streaming) => set({ companionStreaming: streaming }),
  setCompanionProvider: (provider) => set({ companionProvider: provider }),
  setCompanionCustomCommand: (command) => set({ companionCustomCommand: command }),
  setCompanionProviders: (providers) => set({ companionProviders: providers }),
  setCompanionContext: (context) => set({ companionContext: context }),
  setCompanionError: (error) => set({ companionError: error }),
  appendCompanionMessage: (role, content, status = 'complete') => {
    const message: CompanionMessage = {
      id: crypto.randomUUID(),
      role,
      content,
      status,
      citations: [],
      createdAt: Date.now(),
    }
    set((state) => ({ companionMessages: [...state.companionMessages, message] }))
    return message
  },
  appendCompanionAssistantDelta: (messageId, delta) =>
    set((state) => ({
      companionMessages: state.companionMessages.map((message) =>
        message.id === messageId ? { ...message, content: message.content + delta } : message,
      ),
    })),
  updateCompanionMessage: (message) =>
    set((state) => ({
      companionMessages: state.companionMessages.map((item) =>
        item.id === message.id ? message : item,
      ),
    })),
  resetCompanion: () => set(initialCompanionState),
})
```

Modify `apps/desktop/src/renderer/src/store/app-store.ts`:

```ts
import { createCompanionSlice, type CompanionSlice } from './slices/companion-slice'
```

```ts
type AppStore = TabSlice & UiSlice & FolderSlice & SettingsSlice & CompanionSlice
```

```ts
  ...createCompanionSlice(...args),
```

- [ ] **Step 5: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/lib/companion-citations.test.ts src/renderer/src/store/slices/companion-slice.test.ts
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/renderer/src/lib/companion-citations.ts apps/desktop/src/renderer/src/lib/companion-citations.test.ts apps/desktop/src/renderer/src/store/slices/companion-slice.ts apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts apps/desktop/src/renderer/src/store/app-store.ts
git commit -m "feat: add companion renderer state"
```

## Task 7: Install And Adapt AI Elements Components

**Files:**
- Create: `apps/desktop/src/renderer/src/components/ai-elements/conversation.tsx`
- Create: `apps/desktop/src/renderer/src/components/ai-elements/message.tsx`
- Create: `apps/desktop/src/renderer/src/components/ai-elements/prompt-input.tsx`
- Create: `apps/desktop/src/renderer/src/components/ai-elements/sources.tsx`
- Modify: `apps/desktop/src/renderer/src/assets/styles/index.css`

- [ ] **Step 1: Add AI Elements through the shadcn registry**

Run from the repo root:

```bash
pnpm dlx shadcn@latest add -c apps/desktop @ai-elements/conversation @ai-elements/message @ai-elements/prompt-input @ai-elements/sources
```

Expected: files are created under `apps/desktop/src/renderer/src/components/ai-elements/`. If the registry creates files under a different configured alias path, move them into that directory and keep imports local to Mdow.

- [ ] **Step 2: Adapt imports to Mdow aliases and primitives**

In the copied AI Elements files, replace imports that start with `@/components/ui/` with `@renderer/components/ui/`, and imports that start with `@/lib/utils` with `@renderer/lib/utils`.

For `message.tsx`, keep `Message`, `MessageContent`, and `MessageResponse` exported. Do not keep attachment, branch, or action exports unless the registry component requires them for these three exports to compile.

For `prompt-input.tsx`, keep these exports:

```ts
export {
  PromptInput,
  PromptInputBody,
  PromptInputFooter,
  PromptInputSubmit,
  PromptInputTextarea,
}
export type { PromptInputMessage }
```

For `conversation.tsx`, keep these exports:

```ts
export {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
  ConversationScrollButton,
}
```

For `sources.tsx`, keep these exports:

```ts
export { Source, Sources, SourcesContent, SourcesTrigger }
```

- [ ] **Step 3: Add Streamdown source scanning when MessageResponse uses Streamdown**

If `apps/desktop/src/renderer/src/components/ai-elements/message.tsx` imports `streamdown`, add this line near the top of `apps/desktop/src/renderer/src/assets/styles/index.css`, after local font imports and before `@theme` usage:

```css
@source "../../../../../node_modules/streamdown/dist/*.js";
```

If the copied `MessageResponse` does not import `streamdown`, do not add the `@source` line.

- [ ] **Step 4: Run typecheck and commit**

Run:

```bash
pnpm run --filter desktop typecheck:web
```

Expected: PASS. If the copied components require missing shadcn primitives, add them with the required Mdow command format, for example:

```bash
pnpm dlx shadcn@latest add -c apps/desktop button
```

Commit:

```bash
git add apps/desktop/src/renderer/src/components/ai-elements apps/desktop/src/renderer/src/assets/styles/index.css apps/desktop/package.json pnpm-lock.yaml
git commit -m "feat: add AI Elements chat primitives"
```

## Task 8: Companion Controller Hook

**Files:**
- Create: `apps/desktop/src/renderer/src/hooks/useCompanionController.ts`
- Create: `apps/desktop/src/renderer/src/hooks/useCompanionController.test.tsx`

- [ ] **Step 1: Write failing hook tests**

Create `apps/desktop/src/renderer/src/hooks/useCompanionController.test.tsx`:

```tsx
import { renderHook, act } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from '../store/app-store'
import { useCompanionController } from './useCompanionController'

const api = vi.hoisted(() => ({
  detectCompanionProviders: vi.fn(async () => []),
  getCompanionSettings: vi.fn(async () => ({ provider: 'auto', customCommand: '' })),
  saveCompanionSettings: vi.fn(async () => {}),
  sendCompanionMessage: vi.fn(async () => {}),
  cancelCompanionMessage: vi.fn(async () => {}),
  onCompanionUpdate: vi.fn(() => () => {}),
}))

describe('useCompanionController', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    Object.defineProperty(window, 'api', { value: api, configurable: true })
    useAppStore.getState().resetCompanion()
    useAppStore.setState({ tabs: [], activeTabId: null, openFolderPath: null })
  })

  it('loads settings and providers on mount', async () => {
    api.detectCompanionProviders.mockResolvedValue([
      { id: 'opencode', label: 'opencode', command: 'opencode acp', status: 'available' },
    ])

    renderHook(() => useCompanionController())
    await act(async () => {})

    expect(useAppStore.getState().companionProvider).toBe('auto')
    expect(useAppStore.getState().companionProviders).toHaveLength(1)
  })

  it('appends user and assistant messages before sending to main', async () => {
    useAppStore.setState({
      tabs: [{ id: 'tab_1', path: '/docs/README.md', content: '# Mdow', scrollPosition: 0 }],
      activeTabId: 'tab_1',
      openFolderPath: '/docs',
    })
    const { result } = renderHook(() => useCompanionController())

    await act(async () => {
      await result.current.send('What is Mdow?')
    })

    expect(useAppStore.getState().companionMessages.map((message) => message.role)).toEqual([
      'user',
      'assistant',
    ])
    expect(api.sendCompanionMessage).toHaveBeenCalledWith(
      expect.objectContaining({ text: 'What is Mdow?', activePath: '/docs/README.md', openFolderPath: '/docs' }),
    )
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/hooks/useCompanionController.test.tsx
```

Expected: FAIL because `useCompanionController.ts` does not exist.

- [ ] **Step 3: Implement controller hook**

Create `apps/desktop/src/renderer/src/hooks/useCompanionController.ts`:

```ts
import { useCallback, useEffect, useEffectEvent } from 'react'
import { useAppStore, selectActiveTab } from '../store/app-store'
import { extractCompanionCitations } from '../lib/companion-citations'

export function useCompanionController() {
  const activeTab = useAppStore(selectActiveTab)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const provider = useAppStore((s) => s.companionProvider)
  const appendMessage = useAppStore((s) => s.appendCompanionMessage)
  const appendDelta = useAppStore((s) => s.appendCompanionAssistantDelta)
  const updateMessage = useAppStore((s) => s.updateCompanionMessage)
  const setStreaming = useAppStore((s) => s.setCompanionStreaming)
  const setProviders = useAppStore((s) => s.setCompanionProviders)
  const setProvider = useAppStore((s) => s.setCompanionProvider)
  const setCustomCommand = useAppStore((s) => s.setCompanionCustomCommand)
  const setContext = useAppStore((s) => s.setCompanionContext)
  const setError = useAppStore((s) => s.setCompanionError)

  const onUpdate = useEffectEvent((update: Parameters<typeof window.api.onCompanionUpdate>[0] extends (arg: infer T) => void ? T : never) => {
    if (update.type === 'status') setStreaming(update.status === 'streaming' || update.status === 'starting')
    if (update.type === 'context') setContext(update.summary)
    if (update.type === 'assistant-delta') appendDelta(update.messageId, update.text)
    if (update.type === 'error') setError(update.message)
  })

  useEffect(() => {
    let disposed = false
    void window.api.getCompanionSettings().then((settings) => {
      if (disposed) return
      setProvider(settings.provider)
      setCustomCommand(settings.customCommand)
    })
    void window.api.detectCompanionProviders().then((providers) => {
      if (!disposed) setProviders(providers)
    })
    const unsubscribe = window.api.onCompanionUpdate((update) => onUpdate(update))
    return () => {
      disposed = true
      unsubscribe()
    }
  }, [])

  const send = useCallback(
    async (text: string) => {
      const trimmed = text.trim()
      if (!trimmed) return
      appendMessage('user', trimmed)
      const assistant = appendMessage('assistant', '', 'streaming')
      setStreaming(true)
      await window.api.sendCompanionMessage({
        messageId: assistant.id,
        text: trimmed,
        provider,
        activePath: activeTab?.path ?? null,
        openFolderPath,
      })
      const state = useAppStore.getState()
      const latest = state.companionMessages.find((message) => message.id === assistant.id)
      if (latest) {
        const parsed = extractCompanionCitations(latest.content, state.companionContext?.sources ?? [])
        updateMessage({ ...latest, content: parsed.text, citations: parsed.citations, status: 'complete' })
      }
      setStreaming(false)
    },
    [activeTab?.path, appendMessage, openFolderPath, provider, setStreaming, updateMessage],
  )

  const cancel = useCallback(async () => {
    await window.api.cancelCompanionMessage()
    setStreaming(false)
  }, [setStreaming])

  return { send, cancel }
}
```

- [ ] **Step 4: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/hooks/useCompanionController.test.tsx
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/renderer/src/hooks/useCompanionController.ts apps/desktop/src/renderer/src/hooks/useCompanionController.test.tsx
git commit -m "feat: connect companion renderer to IPC"
```

## Task 9: Companion UI Surfaces

**Files:**
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionFullscreen.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionMessages.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionComposer.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionSetup.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionStatus.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`

- [ ] **Step 1: Write failing UI tests**

Create `apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`:

```tsx
import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from '../../store/app-store'
import { CompanionPanel } from './CompanionPanel'
import { CompanionFullscreen } from './CompanionFullscreen'

vi.mock('../../hooks/useCompanionController', () => ({
  useCompanionController: () => ({ send: vi.fn(), cancel: vi.fn() }),
}))

describe('Companion UI', () => {
  beforeEach(() => {
    useAppStore.getState().resetCompanion()
    useAppStore.setState({ sidebarMode: 'folder' })
  })

  it('renders the right companion panel without changing left sidebar mode', () => {
    useAppStore.setState({ companionOpen: true })
    render(<CompanionPanel />)

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toBeInTheDocument()
    expect(useAppStore.getState().sidebarMode).toBe('folder')
  })

  it('shows provider setup when no provider is available', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [] })
    render(<CompanionPanel />)

    expect(screen.getByText('Connect a local companion')).toBeInTheDocument()
    expect(screen.getByText('opencode acp')).toBeInTheDocument()
    expect(screen.getByText('npx --no-install @zed-industries/codex-acp')).toBeInTheDocument()
  })

  it('expands and collapses the same conversation', () => {
    useAppStore.setState({ companionOpen: true })
    useAppStore.getState().appendCompanionMessage('user', 'Hello')
    render(
      <>
        <CompanionPanel />
        <CompanionFullscreen />
      </>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Expand companion' }))

    expect(screen.getByRole('dialog', { name: 'AI companion' })).toBeInTheDocument()
    expect(screen.getAllByText('Hello')).toHaveLength(2)
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionPanel.test.tsx
```

Expected: FAIL because the companion components do not exist.

- [ ] **Step 3: Implement status and setup components**

Create `CompanionStatus.tsx`:

```tsx
import { Badge } from '../ui/badge'
import type { CompanionProviderStatus } from '../../../../shared/types'

export function CompanionStatus({ providers }: { providers: CompanionProviderStatus[] }) {
  const available = providers.find((provider) => provider.status === 'available')
  return <Badge variant="outline">{available ? available.label : 'Not connected'}</Badge>
}
```

Create `CompanionSetup.tsx`:

```tsx
import { Bot, RefreshCcw } from 'lucide-react'
import { Button } from '../ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card'
import { Input } from '../ui/input'
import type { CompanionProviderStatus } from '../../../../shared/types'

export function CompanionSetup({ providers }: { providers: CompanionProviderStatus[] }) {
  const rows = providers.length
    ? providers
    : [
        { id: 'opencode', label: 'opencode', command: 'opencode acp', status: 'missing' as const },
        {
          id: 'codex',
          label: 'Codex ACP',
          command: 'npx --no-install @zed-industries/codex-acp',
          status: 'missing' as const,
        },
      ]

  return (
    <Card size="sm" className="m-3 bg-background">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Bot className="size-4" aria-hidden />
          Connect a local companion
        </CardTitle>
        <CardDescription>
          Mdow runs a local ACP provider as a subprocess. Custom commands run on this computer.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex flex-col gap-2">
          {rows.map((provider) => (
            <div key={provider.id} className="rounded-md border border-border-subtle p-2">
              <div className="font-medium">{provider.label}</div>
              <div className="text-xs text-muted-foreground">{provider.command}</div>
            </div>
          ))}
        </div>
        <Input name="companion-custom-command" aria-label="Custom ACP command" placeholder="Custom ACP command" />
        <Button type="button" variant="outline" size="sm">
          <RefreshCcw className="size-3" aria-hidden />
          Retry detection
        </Button>
      </CardContent>
    </Card>
  )
}
```

- [ ] **Step 4: Implement messages, composer, panel, and full-screen**

Create `CompanionMessages.tsx` using the copied AI Elements exports:

```tsx
import { Conversation, ConversationContent, ConversationScrollButton } from '../ai-elements/conversation'
import { Message, MessageContent, MessageResponse } from '../ai-elements/message'
import { Source, Sources, SourcesContent, SourcesTrigger } from '../ai-elements/sources'
import type { CompanionMessage } from '../../../../shared/types'

export function CompanionMessages({ messages }: { messages: CompanionMessage[] }) {
  return (
    <Conversation className="min-h-0 flex-1">
      <ConversationContent>
        {messages.map((message) => (
          <div key={message.id}>
            {message.role === 'assistant' && message.citations.length > 0 && (
              <Sources>
                <SourcesTrigger count={message.citations.length} />
                <SourcesContent>
                  {message.citations.map((citation) => (
                    <Source key={citation.sourceId} href={`mdow-source:${citation.sourceId}`} title={citation.title} />
                  ))}
                </SourcesContent>
              </Sources>
            )}
            <Message from={message.role === 'system' ? 'assistant' : message.role}>
              <MessageContent>
                <MessageResponse>{message.content || (message.status === 'streaming' ? 'Thinking…' : '')}</MessageResponse>
              </MessageContent>
            </Message>
          </div>
        ))}
      </ConversationContent>
      <ConversationScrollButton />
    </Conversation>
  )
}
```

Create `CompanionComposer.tsx`:

```tsx
import { useState } from 'react'
import { PromptInput, PromptInputBody, PromptInputSubmit, PromptInputTextarea } from '../ai-elements/prompt-input'

export function CompanionComposer({
  streaming,
  onSend,
  onCancel,
}: {
  streaming: boolean
  onSend: (text: string) => Promise<void>
  onCancel: () => Promise<void>
}) {
  const [text, setText] = useState('')
  return (
    <PromptInput
      className="border-t border-border-subtle p-2"
      onSubmit={(message) => {
        if (streaming) {
          void onCancel()
          return
        }
        const value = message.text.trim()
        if (!value) return
        setText('')
        void onSend(value)
      }}
    >
      <PromptInputBody>
        <PromptInputTextarea
          aria-label="Ask about these docs"
          value={text}
          placeholder="Ask about these docs..."
          onChange={(event) => setText(event.currentTarget.value)}
        />
        <PromptInputSubmit status={streaming ? 'streaming' : 'ready'} disabled={!streaming && !text.trim()} />
      </PromptInputBody>
    </PromptInput>
  )
}
```

Create `CompanionPanel.tsx`:

```tsx
import { Maximize2, X } from 'lucide-react'
import { useAppStore } from '../../store/app-store'
import { useCompanionController } from '../../hooks/useCompanionController'
import { Button } from '../ui/button'
import { CompanionComposer } from './CompanionComposer'
import { CompanionMessages } from './CompanionMessages'
import { CompanionSetup } from './CompanionSetup'
import { CompanionStatus } from './CompanionStatus'

export function CompanionPanel() {
  const open = useAppStore((s) => s.companionOpen)
  const providers = useAppStore((s) => s.companionProviders)
  const messages = useAppStore((s) => s.companionMessages)
  const streaming = useAppStore((s) => s.companionStreaming)
  const setOpen = useAppStore((s) => s.setCompanionOpen)
  const setFullscreen = useAppStore((s) => s.setCompanionFullscreen)
  const { send, cancel } = useCompanionController()

  if (!open) return null

  const hasProvider = providers.some((provider) => provider.status === 'available')

  return (
    <aside aria-label="AI companion" className="hidden w-80 shrink-0 border-l border-border-subtle bg-background lg:flex lg:flex-col">
      <div className="flex items-center justify-between gap-2 border-b border-border-subtle px-3 py-2">
        <div className="min-w-0">
          <h2 className="truncate text-sm font-medium">AI companion</h2>
          <CompanionStatus providers={providers} />
        </div>
        <div className="flex items-center gap-1">
          <Button type="button" variant="ghost" size="icon-sm" aria-label="Expand companion" onClick={() => setFullscreen(true)}>
            <Maximize2 />
          </Button>
          <Button type="button" variant="ghost" size="icon-sm" aria-label="Close companion" onClick={() => setOpen(false)}>
            <X />
          </Button>
        </div>
      </div>
      {hasProvider ? <CompanionMessages messages={messages} /> : <CompanionSetup providers={providers} />}
      <CompanionComposer streaming={streaming} onSend={send} onCancel={cancel} />
    </aside>
  )
}
```

Create `CompanionFullscreen.tsx`:

```tsx
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog'
import { useAppStore } from '../../store/app-store'
import { useCompanionController } from '../../hooks/useCompanionController'
import { CompanionComposer } from './CompanionComposer'
import { CompanionMessages } from './CompanionMessages'

export function CompanionFullscreen() {
  const open = useAppStore((s) => s.companionFullscreen)
  const setOpen = useAppStore((s) => s.setCompanionFullscreen)
  const messages = useAppStore((s) => s.companionMessages)
  const streaming = useAppStore((s) => s.companionStreaming)
  const { send, cancel } = useCompanionController()

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="flex h-[min(46rem,calc(100vh-4rem))] max-w-[min(64rem,calc(100vw-2rem))] flex-col p-0" aria-label="AI companion">
        <DialogHeader className="border-b border-border-subtle px-4 py-3">
          <DialogTitle>AI companion</DialogTitle>
        </DialogHeader>
        <CompanionMessages messages={messages} />
        <CompanionComposer streaming={streaming} onSend={send} onCancel={cancel} />
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 5: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionPanel.test.tsx
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/renderer/src/components/companion
git commit -m "feat: add companion chat surfaces"
```

## Task 10: App And Settings Integration

**Files:**
- Modify: `apps/desktop/src/renderer/src/App.tsx`
- Modify: `apps/desktop/src/renderer/src/components/SettingsDialog.tsx`
- Modify: `apps/desktop/src/renderer/src/hooks/useAppBindings.ts`
- Modify: `apps/desktop/src/renderer/src/components/ShortcutsDialog.tsx` if the shortcut list is hard-coded

- [ ] **Step 1: Write integration tests**

Add tests to `CompanionPanel.test.tsx` or create `apps/desktop/src/renderer/src/components/companion/CompanionIntegration.test.tsx`:

```tsx
import { render, screen, fireEvent } from '@testing-library/react'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { useAppStore } from '../../store/app-store'
import { SettingsDialog } from '../SettingsDialog'

const saveSettings = vi.hoisted(() => vi.fn(async () => {}))

beforeEach(() => {
  Object.defineProperty(window, 'api', {
    configurable: true,
    value: {
      saveCompanionSettings: saveSettings,
      setTheme: vi.fn(),
      saveAppState: vi.fn(),
      setAutoUpdateScheduling: vi.fn(),
    },
  })
})

describe('companion settings integration', () => {
  it('renders companion provider settings', () => {
    render(<SettingsDialog open onOpenChange={() => {}} />)

    expect(screen.getByText('Companion')).toBeInTheDocument()
    expect(screen.getByLabelText('Custom ACP command')).toBeInTheDocument()
  })

  it('persists custom companion command from settings', () => {
    render(<SettingsDialog open onOpenChange={() => {}} />)

    fireEvent.change(screen.getByLabelText('Custom ACP command'), {
      target: { value: 'custom acp' },
    })

    expect(useAppStore.getState().companionCustomCommand).toBe('custom acp')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionIntegration.test.tsx
```

Expected: FAIL because settings do not render companion controls.

- [ ] **Step 3: Render companion surfaces in App**

Modify `apps/desktop/src/renderer/src/App.tsx` imports:

```ts
import { CompanionPanel } from './components/companion/CompanionPanel'
import { CompanionFullscreen } from './components/companion/CompanionFullscreen'
```

Render the right panel as a sibling after `<main>` and render full-screen near dialogs:

```tsx
          <main aria-label="Document" className="flex flex-1 flex-col overflow-hidden">
            <TabBar />
            {activeTab && <DocumentBreadcrumb tab={activeTab} />}
            <MainContent activeTab={activeTab} />
            <UpdateBanner />
          </main>
          <CompanionPanel />
          <CommandPalette />
          <ShortcutsDialog open={shortcutsDialogOpen} onOpenChange={setShortcutsDialogOpen} />
          <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
          <CompanionFullscreen />
```

- [ ] **Step 4: Add companion settings controls**

Modify `SettingsDialog.tsx` to read and write companion state:

```ts
  const companionProvider = useAppStore((s) => s.companionProvider)
  const companionCustomCommand = useAppStore((s) => s.companionCustomCommand)
  const setCompanionProvider = useAppStore((s) => s.setCompanionProvider)
  const setCompanionCustomCommand = useAppStore((s) => s.setCompanionCustomCommand)
```

Add this section before the Updates section:

```tsx
        <section className="space-y-2">
          <h3 className="text-sm font-medium">Companion</h3>
          <Field label="Provider">
            <PresetToggleGroup
              groupLabel="Companion provider"
              value={companionProvider}
              options={[
                { value: 'auto', label: 'Auto' },
                { value: 'opencode', label: 'opencode' },
                { value: 'codex', label: 'Codex' },
              ]}
              onChange={(value) => {
                setCompanionProvider(value)
                void window.api.saveCompanionSettings({ provider: value, customCommand: companionCustomCommand })
              }}
            />
          </Field>
          <Input
            name="companion-custom-command"
            aria-label="Custom ACP command"
            value={companionCustomCommand}
            placeholder="Custom ACP command"
            onChange={(event) => {
              const customCommand = event.currentTarget.value
              setCompanionCustomCommand(customCommand)
              void window.api.saveCompanionSettings({ provider: companionProvider, customCommand })
            }}
          />
          <p className="text-xs/relaxed text-muted-foreground">
            Custom commands run as local subprocesses and should point to an ACP-compatible agent.
          </p>
        </section>
```

Add `Input` import if missing:

```ts
import { Input } from './ui/input'
```

- [ ] **Step 5: Add an open button or shortcut**

Add a compact way to open the companion. The smallest first version is a keyboard shortcut in `useAppBindings.ts`: `CmdOrCtrl+Shift+K` toggles the companion panel.

Modify selectors in `useAppKeyboardShortcuts()`:

```ts
  const setCompanionOpen = useAppStore((s) => s.setCompanionOpen)
```

Add inside `onKeyDown` before the `Cmd+K` branch:

```ts
    if (mod && e.shiftKey && e.key.toLowerCase() === 'k') {
      e.preventDefault()
      const nextOpen = !useAppStore.getState().companionOpen
      setCompanionOpen(nextOpen)
      return
    }
```

- [ ] **Step 6: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionIntegration.test.tsx
```

Expected: PASS.

Run:

```bash
pnpm run --filter desktop typecheck:web
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/renderer/src/App.tsx apps/desktop/src/renderer/src/components/SettingsDialog.tsx apps/desktop/src/renderer/src/hooks/useAppBindings.ts apps/desktop/src/renderer/src/components/companion/CompanionIntegration.test.tsx
git commit -m "feat: integrate companion into app shell"
```

## Task 11: Citation Clicks And Document Navigation

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/companion/CompanionMessages.tsx`
- Modify: `apps/desktop/src/renderer/src/hooks/useOpenMarkdownFile.ts` only if a reusable return value is needed
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionCitations.test.tsx`

- [ ] **Step 1: Write failing citation click test**

Create `apps/desktop/src/renderer/src/components/companion/CompanionCitations.test.tsx`:

```tsx
import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { CompanionMessages } from './CompanionMessages'

const openMarkdownFile = vi.hoisted(() => vi.fn(async () => {}))

vi.mock('../../hooks/useOpenMarkdownFile', () => ({
  useOpenMarkdownFile: () => openMarkdownFile,
}))

describe('companion citations', () => {
  it('opens cited markdown files from source chips', () => {
    render(
      <CompanionMessages
        messages={[
          {
            id: 'a1',
            role: 'assistant',
            content: 'See the docs.',
            status: 'complete',
            createdAt: 1,
            citations: [{ sourceId: 'src_active', title: 'README.md', path: '/docs/README.md' }],
          },
        ]}
      />,
    )

    fireEvent.click(screen.getByText('README.md'))

    expect(openMarkdownFile).toHaveBeenCalledWith('/docs/README.md')
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionCitations.test.tsx
```

Expected: FAIL because citation chips do not open documents.

- [ ] **Step 3: Make source chips open files**

Modify `CompanionMessages.tsx` to render citation buttons using `useOpenMarkdownFile` instead of inert `Source` anchors:

```tsx
import { Button } from '../ui/button'
import { useOpenMarkdownFile } from '../../hooks/useOpenMarkdownFile'
```

Inside `CompanionMessages`:

```tsx
  const openMarkdownFile = useOpenMarkdownFile()
```

Replace each `<Source ... />` with:

```tsx
<Button
  key={citation.sourceId}
  type="button"
  variant="outline"
  size="xs"
  onClick={() => void openMarkdownFile(citation.path)}
>
  {citation.title}
</Button>
```

Keep `Sources`, `SourcesTrigger`, and `SourcesContent` from AI Elements for disclosure structure.

- [ ] **Step 4: Run tests and commit**

Run:

```bash
pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionCitations.test.tsx
```

Expected: PASS.

Commit:

```bash
git add apps/desktop/src/renderer/src/components/companion/CompanionMessages.tsx apps/desktop/src/renderer/src/components/companion/CompanionCitations.test.tsx
git commit -m "feat: open companion citation sources"
```

## Task 12: Final Verification And Polish

**Files:**
- Review all files touched by Tasks 1-11.

- [ ] **Step 1: Run focused companion tests**

Run:

```bash
pnpm run --filter desktop test -- -t companion
```

Expected: PASS.

- [ ] **Step 2: Run full desktop tests**

Run:

```bash
pnpm run --filter desktop test
```

Expected: PASS.

- [ ] **Step 3: Run typechecks**

Run:

```bash
pnpm run --filter desktop typecheck
```

Expected: PASS.

- [ ] **Step 4: Run lint and format check**

Run:

```bash
pnpm run --filter desktop lint
```

Expected: PASS.

Run:

```bash
pnpm run --filter desktop fmt:check
```

Expected: PASS.

- [ ] **Step 5: Manual smoke checks**

Run the app:

```bash
pnpm run --filter desktop dev
```

Manual checks:

- With no provider installed, open the companion with `CmdOrCtrl+Shift+K` and confirm the setup card shows opencode, Codex ACP, custom command, retry, and the local subprocess warning.
- With a markdown file open, confirm the context line references the active file after sending a message.
- With the left sidebar in Folder mode, open and close the right companion and confirm the left sidebar remains in Folder mode.
- Expand to full-screen and return to the panel; confirm the same messages are visible.
- Start a prompt against a real provider if installed; confirm deltas stream and cancel stops the turn.
- Click a citation chip and confirm Mdow opens the cited markdown file.

- [ ] **Step 6: Commit final fixes**

If any verification command changed files through formatting or small fixes, commit them:

```bash
git add apps/desktop docs/superpowers/plans/2026-06-04-ai-companion-chat.md
git commit -m "chore: polish companion chat"
```

If no files changed, do not create an empty commit.
