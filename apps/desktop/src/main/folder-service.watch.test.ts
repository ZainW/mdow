import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Dirent } from 'node:fs'

const eventCallbacks = vi.hoisted(() => new Map<string, Array<(path: string) => void>>())
const mockWatch = vi.hoisted(() => vi.fn())
const mockReaddir = vi.hoisted(() => vi.fn())

vi.mock('electron', () => ({
  dialog: { showOpenDialog: vi.fn() },
  BrowserWindow: vi.fn(),
}))

vi.mock('chokidar', () => ({
  watch: mockWatch,
}))

vi.mock('fs/promises', () => ({
  readdir: mockReaddir,
}))

import { unwatchFolder, watchFolder } from './folder-service'

function file(name: string): Dirent {
  return { name, isDirectory: () => false } as Dirent
}

function createDeferred<T>(): {
  promise: Promise<T>
  resolve: (value: T) => void
} {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((res) => {
    resolve = res
  })
  return { promise, resolve }
}

function createMockWatcher() {
  return {
    on: vi.fn((event: string, cb: (path: string) => void) => {
      const callbacks = eventCallbacks.get(event) ?? []
      callbacks.push(cb)
      eventCallbacks.set(event, callbacks)
    }),
    close: vi.fn(),
  }
}

describe('watchFolder', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    eventCallbacks.clear()
    mockWatch.mockReset()
    mockReaddir.mockReset()
    mockWatch.mockImplementation(createMockWatcher)
  })

  afterEach(() => {
    unwatchFolder()
    vi.useRealTimers()
  })

  it('keeps newer rescan results when the initial scan resolves late', async () => {
    const initialScan = createDeferred<Dirent[]>()
    mockReaddir
      .mockReturnValueOnce(initialScan.promise)
      .mockResolvedValueOnce([file('old.md'), file('new.md')])

    const onChange = vi.fn()
    watchFolder('/root', onChange)

    eventCallbacks.get('add')!.forEach((cb) => cb('/root/new.md'))
    await vi.advanceTimersByTimeAsync(1000)

    expect(onChange).toHaveBeenCalledWith({
      tree: [
        { name: 'new.md', path: '/root/new.md', isDirectory: false },
        { name: 'old.md', path: '/root/old.md', isDirectory: false },
      ],
      truncated: false,
    })

    initialScan.resolve([file('old.md')])
    await vi.waitFor(() => expect(mockReaddir).toHaveBeenCalledTimes(2))

    mockReaddir.mockResolvedValueOnce([file('another.md'), file('new.md'), file('old.md')])
    eventCallbacks.get('add')!.forEach((cb) => cb('/root/another.md'))
    await vi.advanceTimersByTimeAsync(1000)

    expect(onChange).toHaveBeenLastCalledWith({
      tree: [
        { name: 'another.md', path: '/root/another.md', isDirectory: false },
        { name: 'new.md', path: '/root/new.md', isDirectory: false },
        { name: 'old.md', path: '/root/old.md', isDirectory: false },
      ],
      truncated: false,
    })
  })
})
