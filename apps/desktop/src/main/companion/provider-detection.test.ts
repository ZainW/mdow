import { EventEmitter } from 'node:events'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const spawnMock = vi.hoisted(() => vi.fn())

vi.mock('node:child_process', () => ({ spawn: spawnMock }))

import { detectCompanionProviders, parseProviderCommand } from './provider-detection'

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
