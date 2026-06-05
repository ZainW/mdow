import { mkdtemp, mkdir, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { beforeEach, describe, expect, it } from 'vitest'
import { clearAllowedPaths, registerAllowedFile, registerAllowedPath } from '../allowed-paths'
import { buildCompanionContext, buildCompanionPromptBlocks } from './context-builder'

async function createDocs() {
  const root = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
  await mkdir(join(root, 'guide'))
  const active = join(root, 'README.md')
  const guide = join(root, 'guide', 'install.md')
  const text = join(root, 'notes.txt')
  await writeFile(active, '# Mdow\n\nOpen markdown files and read quietly.')
  await writeFile(guide, '# Install\n\nDownload the release for your platform.')
  await writeFile(text, 'not markdown')
  return { root, active, guide, text }
}

describe('companion context builder', () => {
  beforeEach(() => clearAllowedPaths())

  it('prioritizes the active markdown document and includes open-folder markdown files', async () => {
    const docs = await createDocs()
    registerAllowedFile(docs.active)
    registerAllowedPath(docs.root)

    const context = await buildCompanionContext({
      activePath: docs.active,
      openFolderPath: docs.root,
      maxSources: 8,
      maxCharsPerSource: 1_000,
    })

    expect(context.sources.map((source) => source.path)).toEqual([docs.active, docs.guide])
    expect(context.sources[0].id).toBe('src_active')
    expect(context.summary.sourceCount).toBe(2)
    expect(context.summary.truncated).toBe(false)
  })

  it('omits paths outside allowed roots and reports a context warning', async () => {
    const docs = await createDocs()

    const context = await buildCompanionContext({
      activePath: docs.active,
      openFolderPath: docs.root,
      maxSources: 8,
      maxCharsPerSource: 1_000,
    })

    expect(context.sources).toHaveLength(0)
    expect(context.summary.warnings[0]).toEqual({
      type: 'permission-denied',
      message: 'Active document is not available to the companion.',
    })
  })

  it('builds text-only prompt blocks with explicit source marker instructions', async () => {
    const docs = await createDocs()
    registerAllowedFile(docs.active)
    registerAllowedPath(docs.root)
    const context = await buildCompanionContext({
      activePath: docs.active,
      openFolderPath: docs.root,
      maxSources: 8,
      maxCharsPerSource: 1_000,
    })

    const blocks = buildCompanionPromptBlocks('How do I install it?', context)

    expect(blocks).toHaveLength(1)
    expect(blocks[0].type).toBe('text')
    if (blocks[0].type !== 'text') {
      throw new Error('expected a text prompt block')
    }
    expect(blocks[0].text).toContain('Use source markers like [[source:src_active]]')
    expect(blocks[0].text).toContain('# Install')
  })
})
