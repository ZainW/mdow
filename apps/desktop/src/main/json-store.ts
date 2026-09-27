import { app } from 'electron'
import { mkdirSync, readFileSync, renameSync, writeFileSync } from 'fs'
import { dirname, join } from 'path'

/**
 * A minimal synchronous JSON settings store. It reads and writes the same file electron-store
 * used (`<userData>/config.json`, tab-indented), so existing settings carry over, without the
 * schema-validation machinery that cost ~20ms on every launch.
 */
export class JsonStore<T extends object> {
  private readonly path: string
  private data: T

  constructor({ defaults, path }: { defaults: T; path?: string }) {
    this.path = path ?? join(app.getPath('userData'), 'config.json')
    this.data = { ...defaults, ...this.read() }
  }

  get<K extends keyof T>(key: K): T[K] {
    return this.data[key]
  }

  set<K extends keyof T>(key: K, value: T[K]): void {
    this.data = { ...this.data, [key]: value }
    this.write()
  }

  private read(): Partial<T> {
    try {
      const parsed: unknown = JSON.parse(readFileSync(this.path, 'utf8'))
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- settings file written by this store; fields are validated where they are read.
        return parsed as Partial<T>
      }
    } catch {
      // Missing or corrupt file: start from defaults. The next write replaces it.
    }
    return {}
  }

  private write(): void {
    // Write-then-rename so a crash mid-write never leaves a truncated settings file.
    const temp = `${this.path}.tmp`
    try {
      mkdirSync(dirname(this.path), { recursive: true })
      writeFileSync(temp, JSON.stringify(this.data, undefined, '\t'))
      renameSync(temp, this.path)
    } catch (error) {
      console.error('Failed to save settings', error)
    }
  }
}
