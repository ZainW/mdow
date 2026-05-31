import { afterEach, describe, expect, it, vi } from 'vitest'
import { createBoundedMemoryStorage } from './cache-storage'

describe('createBoundedMemoryStorage', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('evicts the least-recently-used entry when the entry limit is exceeded', () => {
    const storage = createBoundedMemoryStorage(2)

    void storage.set('a', 'A')
    void storage.set('b', 'B')
    expect(storage.get('a')).toBe('A')

    void storage.set('c', 'C')

    expect(storage.get('a')).toBe('A')
    expect(storage.get('b')).toBeNull()
    expect(storage.get('c')).toBe('C')
  })

  it('expires entries by ttl', () => {
    vi.useFakeTimers()
    const storage = createBoundedMemoryStorage(2)

    void storage.set('a', 'A', { ttl: 1 })

    expect(storage.get('a')).toBe('A')
    vi.advanceTimersByTime(1000)
    expect(storage.get('a')).toBeNull()
  })
})
