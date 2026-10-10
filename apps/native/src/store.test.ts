import { beforeEach, describe, expect, test } from 'bun:test'
import { mkdtempSync, realpathSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { EMPTY_SESSION } from './lib/persist'
import { DEFAULT_PREFS } from './lib/prefs'
import {
  appStore,
  closeTab,
  createAppStore,
  cycleTab,
  openDocument,
  openDocumentAsync,
  openPaths,
  reloadDocument,
  setAppStore,
  showSidebar,
} from './store'

let dir: string
const file = (name: string, body = `# ${name}\n`) => {
  const path = join(dir, name)
  writeFileSync(path, body)
  return path
}
const state = () => appStore().getState()

beforeEach(() => {
  dir = realpathSync(mkdtempSync(join(tmpdir(), 'mdow-store-')))
  setAppStore(
    createAppStore({ prefs: { ...DEFAULT_PREFS }, session: { ...EMPTY_SESSION } }, 'light'),
  )
})

describe('tabs', () => {
  test('opening an open file switches to its tab', () => {
    const a = file('a.md')
    const b = file('b.md')
    openDocument(a)
    openDocument(b)
    openDocument(a)
    expect(state().tabs.map((tab) => tab.path)).toEqual([a, b])
    expect(state().activePath).toBe(a)
    expect(state().recents).toEqual([b, a])
  })

  test('async opening publishes the document after it loads', async () => {
    const path = file('async.md', '# Loaded off-thread\n')
    const loading = openDocumentAsync(path)
    expect(state().tabs[0]?.document).toBeNull()
    const result = await loading
    expect(result.ok).toBe(true)
    expect(state().tabs[0]?.document).toEqual(result)
    expect(state().recents).toEqual([path])
  })

  test('closing a tab while it loads does not add it back to recents', async () => {
    const path = file('closed-while-loading.md', '# Open then close\n')
    const loading = openDocumentAsync(path)
    closeTab(path)
    await loading
    expect(state().tabs).toEqual([])
    expect(state().recents).toEqual([])
  })

  test('new tabs open next to the active one', () => {
    const [a, b, c] = ['a.md', 'b.md', 'c.md'].map((name) => file(name)) as [string, string, string]
    openDocument(a)
    openDocument(b)
    cycleTab(-1)
    openDocument(c)
    expect(state().tabs.map((tab) => tab.path)).toEqual([a, c, b])
  })

  test('closing the active tab activates its neighbour', () => {
    const [a, b, c] = ['a.md', 'b.md', 'c.md'].map((name) => file(name))
    for (const path of [a!, b!, c!]) openDocument(path)
    cycleTab(-1)
    closeTab()
    expect(state().activePath).toBe(c!)
    closeTab()
    closeTab()
    expect(state().activePath).toBeNull()
  })

  test('unreadable files open as error tabs and stay out of recents', () => {
    openDocument(join(dir, 'missing.md'))
    const tab = state().tabs[0]!
    expect(tab.document).toEqual({ ok: false, error: 'missing' })
    expect(state().recents).toEqual([])
    openDocument(file('notes.txt', 'x'))
    expect(state().tabs[1]!.document).toEqual({ ok: false, error: 'unsupported' })
  })

  test('a failed reload keeps the last good document', () => {
    const a = file('a.md', '# One\n')
    openDocument(a)
    writeFileSync(a, Buffer.from([0xff, 0xfe, 0x00]))
    reloadDocument(a)
    const tab = state().tabs[0]!
    expect(tab.reloadFailed).toBe(true)
    expect(tab.document?.ok && tab.document.parsed.outline[0]?.text).toBe('One')
    writeFileSync(a, '# Two\n')
    reloadDocument(a)
    const next = state().tabs[0]!
    expect(next.reloadFailed).toBe(false)
    expect(next.revision).toBe(1)
    expect(next.document?.ok && next.document.parsed.outline[0]?.text).toBe('Two')
  })

  test('folders open as the workspace', () => {
    file('a.md')
    openPaths([dir])
    expect(state().workspace?.folder).toBe(dir)
    expect(state().prefs.sidebarMode).toBe('folder')
  })
})

test('choosing the visible sidebar mode again hides the sidebar', () => {
  showSidebar('outline')
  expect(state()).toMatchObject({ sidebarOpen: true, prefs: { sidebarMode: 'outline' } })
  showSidebar('outline')
  expect(state().sidebarOpen).toBe(false)
})
