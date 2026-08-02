import { EventEmitter } from 'events'
import { Readable, Writable } from 'stream'
import { describe, expect, it, vi } from 'vitest'
import packageJson from '../../../package.json' with { type: 'json' }
import type { CompanionUpdate } from '../../shared/types'
import { AcpClient } from './acp-client'

class FakeChild extends EventEmitter {
  stdin = new Writable({
    write: (chunk, _enc, cb) => {
      this.written.push(String(chunk))
      queueMicrotask(() => this.respondTo(String(chunk)))
      cb()
    },
  })
  stdout = new Readable({ read() {} })
  stderr = new Readable({ read() {} })
  written: string[] = []
  private requestCount = 0
  ignoredMethods = new Set<string>()

  private modelConfig(currentValue = 'opencode/claude-sonnet-4-5') {
    return [
      {
        id: 'model',
        name: 'Model',
        category: 'model',
        type: 'select',
        currentValue,
        options: [
          { value: 'openai/gpt-5.4', name: 'GPT-5.4' },
          { value: 'opencode/claude-sonnet-4-5', name: 'Claude Sonnet 4.5' },
          { value: 'opencode-go/kimi-k2.5', name: 'Kimi K2.5' },
          { value: 'anthropic/claude-opus-4', name: 'Hidden direct provider' },
        ],
      },
    ]
  }

