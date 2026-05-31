import { setStorage, type StorageInterface } from 'ocache'

interface StorageEntry<T = unknown> {
  value: T
  expiresAt?: number
}

export function createBoundedMemoryStorage(maxEntries: number): StorageInterface {
  const entries = new Map<string, StorageEntry>()

  function evictOverflow(): void {
    while (entries.size > maxEntries) {
      const oldest = entries.keys().next().value
      if (oldest === undefined) return
      entries.delete(oldest)
    }
  }

  return {
    get<T = unknown>(key: string): T | null {
      const entry = entries.get(key)
      if (!entry) return null
      if (entry.expiresAt !== undefined && entry.expiresAt <= Date.now()) {
        entries.delete(key)
        return null
      }

      entries.delete(key)
      entries.set(key, entry)
      return entry.value as T
    },
    set<T = unknown>(key: string, value: T, opts?: { ttl?: number }): void {
      if (maxEntries <= 0) return

      entries.delete(key)
      entries.set(key, {
        value,
        expiresAt: opts?.ttl === undefined ? undefined : Date.now() + opts.ttl * 1000,
      })
      evictOverflow()
    },
  }
}

let rendererCacheStorageConfigured = false

export function configureRendererCacheStorage(): void {
  if (rendererCacheStorageConfigured) return
  setStorage(createBoundedMemoryStorage(200))
  rendererCacheStorageConfigured = true
}
