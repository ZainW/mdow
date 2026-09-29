import { describe, expect, test } from 'bun:test'
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { decodeState, loadState, MAX_RECENTS, pushRecent, saveState } from './persist'
import { DEFAULT_PREFS } from './prefs'

const tmp = () => join(mkdtempSync(join(tmpdir(), 'mdow-persist-')), 'state.json')

describe('persistence', () => {
  test('round-trips prefs and session', () => {
    const path = tmp()
    const state = {
      prefs: { ...DEFAULT_PREFS, theme: 'dark' as const, zoomLevel: 130, wideMode: true },
      session: {
        recents: ['/a.md'],
        lastFolder: '/docs',
        tabs: ['/a.md', '/b.md'],
        activeTab: '/b.md',
        window: { width: 1000, height: 700 },
      },
    }
    saveState(path, state)
    expect(loadState(path)).toEqual(state)
  })

  test('reads the Rust and Electron wire keys', () => {
    const decoded = decodeState({
      theme: 'light',
      contentFont: 'charter',
      codeFont: 'jetbrains-mono',
      interfaceScale: 'large',
      readingWidth: 'wide',
      wideMode: false,
      zoomLevel: 90,
      sidebarMode: 'outline',
      recents: ['/x.md'],
      lastFolder: '/notes',
      sessionTabs: [{ path: '/x.md' }],
      sessionActiveTabPath: '/x.md',
      windowBounds: { x: 10, y: 20, width: 900, height: 600 },
    })
    expect(decoded.prefs).toMatchObject({ contentFont: 'charter', sidebarMode: 'outline' })
    expect(decoded.session).toMatchObject({
      activeTab: '/x.md',
      window: { width: 900, height: 600 },
    })
  })

  test('decodes each field on its own so one bad value keeps the rest', () => {
    const decoded = decodeState({
      theme: 'neon',
      zoomLevel: 'big',
      codeFont: 'sf-mono',
      recents: ['/a.md', 42, '/a.md'],
      sessionTabs: [{ path: '/t.md' }, { nope: 1 }],
      sessionActiveTabPath: '/closed.md',
      windowBounds: { width: 10, height: 10 },
    })
    expect(decoded.prefs.theme).toBe('system')
    expect(decoded.prefs.zoomLevel).toBe(100)
    expect(decoded.prefs.codeFont).toBe('sf-mono')
    expect(decoded.session.recents).toEqual(['/a.md'])
    expect(decoded.session.activeTab).toBe('/t.md')
    expect(decoded.session.window).toBeNull()
  })

  test('a corrupt file falls back to defaults', () => {
    const path = tmp()
    writeFileSync(path, '{not json')
    expect(loadState(path).prefs).toEqual(DEFAULT_PREFS)
  })

  test('writes atomically with camelCase keys', () => {
    const path = tmp()
    saveState(path, loadState('/nonexistent'))
    const raw = JSON.parse(readFileSync(path, 'utf8'))
    expect(Object.keys(raw)).toContain('sessionActiveTabPath')
  })

  test('recents move to the front and cap at the limit', () => {
    let recents: string[] = []
    for (let i = 0; i < MAX_RECENTS + 5; i++) recents = pushRecent(recents, `/${i}.md`)
    recents = pushRecent(recents, '/10.md')
    expect(recents).toHaveLength(MAX_RECENTS)
    expect(recents[0]).toBe('/10.md')
  })
})
