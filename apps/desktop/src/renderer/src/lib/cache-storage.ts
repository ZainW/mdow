interface MemoizeOptions<Args extends unknown[]> {
  /** Most entries kept; the least recently used entry is evicted first. */
  maxEntries: number
  getKey: (...args: Args) => string
  shouldBypassCache?: (...args: Args) => boolean
}

/**
 * Memoizes an async function in a bounded in-memory LRU. Concurrent calls with the same key
 * share one pending promise, and rejected calls are not cached.
 */
export function memoizeAsync<Args extends unknown[], Result>(
  fn: (...args: Args) => Promise<Result>,
  { maxEntries, getKey, shouldBypassCache }: MemoizeOptions<Args>,
): (...args: Args) => Promise<Result> {
  const entries = new Map<string, Promise<Result>>()

  return (...args: Args) => {
    if (shouldBypassCache?.(...args)) return fn(...args)

    const key = getKey(...args)
    const cached = entries.get(key)
    if (cached) {
      entries.delete(key)
      entries.set(key, cached)
      return cached
    }

    const promise = fn(...args)
    entries.set(key, promise)
    promise.catch(() => {
      if (entries.get(key) === promise) entries.delete(key)
    })
    while (entries.size > maxEntries) {
      const oldest = entries.keys().next().value
      if (oldest === undefined) break
      entries.delete(oldest)
    }
    return promise
  }
}
