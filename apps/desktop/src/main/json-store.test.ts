import { mkdtempSync, readFileSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { describe, expect, it, vi } from 'vitest'

vi.mock('electron', () => ({ app: { getPath: () => tmpdir() } }))

import { JsonStore } from './json-store'

interface Settings {
  theme: string
  zoom: number
}

const defaults: Settings = { theme: 'system', zoom: 100 }

function tempFile(): string {
  return join(mkdtempSync(join(tmpdir(), 'mdow-store-')), 'config.json')
}

describe('JsonStore', () => {
  it('starts from defaults when the file is missing', () => {
    const store = new JsonStore<Settings>({ defaults, path: tempFile() })
    expect(store.get('theme')).toBe('system')
    expect(store.get('zoom')).toBe(100)
  })

  it('reads settings written by electron-store and keeps unknown keys', () => {
    const path = tempFile()
    writeFileSync(path, JSON.stringify({ theme: 'dark', legacy: true }, undefined, '\t'))

    const store = new JsonStore<Settings>({ defaults, path })
    store.set('zoom', 120)

    expect(store.get('theme')).toBe('dark')
    expect(JSON.parse(readFileSync(path, 'utf8'))).toEqual({
      theme: 'dark',
      zoom: 120,
      legacy: true,
    })
  })

  it('persists across instances in the same tab-indented format', () => {
    const path = tempFile()
    new JsonStore<Settings>({ defaults, path }).set('theme', 'light')

    expect(new JsonStore<Settings>({ defaults, path }).get('theme')).toBe('light')
    expect(readFileSync(path, 'utf8')).toContain('\t"theme": "light"')
  })

  it('falls back to defaults when the file is corrupt', () => {
    const path = tempFile()
    writeFileSync(path, '{not json')
    expect(new JsonStore<Settings>({ defaults, path }).get('zoom')).toBe(100)
  })
})
