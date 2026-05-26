import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const changeCallbacks = vi.hoisted(() => new Map<string, Array<() => void>>())
const unlinkCallbacks = vi.hoisted(() => new Map<string, Array<() => void>>())
const mockWatch = vi.hoisted(() => vi.fn())
const mockReadFile = vi.hoisted(() => vi.fn())

vi.mock('electron', () => ({
  dialog: { showOpenDialog: vi.fn() },
  BrowserWindow: vi.fn(),
}))

vi.mock('chokidar', () => ({
  watch: mockWatch,
}))

vi.mock('fs/promises', () => ({
  readFile: mockReadFile,
}))

import { watchFile, unwatchFile, unwatchAllFiles } from './file-service'

function createMockWatcher(path: string) {
  const changeCbs: Array<() => void> = []
  const unlinkCbs: Array<() => void> = []
  changeCallbacks.set(path, changeCbs)
  unlinkCallbacks.set(path, unlinkCbs)
  return {
    on: vi.fn((event: string, cb: () => void) => {
      if (event === 'change') changeCbs.push(cb)
      if (event === 'unlink') unlinkCbs.push(cb)
    }),
    close: vi.fn(),
  }
}

describe('file-service watchers', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    changeCallbacks.clear()
    unlinkCallbacks.clear()
    mockWatch.mockReset()
    mockReadFile.mockReset()
    mockWatch.mockImplementation((path: string) => createMockWatcher(path))
  })

  afterEach(() => {
    unwatchAllFiles()
    vi.useRealTimers()
  })

  it('debounces change events by 300ms', async () => {
    const onChange = vi.fn()
    watchFile('/docs/readme.md', onChange)

    const changeCbs = changeCallbacks.get('/docs/readme.md')!
    changeCbs.forEach((cb) => cb())
    changeCbs.forEach((cb) => cb())

    expect(mockReadFile).not.toHaveBeenCalled()
    expect(onChange).not.toHaveBeenCalled()

    mockReadFile.mockResolvedValue('updated')
    await vi.advanceTimersByTimeAsync(300)

    expect(mockReadFile).toHaveBeenCalledOnce()
    expect(onChange).toHaveBeenCalledOnce()
    expect(onChange).toHaveBeenCalledWith({ type: 'changed', content: 'updated' })
  })

  it('unwatch clears pending debounced callbacks', async () => {
    const onChange = vi.fn()
    watchFile('/docs/readme.md', onChange)

    const changeCbs = changeCallbacks.get('/docs/readme.md')!
    changeCbs.forEach((cb) => cb())

    unwatchFile('/docs/readme.md')

    mockReadFile.mockResolvedValue('updated')
    await vi.advanceTimersByTimeAsync(300)

    expect(onChange).not.toHaveBeenCalled()
  })

  it('unwatch closes the watcher', () => {
    const onChange = vi.fn()
    watchFile('/docs/readme.md', onChange)
    const watcher = mockWatch.mock.results[0].value as { close: ReturnType<typeof vi.fn> }

    unwatchFile('/docs/readme.md')

    expect(watcher.close).toHaveBeenCalledOnce()
  })

  it('reuses an existing watcher and swaps the callback', async () => {
    const onChange1 = vi.fn()
    const onChange2 = vi.fn()

    watchFile('/docs/readme.md', onChange1)
    watchFile('/docs/readme.md', onChange2)

    expect(mockWatch).toHaveBeenCalledOnce()

    const changeCbs = changeCallbacks.get('/docs/readme.md')!
    changeCbs.forEach((cb) => cb())
    mockReadFile.mockResolvedValue('updated')
    await vi.advanceTimersByTimeAsync(300)

    expect(onChange1).not.toHaveBeenCalled()
    expect(onChange2).toHaveBeenCalledWith({ type: 'changed', content: 'updated' })
  })

  it('emits deleted events immediately on unlink', () => {
    const onChange = vi.fn()
    watchFile('/docs/readme.md', onChange)

    const unlinkCbs = unlinkCallbacks.get('/docs/readme.md')!
    unlinkCbs.forEach((cb) => cb())

    expect(onChange).toHaveBeenCalledWith({ type: 'deleted' })
  })
})
