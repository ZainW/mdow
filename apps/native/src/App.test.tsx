import { beforeEach, describe, expect, test } from 'bun:test'
import { mkdtempSync, realpathSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { flushSync } from '@gpuix/react'
import { createTestRoot, hasNativeTestRenderer } from '@gpuix/react/testing'
import { connectTest } from '@gpuix/react/automation'
import { App } from './App'
import { EMPTY_SESSION } from './lib/persist'
import { DEFAULT_PREFS } from './lib/prefs'
import { createAppStore, openDocument, setAppStore, setDragging, setOverlay } from './store'

// The GPU test renderer needs a display on Linux (CI runs these under xvfb in native.yml).
const hasDisplay =
  process.platform !== 'linux' || !!process.env.DISPLAY || !!process.env.WAYLAND_DISPLAY
const describeUi = hasNativeTestRenderer && hasDisplay ? describe : describe.skip

function mount() {
  const root = createTestRoot({ width: 1100, height: 720 })
  flushSync(() => root.render(<App saveNow={() => {}} />))
  root.renderer.flush()
  return root
}

const text = (root: ReturnType<typeof createTestRoot>) => {
  root.renderer.flush()
  return root.renderer.getAllText().join('\n')
}

/** What the last frame painted, including text native `<markdown>` builds in Rust. */
const painted = (root: ReturnType<typeof createTestRoot>) => {
  root.renderer.flush()
  return root.renderer.getPaintedText().join('\n')
}

beforeEach(() => {
  setAppStore(
    createAppStore({ prefs: { ...DEFAULT_PREFS }, session: { ...EMPTY_SESSION } }, 'dark'),
  )
})

describeUi('App', () => {
  test('shows the welcome screen with no documents', () => {
    const root = mount()
    expect(text(root)).toContain('A quiet markdown viewer')
    root.unmount()
  })

  test('opens a document into a tab and the reader', () => {
    const dir = realpathSync(mkdtempSync(join(tmpdir(), 'mdow-app-')))
    const path = join(dir, 'guide.md')
    writeFileSync(path, '---\ntitle: The Guide\n---\n# Heading\n\nBody text.\n')
    const root = mount()
    flushSync(() => openDocument(path))
    const shown = text(root)
    expect(shown).toContain('guide.md')
    expect(shown).toContain('The Guide')
    root.unmount()
  })

  test('shows copy for files that cannot be opened', () => {
    const root = mount()
    flushSync(() => openDocument('/nope/missing.md'))
    expect(text(root)).toContain('File not found')
    root.unmount()
  })

  test('the command palette filters actions', async () => {
    const root = mount()
    flushSync(() => setOverlay('palette'))
    const app = await connectTest(root.renderer)
    await app.getByTestId('palette-input').fill('wide')
    const shown = text(root)
    expect(shown).toContain('Toggle Wide Mode')
    expect(shown).not.toContain('Zoom In')
    root.unmount()
  })

  test('dragging the scrollbar moves through a long document', async () => {
    const dir = realpathSync(mkdtempSync(join(tmpdir(), 'mdow-app-')))
    const path = join(dir, 'long.md')
    const sections = Array.from({ length: 300 }, (_, i) => `## Section ${i}\n\nParagraph ${i}.`)
    writeFileSync(path, sections.join('\n\n'))
    const root = mount()
    flushSync(() => openDocument(path))
    root.renderer.flush()
    const app = await connectTest(root.renderer)
    const scrollbar = app.getByTestId('reader-scrollbar')
    await scrollbar.waitFor()
    expect(painted(root)).toContain('Section 0')
    await scrollbar.dragBy(0, 2000, { steps: 8, offset: { x: 0, y: -250 } })
    const shown = painted(root)
    expect(shown).toContain('Section 299')
    expect(shown).not.toMatch(/^Section 0$/m)
    await app.screenshot({ path: '/tmp/sp/scrollbar-end.png' })
    root.unmount()
  })

  test('the welcome drop zone answers a drag over the window', async () => {
    const root = mount()
    expect(text(root)).toContain('Anywhere in this window')
    flushSync(() => setDragging(true))
    expect(text(root)).toContain('Release to open in Mdow')
    const app = await connectTest(root.renderer)
    await app.screenshot({ path: '/tmp/sp/drop-active.png' })
    flushSync(() => setDragging(false))
    expect(text(root)).toContain('Anywhere in this window')
    root.unmount()
  })
})
