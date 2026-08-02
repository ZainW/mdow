import { mkdtemp, mkdir, rm, writeFile } from 'fs/promises'
import { join } from 'path'
import { tmpdir } from 'os'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { CompanionContextTag } from '../../shared/types'
import { retrieveMarkdownRanges, shouldRetrieve } from './retrieval'

describe('Companion adaptive retrieval', () => {
  let dir: string

  afterEach(async () => {
    if (dir) await rm(dir, { recursive: true, force: true })
  })

  it('only triggers for structurally cross-document questions', () => {
    const active = '/docs/current.md'
    const folderTags: CompanionContextTag[] = [
      { kind: 'folder', path: '/docs', sourceId: 'tag:/docs' },
    ]

    expect(shouldRetrieve('Summarize this', active, [])).toBe(false)
    expect(shouldRetrieve('Compare this with architecture.md', active, [])).toBe(true)
    expect(
      shouldRetrieve('What do the docs in this folder say about caching?', active, folderTags),
    ).toBe(true)
  })

  it('returns at most three relevant markdown ranges within strict byte limits', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-retrieval-'))
    const nested = join(dir, 'notes')
    await mkdir(nested)
    const active = join(dir, 'current.md')
    const cacheDoc = join(nested, 'cache.md')
    await writeFile(active, '# Current\nNo answer here')
    await writeFile(
      cacheDoc,
      `# Cache\n\n## Invalidation\nCache entries are invalidated by version keys.\n${'detail '.repeat(900)}`,
    )
    await writeFile(join(nested, 'other.md'), '# Other\nUnrelated release notes')
    await writeFile(join(nested, 'page.html'), '<p>cache invalidation</p>')

    const ranges = await retrieveMarkdownRanges({
      question: 'How is caching invalidated?',
      roots: [dir],
      excludedPaths: [active],
    })

    expect(ranges.length).toBeLessThanOrEqual(3)
    expect(ranges.every((range) => range.bytes <= 4_096)).toBe(true)
    expect(ranges.reduce((sum, range) => sum + range.bytes, 0)).toBeLessThanOrEqual(12_288)
    expect(ranges[0]?.path).toBe(cacheDoc)
    expect(ranges[0]?.excerpt).toContain('invalidated by version keys')
  })

  it('does not scan roots when the caller skips adaptive retrieval', async () => {
    const scan = vi.fn()
    if (shouldRetrieve('Summarize this', '/docs/current.md', [])) {
      await retrieveMarkdownRanges({
        question: 'Summarize this',
        roots: ['/docs'],
        excludedPaths: [],
        scan,
      })
    }

    expect(scan).not.toHaveBeenCalled()
  })
})
