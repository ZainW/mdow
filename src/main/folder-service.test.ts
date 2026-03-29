import { describe, expect, it, beforeEach, afterEach } from 'vitest'
import { join } from 'path'
import { mkdtemp, mkdir, writeFile, rm } from 'fs/promises'
import { tmpdir } from 'os'
import { scanFolder } from './folder-service'

describe('scanFolder', () => {
  let tempDir: string

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), 'mdview-test-'))
  })

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true })
  })

  it('returns empty array for empty directory', async () => {
    const result = await scanFolder(tempDir)
    expect(result).toEqual([])
  })

  it('finds markdown files', async () => {
    await writeFile(join(tempDir, 'readme.md'), '# Hello')
    await writeFile(join(tempDir, 'notes.markdown'), 'notes')
    await writeFile(join(tempDir, 'doc.mdx'), 'mdx content')

    const result = await scanFolder(tempDir)
    expect(result).toHaveLength(3)
    expect(result.map((n) => n.name).sort()).toEqual(['doc.mdx', 'notes.markdown', 'readme.md'])
  })

  it('ignores non-markdown files', async () => {
    await writeFile(join(tempDir, 'readme.md'), '# Hello')
    await writeFile(join(tempDir, 'script.ts'), 'code')
    await writeFile(join(tempDir, 'style.css'), 'css')
    await writeFile(join(tempDir, 'data.json'), '{}')

    const result = await scanFolder(tempDir)
    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('readme.md')
  })

  it('ignores hidden files and directories', async () => {
    await writeFile(join(tempDir, '.hidden.md'), 'hidden')
    await mkdir(join(tempDir, '.git'))
    await writeFile(join(tempDir, '.git', 'config.md'), 'git config')
    await writeFile(join(tempDir, 'visible.md'), 'visible')

    const result = await scanFolder(tempDir)
    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('visible.md')
  })

  it('scans subdirectories recursively', async () => {
    await mkdir(join(tempDir, 'docs'))
    await writeFile(join(tempDir, 'docs', 'guide.md'), 'guide')
    await writeFile(join(tempDir, 'readme.md'), 'readme')

    const result = await scanFolder(tempDir)
    expect(result).toHaveLength(2)

    // Directories come first
    const dir = result[0]
    expect(dir.name).toBe('docs')
    expect(dir.isDirectory).toBe(true)
    expect(dir.children).toHaveLength(1)
    expect(dir.children![0].name).toBe('guide.md')

    // Then files
    expect(result[1].name).toBe('readme.md')
    expect(result[1].isDirectory).toBe(false)
  })

  it('sorts directories before files', async () => {
    await writeFile(join(tempDir, 'zebra.md'), 'z')
    await mkdir(join(tempDir, 'alpha'))
    await writeFile(join(tempDir, 'alpha', 'file.md'), 'a')

    const result = await scanFolder(tempDir)
    expect(result[0].isDirectory).toBe(true)
    expect(result[0].name).toBe('alpha')
    expect(result[1].isDirectory).toBe(false)
    expect(result[1].name).toBe('zebra.md')
  })

  it('sorts alphabetically within same type', async () => {
    await writeFile(join(tempDir, 'charlie.md'), 'c')
    await writeFile(join(tempDir, 'alpha.md'), 'a')
    await writeFile(join(tempDir, 'bravo.md'), 'b')

    const result = await scanFolder(tempDir)
    expect(result.map((n) => n.name)).toEqual(['alpha.md', 'bravo.md', 'charlie.md'])
  })

  it('excludes empty directories (no markdown inside)', async () => {
    await mkdir(join(tempDir, 'empty'))
    await mkdir(join(tempDir, 'has-code'))
    await writeFile(join(tempDir, 'has-code', 'script.ts'), 'code')

    const result = await scanFolder(tempDir)
    expect(result).toHaveLength(0)
  })

  it('includes directories that have nested markdown', async () => {
    await mkdir(join(tempDir, 'outer'))
    await mkdir(join(tempDir, 'outer', 'inner'))
    await writeFile(join(tempDir, 'outer', 'inner', 'deep.md'), 'deep')

    const result = await scanFolder(tempDir)
    expect(result).toHaveLength(1)
    expect(result[0].name).toBe('outer')
    expect(result[0].children![0].name).toBe('inner')
    expect(result[0].children![0].children![0].name).toBe('deep.md')
  })

  it('sets correct paths on nodes', async () => {
    await writeFile(join(tempDir, 'file.md'), 'content')

    const result = await scanFolder(tempDir)
    expect(result[0].path).toBe(join(tempDir, 'file.md'))
  })

  it('handles case-insensitive extensions', async () => {
    await writeFile(join(tempDir, 'upper.MD'), 'upper')
    await writeFile(join(tempDir, 'mixed.Markdown'), 'mixed')

    const result = await scanFolder(tempDir)
    expect(result).toHaveLength(2)
  })

  it('file nodes have no children property', async () => {
    await writeFile(join(tempDir, 'file.md'), 'content')

    const result = await scanFolder(tempDir)
    expect(result[0].children).toBeUndefined()
  })
})
