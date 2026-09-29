import { watch, type FSWatcher } from 'node:fs'
import { basename, dirname, join, resolve } from 'node:path'

export const DEBOUNCE_MS = 150

/**
 * Watches each open file's parent folder rather than the file itself, so editors that save
 * by writing a temp file and renaming it over the original (atomic saves) keep reloading.
 * Changes coalesce per path for DEBOUNCE_MS.
 */
export class FileWatcher {
  private parents = new Map<string, { watcher: FSWatcher; files: Set<string> }>()
  private timers = new Map<string, ReturnType<typeof setTimeout>>()

  constructor(private onChange: (path: string) => void) {}

  sync(paths: Iterable<string>) {
    const wanted = new Map<string, Set<string>>()
    for (const path of paths) {
      const absolute = resolve(path)
      const parent = dirname(absolute)
      if (!wanted.has(parent)) wanted.set(parent, new Set())
      wanted.get(parent)!.add(basename(absolute))
    }
    for (const [parent, entry] of this.parents) {
      if (!wanted.has(parent)) {
        entry.watcher.close()
        this.parents.delete(parent)
      }
    }
    for (const [parent, files] of wanted) {
      const existing = this.parents.get(parent)
      if (existing) {
        existing.files = files
        continue
      }
      try {
        const watcher = watch(parent, { persistent: false }, (_event, name) => {
          const entry = this.parents.get(parent)
          if (!entry) return
          const changed = name ? [name] : [...entry.files]
          for (const file of changed) if (entry.files.has(file)) this.schedule(join(parent, file))
        })
        watcher.on('error', () => {})
        this.parents.set(parent, { watcher, files })
      } catch {
        // Unwatchable folders (removed, no permission) just don't live-reload.
      }
    }
  }

  private schedule(path: string) {
    clearTimeout(this.timers.get(path))
    this.timers.set(
      path,
      setTimeout(() => {
        this.timers.delete(path)
        this.onChange(path)
      }, DEBOUNCE_MS),
    )
  }

  close() {
    for (const entry of this.parents.values()) entry.watcher.close()
    for (const timer of this.timers.values()) clearTimeout(timer)
    this.parents.clear()
    this.timers.clear()
  }
}

/** Rescans a workspace folder when anything below it changes, coalesced. */
export class FolderWatcher {
  private watcher: FSWatcher | null = null
  private timer: ReturnType<typeof setTimeout> | null = null

  constructor(private onChange: () => void) {}

  watch(folder: string | null) {
    this.close()
    if (!folder) return
    try {
      this.watcher = watch(folder, { recursive: true, persistent: false }, (_event, name) => {
        if (name && /(^|\/)(\.git|node_modules|target|dist|build)(\/|$)/.test(name)) return
        if (this.timer) clearTimeout(this.timer)
        this.timer = setTimeout(() => {
          this.timer = null
          this.onChange()
        }, 300)
      })
      this.watcher.on('error', () => {})
    } catch {
      this.watcher = null
    }
  }

  close() {
    this.watcher?.close()
    this.watcher = null
    if (this.timer) clearTimeout(this.timer)
    this.timer = null
  }
}