  respondTo(chunk: string): void {
    for (const line of chunk.split('\n').filter(Boolean)) {
      const msg = JSON.parse(line) as {
        id?: number
        method?: string
        params?: Record<string, unknown>
      }
      if (msg.method && this.ignoredMethods.has(msg.method)) continue
      if (msg.method === 'initialize' && msg.id !== undefined) {
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            id: msg.id,
            result: { protocolVersion: 1, agentCapabilities: {} },
          })}\n`,
        )
      } else if (msg.method === 'session/new' && msg.id !== undefined) {
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            id: msg.id,
            result: { sessionId: 'sess_test', configOptions: this.modelConfig() },
          })}\n`,
        )
      } else if (msg.method === 'session/set_config_option' && msg.id !== undefined) {
        const value = msg.params?.value
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            id: msg.id,
            result: {
              configOptions: this.modelConfig(typeof value === 'string' ? value : ''),
            },
          })}\n`,
        )
      } else if (msg.method === 'session/prompt' && msg.id !== undefined) {
        this.requestCount += 1
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: {
              sessionId: 'sess_test',
              update: {
                sessionUpdate: 'agent_thought_chunk',
                content: { type: 'text', text: 'Thinking…' },
              },
            },
          })}\n`,
        )
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: {
              sessionId: 'sess_test',
              update: {
                sessionUpdate: 'tool_call',
                toolCallId: 'tool_1',
                title: 'read',
                status: 'completed',
                rawInput: { path: 'a.md' },
                rawOutput: { result: 'ok' },
              },
            },
          })}\n`,
        )
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: {
              sessionId: 'sess_test',
              update: {
                sessionUpdate: 'agent_message_chunk',
                content: { type: 'text', text: 'Hello from agent' },
              },
            },
          })}\n`,
        )
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            id: msg.id,
            result: { stopReason: 'end_turn' },
          })}\n`,
        )
      } else if (msg.method === 'fs/write_text_file' && msg.id !== undefined) {
        // ignored; client handles incoming only
      }
    }
  }

  kill(): void {
    this.emit('exit', 0)
  }
}

describe('Companion ACP client', () => {
  it('initializes, creates a session, and streams prompt deltas', async () => {
    const updates: CompanionUpdate[] = []
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: (u) => updates.push(u),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })

    await client.start()
    const writtenMessages = fake.written
      .flatMap((chunk) => chunk.split('\n'))
      .filter(Boolean)
      .map(
        (line) =>
          JSON.parse(line) as {
            method?: string
            params?: {
              clientInfo?: { version?: string }
              clientCapabilities?: { fs?: { readTextFile?: boolean } }
            }
          },
      )
    const initialize = writtenMessages.find((message) => message.method === 'initialize')
    expect(initialize?.params?.clientInfo?.version).toBe(packageJson.version)
    expect(initialize?.params?.clientCapabilities?.fs?.readTextFile).toBe(false)
    expect(writtenMessages.map((message) => message.method)).not.toContain(
      'notifications/initialized',
    )

    await client.createSession('/tmp/docs')
    await client.prompt('What is this?')

    expect(client.getSessionId()).toBe('sess_test')
    expect(updates.some((u) => u.kind === 'thinking' && u.text === 'Thinking…')).toBe(true)
    expect(updates.some((u) => u.kind === 'thinking-done')).toBe(true)
    expect(
      updates.some(
        (u) =>
          u.kind === 'tool' &&
          u.toolCallId === 'tool_1' &&
          u.name === 'read' &&
          u.state === 'completed' &&
          u.output === '{\n  "result": "ok"\n}',
      ),
    ).toBe(true)
    expect(updates.some((u) => u.kind === 'delta' && u.text === 'Hello from agent')).toBe(true)
    await client.shutdown()
  })

  it('passes optional read-only MCP servers into session setup', async () => {
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: vi.fn(),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()

    await client.createSession('/tmp/docs', [
      {
        name: 'fff',
        command: '/opt/homebrew/bin/fff-mcp',
        args: [],
        env: [],
      },
    ])

    const sessionRequest = fake.written
      .flatMap((chunk) => chunk.split('\n'))
      .filter(Boolean)
      .map((line) => JSON.parse(line) as { method?: string; params?: Record<string, unknown> })
      .find((message) => message.method === 'session/new')
    expect(sessionRequest?.params?.mcpServers).toEqual([
      {
        name: 'fff',
        command: '/opt/homebrew/bin/fff-mcp',
        args: [],
        env: [],
      },
    ])
    await client.shutdown()
  })

  it('reads and changes the live model configuration exposed by the agent', async () => {
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: vi.fn(),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()
    await client.createSession('/tmp/docs')

    expect(client.getModelState()).toEqual({
      options: [
        {
          value: 'openai/gpt-5.4',
          name: 'GPT-5.4',
          provider: 'openai',
        },
        {
          value: 'opencode/claude-sonnet-4-5',
          name: 'Claude Sonnet 4.5',
          provider: 'opencode',
        },
        {
          value: 'opencode-go/kimi-k2.5',
          name: 'Kimi K2.5',
          provider: 'opencode-go',
        },
      ],
      currentValue: 'opencode/claude-sonnet-4-5',
      stale: false,
    })

    await expect(client.setModel('openai/gpt-5.4')).resolves.toMatchObject({
      currentValue: 'openai/gpt-5.4',
    })
    const setRequest = fake.written
      .flatMap((chunk) => chunk.split('\n'))
      .filter(Boolean)
      .map((line) => JSON.parse(line) as { method?: string; params?: Record<string, unknown> })
      .find((message) => message.method === 'session/set_config_option')
    expect(setRequest?.params).toMatchObject({
      sessionId: 'sess_test',
      configId: 'model',
      value: 'openai/gpt-5.4',
    })
    await client.shutdown()
  })

  it('rejects a model value that is not live in the current session', async () => {
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: vi.fn(),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()
    await client.createSession('/tmp/docs')

    await expect(client.setModel('anthropic/claude-opus-4')).rejects.toThrow(/not available/i)
    await client.shutdown()
  })

  it('does not surface subprocess diagnostics as conversation status', async () => {
    const updates: CompanionUpdate[] = []
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: (update) => updates.push(update),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })

    await client.start()
    fake.stderr.push('agent diagnostic output\n')
    await new Promise((resolve) => setImmediate(resolve))

    expect(updates).not.toContainEqual({
      kind: 'status',
      message: 'agent diagnostic output',
    })
    await client.shutdown()
  })

  it('times out unanswered ACP requests and clears them', async () => {
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: vi.fn(),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()
    fake.ignoredMethods.add('session/new')
    vi.useFakeTimers()

    let rejection: Error | undefined
    const pending = client.createSession('/tmp/docs').catch((error: unknown) => {
      rejection = error instanceof Error ? error : new Error(String(error))
    })
    try {
      await vi.advanceTimersByTimeAsync(30_001)
      expect(rejection?.message).toMatch(/session\/new timed out/i)
    } finally {
      await client.shutdown()
      await pending
      vi.useRealTimers()
    }
  })

  it('does not apply the setup timeout to a long-running prompt', async () => {
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: vi.fn(),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()
    await client.createSession('/tmp/docs')
    fake.ignoredMethods.add('session/prompt')
    vi.useFakeTimers()

    let settled = false
    const pending = client.prompt('Take your time').finally(() => {
      settled = true
    })
    try {
      await vi.advanceTimersByTimeAsync(30_001)
      expect(settled).toBe(false)
    } finally {
      await client.shutdown()
      await expect(pending).rejects.toThrow(/shut down/i)
      vi.useRealTimers()
    }
  })

  it('refuses write and terminal requests from the agent', async () => {
    const updates: { kind: string; message?: string }[] = []
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: (u) => updates.push(u),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()

    fake.stdout.push(
      `${JSON.stringify({
        jsonrpc: '2.0',
        id: 99,
        method: 'fs/write_text_file',
        params: { path: '/tmp/x.md', content: 'nope' },
      })}\n`,
    )

    await vi.waitFor(() => {
      expect(updates.some((u) => u.kind === 'warning' && u.message?.includes('fs/write'))).toBe(
        true,
      )
    })

    const reply = fake.written.find((line) => line.includes('"id":99'))
    expect(reply).toBeTruthy()
    expect(reply).toContain('Refused')
    await client.shutdown()
  })
})
