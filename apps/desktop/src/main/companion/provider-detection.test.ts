import { EventEmitter } from 'node:events'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const spawnMock = vi.hoisted(() => vi.fn())

vi.mock('node:child_process', () => ({ spawn: spawnMock }))

import { detectCompanionProviders, parseProviderCommand } from './provider-detection'

type MockProcess = EventEmitter & { kill: ReturnType<typeof vi.fn> }

function mockProcess({
  exitCode,
  delay = 0,
  onKill,
}: {
  exitCode?: number
  delay?: number
  onKill?: (proc: MockProcess, signal?: NodeJS.Signals | number) => void
}) {
  const proc = new EventEmitter() as MockProcess
  proc.kill = vi.fn((signal?: NodeJS.Signals | number) => {
    if (onKill) {
      onKill(proc, signal)
    } else {
      proc.emit('exit', null, signal ?? 'SIGTERM')
    }
    return true
  })
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

  it('waits for a timed-out provider to exit after sending SIGTERM', async () => {
    const opencodeProcess = mockProcess({
      onKill: (proc, signal) => {
        if (signal === 'SIGTERM') setTimeout(() => proc.emit('exit', null, 'SIGTERM'), 50)
      },
    })
    spawnMock.mockImplementation((command: string) => {
      if (command === 'opencode') return opencodeProcess
      return mockProcess({ exitCode: 1 })
    })

    let settled = false
    const promise = detectCompanionProviders('').then((statuses) => {
      settled = true
      return statuses
    })

    await vi.advanceTimersByTimeAsync(750)

    expect(opencodeProcess.kill).toHaveBeenCalledWith('SIGTERM')
    expect(settled).toBe(false)

    await vi.advanceTimersByTimeAsync(50)
    const statuses = await promise

    expect(statuses.find((s) => s.id === 'opencode')?.status).toBe('available')
  })

  it('escalates timed-out providers to SIGKILL when SIGTERM is ignored', async () => {
    const opencodeProcess = mockProcess({
      onKill: (proc, signal) => {
        if (signal === 'SIGKILL') proc.emit('exit', null, 'SIGKILL')
      },
    })
    spawnMock.mockImplementation((command: string) => {
      if (command === 'opencode') return opencodeProcess
      return mockProcess({ exitCode: 1 })
    })

    const promise = detectCompanionProviders('')

    await vi.advanceTimersByTimeAsync(1_000)
    const statuses = await promise

    expect(opencodeProcess.kill).toHaveBeenCalledWith('SIGTERM')
    expect(opencodeProcess.kill).toHaveBeenCalledWith('SIGKILL')
    expect(statuses.find((s) => s.id === 'opencode')?.status).toBe('available')
  })
})
