import { EventEmitter } from 'events'
import { PassThrough } from 'stream'
import { describe, expect, it, vi } from 'vitest'
import {
  isSupportedOpencodeVersion,
  opencodeSearchDirs,
  parseOpencodeVersion,
} from './opencode-binary'
import { parseListeningUrl, startOpencodeServer } from './opencode-server'

function fakeChild() {
  const child = new EventEmitter() as EventEmitter & {
    stdout: PassThrough
    stderr: PassThrough
    exitCode: number | null
    killed: boolean
    kill: () => void
  }
  child.stdout = new PassThrough()
  child.stderr = new PassThrough()
  child.exitCode = null
  child.killed = false
  child.kill = vi.fn(() => {
    child.killed = true
    child.emit('exit', null)
  })
  return child
}

describe('startOpencodeServer', () => {
  it('resolves with the listening URL and a private password', async () => {
    const child = fakeChild()
    const spawnImpl = vi.fn(() => child)
    const pending = startOpencodeServer('/bin/opencode', spawnImpl as never)
    child.stdout.write('Server listening on http://127.0.0.1:55576\n')
    const server = await pending

    expect(server.url).toBe('http://127.0.0.1:55576')
    expect(server.password).toMatch(/^[0-9a-f]{48}$/)
    const [, args, options] = spawnImpl.mock.calls[0] as unknown as [
      string,
      string[],
      { env: Record<string, string> },
    ]
    expect(args).toEqual(['serve', '--hostname=127.0.0.1', '--port=0'])
    expect(options.env.OPENCODE_SERVER_PASSWORD).toBe(server.password)

    server.stop()
    await server.exited
    expect(child.kill).toHaveBeenCalled()
  })

  it('rejects with the tail of the output when the server exits early', async () => {
    const child = fakeChild()
    const pending = startOpencodeServer('/bin/opencode', (() => child) as never)
    child.stderr.write('error: port in use\n')
    child.emit('exit', 1)
    await expect(pending).rejects.toThrow('OpenCode exited (1): error: port in use')
  })
})

describe('parseListeningUrl', () => {
  it('finds the URL in server output', () => {
    expect(parseListeningUrl('boot\nServer listening on http://127.0.0.1:4096\n')).toBe(
      'http://127.0.0.1:4096',
    )
    expect(parseListeningUrl('starting…')).toBeNull()
  })
})

describe('opencode binary helpers', () => {
  it('requires OpenCode 2 or newer', () => {
    expect(parseOpencodeVersion('opencode v2.0.16\n')).toBe('2.0.16')
    expect(isSupportedOpencodeVersion('2.0.16')).toBe(true)
    expect(isSupportedOpencodeVersion('1.18.34')).toBe(false)
  })

  it('searches the install folder even when PATH is minimal', () => {
    const dirs = opencodeSearchDirs({ PATH: '/usr/bin' }, '/Users/me')
    expect(dirs[0]).toBe('/usr/bin')
    expect(dirs).toContain('/Users/me/.opencode/bin')
    expect(new Set(dirs).size).toBe(dirs.length)
  })
})
