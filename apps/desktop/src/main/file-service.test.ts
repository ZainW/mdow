import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mkdtemp, readFile as fsReadFile, rm } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import { writeFile, createFileInFolder, generateUniqueName } from './file-service'

describe('writeFile', () => {
  let tempDir: string

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), 'mdow-test-'))
  })

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true })
  })

  it('writes utf-8 content to disk', async () => {
    const path = join(tempDir, 'a.md')
    await writeFile(path, '# Hello\n')
    expect(await fsReadFile(path, 'utf8')).toBe('# Hello\n')
  })
})

describe('generateUniqueName', () => {
  it('returns base name when not taken', () => {
    expect(generateUniqueName(['a.md', 'b.md'], 'Untitled.md')).toBe('Untitled.md')
  })
  it('numbers when taken', () => {
    expect(generateUniqueName(['Untitled.md'], 'Untitled.md')).toBe('Untitled-2.md')
    expect(generateUniqueName(['Untitled.md', 'Untitled-2.md'], 'Untitled.md')).toBe(
      'Untitled-3.md',
    )
  })
})

describe('createFileInFolder', () => {
  it('creates an empty markdown file with a unique name', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'mdow-test-'))
    const result = await createFileInFolder(dir)
    expect(result.path).toBe(join(dir, 'Untitled.md'))
    expect(await fsReadFile(result.path, 'utf8')).toBe('')
    const result2 = await createFileInFolder(dir)
    expect(result2.path).toBe(join(dir, 'Untitled-2.md'))
    await rm(dir, { recursive: true })
  })
})
