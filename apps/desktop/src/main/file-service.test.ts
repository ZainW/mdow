import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mkdtemp, readFile as fsReadFile, rm } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import { writeFile } from './file-service'

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
