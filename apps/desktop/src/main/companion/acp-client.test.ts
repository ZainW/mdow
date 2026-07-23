import { EventEmitter } from 'events'
import { Readable, Writable } from 'stream'
import { describe, expect, it, vi } from 'vitest'
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

  respondTo(chunk: string): void {
    for (const line of chunk.split('\n').filter(Boolean)) {
      const msg = JSON.parse(line) as { id?: number; method?: string }
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
            result: { sessionId: 'sess_test' },
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
    const updates: { kind: string; text?: string }[] = []
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: (u) => updates.push(u),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })

    await client.start()
    await client.createSession('/tmp/docs')
    await client.prompt('What is this?')

    expect(client.getSessionId()).toBe('sess_test')
    expect(updates.some((u) => u.kind === 'delta' && u.text === 'Hello from agent')).toBe(true)
    await client.shutdown()
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
