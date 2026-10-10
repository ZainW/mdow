import { describe, expect, test } from 'bun:test'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { loadDocumentAsync } from './document-worker-client'

describe('document worker', () => {
  test('reassembles chunked blocks and outline entries', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'mdow-worker-'))
    const path = join(dir, 'many-headings.md')
    const source = Array.from(
      { length: 700 },
      (_, index) => `# Heading ${index}\n\nParagraph ${index}`,
    ).join('\n\n')
    writeFileSync(path, source)

    const result = await loadDocumentAsync(path)
    expect(result.ok).toBe(true)
    if (!result.ok) return
    expect(result.parsed.outline).toHaveLength(700)
    expect(result.parsed.blocks).toHaveLength(1400)
    expect(result.parsed.outline.at(-1)?.text).toBe('Heading 699')
    expect(result.parsed.slugs.get('heading-699')).toBe(1398)
  })
})
