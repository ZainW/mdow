import { mkdtemp, writeFile, mkdir, rm } from 'fs/promises'
import { join } from 'path'
import { tmpdir } from 'os'
import { describe, expect, it, afterEach } from 'vitest'
import { buildCompanionContext, formatContextPrompt } from './context-builder'

describe('Companion context builder', () => {
  let dir: string

  afterEach(async () => {
    if (dir) await rm(dir, { recursive: true, force: true })
  })

  it('prioritizes tags, then active file, and assigns source ids', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const active = join(dir, 'active.md')
    const tagged = join(dir, 'tagged.md')
    const other = join(dir, 'other.md')
    await writeFile(active, '# Active\nactive body')
    await writeFile(tagged, '# Tagged\ntagged body')
    await writeFile(other, '# Other\nother body')

    const packet = await buildCompanionContext({
      activePath: active,
      openFolderPath: dir,
      tags: [{ kind: 'file', path: tagged, sourceId: `tag:${tagged}` }],
    })

    expect(packet.sources[0]?.path).toBe(tagged)
    expect(packet.sources.some((s) => s.path === active)).toBe(true)
    expect(packet.sources.every((s) => s.sourceId.startsWith('src:'))).toBe(true)
    expect(packet.summary.toLowerCase()).toContain('using')
  })

  it('skips non-markdown paths with a warning', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const html = join(dir, 'page.html')
    await writeFile(html, '<p>hi</p>')

    const packet = await buildCompanionContext({
      activePath: html,
      openFolderPath: null,
      tags: [],
    })

    expect(packet.sources).toHaveLength(0)
    expect(packet.warnings.some((w) => w.toLowerCase().includes('skipped'))).toBe(true)
  })

  it('formats a read-only prompt with source ids', () => {
    const prompt = formatContextPrompt(
      {
        sources: [
          {
            sourceId: 'src:/docs/a.md',
            path: '/docs/a.md',
            excerpt: 'hello',
            bytes: 5,
          },
        ],
        warnings: [],
        summary: 'Using a.md',
      },
      'Summarize this',
    )
    expect(prompt).toContain('src:/docs/a.md')
    expect(prompt).toContain('Summarize this')
    expect(prompt).toContain('read-only')
  })

  it('includes tagged folder markdown files', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const nested = join(dir, 'notes')
    await mkdir(nested)
    const doc = join(nested, 'note.md')
    await writeFile(doc, '# Note')

    const packet = await buildCompanionContext({
      activePath: null,
      openFolderPath: null,
      tags: [{ kind: 'folder', path: nested, sourceId: `tag:${nested}` }],
    })

    expect(packet.sources.some((s) => s.path === doc)).toBe(true)
  })
})
