import { describe, expect, it } from 'vitest'
import { PREVIEW_MIN_CHARS, sliceDocumentHead } from './markdown-preview'

const paragraph = 'Plain prose that fills out the document without any structure of its own.\n\n'

function repeatTo(chars: number, chunk: string): string {
  return chunk.repeat(Math.ceil(chars / chunk.length))
}

describe('sliceDocumentHead', () => {
  it('skips documents that parse quickly anyway', () => {
    expect(sliceDocumentHead('# Small\n\nText')).toBeNull()
  })

  it('cuts just before a heading past the target length', () => {
    const text = repeatTo(PREVIEW_MIN_CHARS, `## Section\n\n${paragraph}`)
    const head = sliceDocumentHead(text)

    expect(head).not.toBeNull()
    expect(head!.length).toBeGreaterThanOrEqual(64 * 1024)
    expect(head!.length).toBeLessThan(text.length)
    expect(text.slice(head!.length).startsWith('## Section')).toBe(true)
  })

  it('never cuts inside a code fence', () => {
    const fenced = '```md\n' + repeatTo(80 * 1024, '# not a heading\n') + '```\n\n'
    const text = fenced + repeatTo(PREVIEW_MIN_CHARS, `# Real\n\n${paragraph}`)
    const head = sliceDocumentHead(text)!

    expect(head.length).toBeGreaterThanOrEqual(fenced.length)
    expect(text.slice(head.length).startsWith('# Real')).toBe(true)
  })

  it('does not treat a fence with an info string as the end of a block', () => {
    const fenced =
      '```\n' +
      repeatTo(40 * 1024, 'code\n') +
      '```bash\n' +
      repeatTo(40 * 1024, '# comment\n') +
      '```\n\n'
    const text = fenced + repeatTo(PREVIEW_MIN_CHARS, `# Real\n\n${paragraph}`)
    const head = sliceDocumentHead(text)!

    expect(head.length).toBeGreaterThanOrEqual(fenced.length)
    expect(text.slice(head.length).startsWith('# Real')).toBe(true)
  })

  it('falls back to a blank line when there are no headings', () => {
    const text = repeatTo(PREVIEW_MIN_CHARS, paragraph)
    const head = sliceDocumentHead(text)!

    expect(head.length).toBeGreaterThanOrEqual(64 * 1024)
    expect(head.length).toBeLessThan(300 * 1024)
    expect(text.slice(head.length).startsWith('\n')).toBe(true)
  })

  it('keeps frontmatter whole', () => {
    const frontmatter = '---\n' + repeatTo(70 * 1024, 'key: value\n') + '---\n'
    const text = frontmatter + repeatTo(PREVIEW_MIN_CHARS, `# Heading\n\n${paragraph}`)
    const head = sliceDocumentHead(text)!

    expect(head.startsWith(frontmatter)).toBe(true)
  })
})
