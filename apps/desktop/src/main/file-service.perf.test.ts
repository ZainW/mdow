import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { join } from 'path'
import { mkdtemp, rm, writeFile } from 'fs/promises'
import { tmpdir } from 'os'

vi.mock('electron', () => ({
  dialog: {
    showOpenDialog: vi.fn(),
  },
  BrowserWindow: vi.fn(),
}))

vi.mock('chokidar', () => ({
  watch: vi.fn(() => ({
    on: vi.fn(),
    close: vi.fn(),
  })),
}))

import { readFileContent } from './file-service'

describe('file-service performance', () => {
  let tempDir: string

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), 'mdow-file-perf-'))
  })

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true })
  })

  it('reads a large markdown file within the file-open budget', async () => {
    const filePath = join(tempDir, 'large.md')
    const content = Array.from({ length: 25_000 }, (_, i) => `- line ${i}`).join('\n')
    await writeFile(filePath, content)

    const startedAt = performance.now()
    const result = await readFileContent(filePath)

    expect(performance.now() - startedAt).toBeLessThan(250)
    expect(result).toHaveLength(content.length)
  })
})
