import { mkdtemp, writeFile, rm } from 'fs/promises'
import { join } from 'path'
import { tmpdir } from 'os'
import { describe, expect, it, afterEach } from 'vitest'
import type { CompanionContextTag } from '../../shared/types'
import { ContextLedger } from './context-ledger'
import { buildCompanionContext, formatContextPrompt } from './context-builder'

describe('Companion context builder', () => {
  let dir: string

  afterEach(async () => {
    if (dir) await rm(dir, { recursive: true, force: true })
  })

  it('prioritizes the active file, then tags, and assigns source ids', async () => {
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

    expect(packet.sources[0]?.path).toBe(active)
    expect(packet.sources.some((s) => s.path === tagged)).toBe(true)
    expect(packet.sources.every((s) => s.sourceId.startsWith('src:'))).toBe(true)
    expect(packet.summary.toLowerCase()).toContain('using')
  })

  it('does not inject unrelated files from the open folder', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const active = join(dir, 'active.md')
    const unrelated = join(dir, 'unrelated.md')
    await writeFile(active, '# Active\nactive body')
    await writeFile(unrelated, '# Unrelated\nshould stay out')

    const packet = await buildCompanionContext({
      activePath: active,
      openFolderPath: dir,
      tags: [],
      question: 'Summarize this document',
    })

    expect(packet.sources.map((source) => source.path)).toEqual([active])
    expect(packet.trace).toMatchObject({
      focusedCount: 1,
      searchedCount: 0,
      readRangeCount: 0,
    })
  })

  it('keeps explicit file attachments but does not expand a folder tag eagerly', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const active = join(dir, 'active.md')
    const tagged = join(dir, 'tagged.md')
    const unrelated = join(dir, 'unrelated.md')
    await writeFile(active, '# Active')
    await writeFile(tagged, '# Tagged')
    await writeFile(unrelated, '# Unrelated')

    const packet = await buildCompanionContext({
      activePath: active,
      openFolderPath: dir,
      tags: [
        { kind: 'file', path: tagged, sourceId: `tag:${tagged}` },
        { kind: 'folder', path: dir, sourceId: `tag:${dir}` },
      ],
      question: 'Compare the focused and attached file',
    })

    expect(packet.sources.map((source) => source.path)).toEqual([active, tagged])
  })

  it('keeps the active file when tags consume the context budget', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const active = join(dir, 'active.md')
    await writeFile(active, '# Focused document\nmust be included')
    const tagPaths = Array.from({ length: 6 }, (_, index) => join(dir, `tag-${index}.md`))
    await Promise.all(tagPaths.map((path) => writeFile(path, 'x'.repeat(24_000))))
    const tags: CompanionContextTag[] = tagPaths.map((path) => ({
      kind: 'file',
      path,
      sourceId: `tag:${path}`,
    }))

    const packet = await buildCompanionContext({
      activePath: active,
      openFolderPath: null,
      tags,
    })

    expect(packet.sources[0]?.path).toBe(active)
    expect(packet.sources.find((source) => source.path === active)?.excerpt).toContain(
      'must be included',
    )
  })

  it('selects a relevant section instead of truncating a large focused document from the top', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const active = join(dir, 'handbook.md')
    await writeFile(
      active,
      `# Handbook\n\n## Background\n${'x'.repeat(20_000)}\n\n## Authentication\nRotate tokens daily.`,
    )

    const packet = await buildCompanionContext({
      activePath: active,
      openFolderPath: dir,
      tags: [],
      question: 'How does authentication work?',
    })

    expect(packet.sources[0]?.excerpt).toContain('## Authentication')
    expect(packet.sources[0]?.excerpt).toContain('Rotate tokens daily')
    expect(packet.sources[0]?.bytes).toBeLessThanOrEqual(16_384)
  })

  it('sends identity metadata instead of unchanged document content within a session', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const active = join(dir, 'active.md')
    await writeFile(active, `# Active\nprivate body that should not be repeated\n${'detail '.repeat(400)}`)
    const ledger = new ContextLedger()
    const input = {
      activePath: active,
      openFolderPath: dir,
      tags: [],
      question: 'Summarize this',
      ledger,
    }

    const first = await buildCompanionContext(input)
    const second = await buildCompanionContext(input)

    expect(first.sources[0]?.excerpt).toContain('private body')
    expect(second.sources[0]?.excerpt).toContain('Content unchanged from earlier in this session')
    expect(second.sources[0]?.excerpt).not.toContain('private body')
    expect(second.trace.injectedBytes).toBeLessThan(first.trace.injectedBytes)
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
        trace: {
          focusedCount: 1,
          attachedCount: 0,
          searchedCount: 0,
          readRangeCount: 0,
          injectedBytes: 5,
          estimatedTokens: 2,
          retrievalMode: 'focused-only',
          items: [{ path: '/docs/a.md', reason: 'focused', bytes: 5 }],
        },
      },
      'Summarize this',
    )
    expect(prompt).toContain('src:/docs/a.md')
    expect(prompt).toContain('Summarize this')
    expect(prompt).toContain('read-only')
  })

})
