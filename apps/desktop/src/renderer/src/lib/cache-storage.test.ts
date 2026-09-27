import { describe, expect, it, vi } from 'vitest'
import { memoizeAsync } from './cache-storage'

describe('memoizeAsync', () => {
  it('returns the cached result for a repeated key', async () => {
    const fn = vi.fn((value: string) => Promise.resolve(value.toUpperCase()))
    const memoized = memoizeAsync(fn, { maxEntries: 2, getKey: (value) => value })

    await expect(memoized('a')).resolves.toBe('A')
    await expect(memoized('a')).resolves.toBe('A')
    expect(fn).toHaveBeenCalledTimes(1)
  })

  it('shares one pending call between concurrent callers', async () => {
    const fn = vi.fn((value: string) => Promise.resolve(value))
    const memoized = memoizeAsync(fn, { maxEntries: 2, getKey: (value) => value })

    await Promise.all([memoized('a'), memoized('a')])
    expect(fn).toHaveBeenCalledTimes(1)
  })

  it('evicts the least-recently-used entry when the entry limit is exceeded', async () => {
    const fn = vi.fn((value: string) => Promise.resolve(value))
    const memoized = memoizeAsync(fn, { maxEntries: 2, getKey: (value) => value })

    await memoized('a')
    await memoized('b')
    await memoized('a')
    await memoized('c')
    fn.mockClear()

    await memoized('a')
    await memoized('b')
    expect(fn).toHaveBeenCalledTimes(1)
    expect(fn).toHaveBeenCalledWith('b')
  })

  it('does not cache rejections', async () => {
    const fn = vi
      .fn<(value: string) => Promise<string>>()
      .mockRejectedValueOnce(new Error('boom'))
      .mockResolvedValue('ok')
    const memoized = memoizeAsync(fn, { maxEntries: 2, getKey: (value) => value })

    await expect(memoized('a')).rejects.toThrow('boom')
    await expect(memoized('a')).resolves.toBe('ok')
  })

  it('skips the cache when asked to', async () => {
    const fn = vi.fn((value: string, _bypass?: boolean) => Promise.resolve(value))
    const memoized = memoizeAsync<[string, boolean?], string>(fn, {
      maxEntries: 2,
      getKey: (value) => value,
      shouldBypassCache: (_value, bypass?: boolean) => bypass === true,
    })

    await memoized('a', true)
    await memoized('a', true)
    expect(fn).toHaveBeenCalledTimes(2)
  })
})
