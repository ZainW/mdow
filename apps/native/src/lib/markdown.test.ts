import { describe, expect, test } from 'bun:test'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  CODE_CHUNK_LINES,
  LIST_CHUNK_ITEMS,
  parseHtml,
  parseMarkdown,
  slugify,
  stripFrontmatter,
  TABLE_CHUNK_ROWS,
} from './markdown'

const kinds = (markdown: string, path = '/docs/a.md') =>
  parseMarkdown(markdown, path).blocks.map((block) => block.kind)

describe('parseMarkdown', () => {
  test('splits top-level blocks into rows', () => {
    expect(kinds('# Title\n\nPara one.\n\n- a\n- b\n\n---\n\n```js\nx\n```\n')).toEqual([
      'heading',
      'markdown',
      'markdown',
      'rule',
      'code',
    ])
  })

  test('strips frontmatter and reads its title', () => {
    const doc = parseMarkdown('---\ntitle: "Hello"\ntags: [a]\n---\n# Body\n', '/a.md')
    expect(doc.title).toBe('Hello')
    expect(doc.blocks[0]).toMatchObject({ kind: 'heading', text: 'Body' })
    expect(stripFrontmatter('no frontmatter').title).toBeNull()
  })

  test('builds a GitHub-style outline with duplicate suffixes', () => {
    const doc = parseMarkdown('# Intro\n\n## Setup & Use\n\n## Setup & Use\n', '/a.md')
    expect(doc.outline.map((entry) => entry.slug)).toEqual(['intro', 'setup--use', 'setup--use-1'])
    expect(doc.slugs.get('setup--use-1')).toBe(2)
    expect(slugify('Émoji ✨ Title')).toBe('émoji--title')
  })

  test('turns GitHub alerts into alert blocks', () => {
    const [block] = parseMarkdown('> [!WARNING]\n> Mind the **gap**.\n', '/a.md').blocks
    expect(block).toMatchObject({ kind: 'alert', alert: 'warning', source: 'Mind the **gap**.' })
    expect(kinds('> plain quote\n')).toEqual(['markdown'])
  })

  test('collects footnotes and replaces references with markers', () => {
    const doc = parseMarkdown(
      'See[^2] and[^x] but not `[^2]`.\n\n[^2]: Second\n  continued.\n[^x]: Named\n',
      '/a.md',
    )
    expect(doc.blocks[0]).toMatchObject({ kind: 'markdown', source: 'See¹ and[x] but not `[^2]`.' })
    expect(doc.blocks.at(-1)).toMatchObject({
      kind: 'footnotes',
      items: [
        { label: '2', marker: '¹', source: 'Second continued.' },
        { label: 'x', marker: '[x]', source: 'Named' },
      ],
    })
  })

  test('leaves footnote-like lines inside code fences alone', () => {
    const doc = parseMarkdown('```\n[^1]: not a note\n```\n', '/a.md')
    expect(doc.blocks).toHaveLength(1)
    expect(doc.blocks[0]).toMatchObject({ kind: 'code', code: '[^1]: not a note' })
  })

  test('renders task list boxes as glyphs', () => {
    const [block] = parseMarkdown('- [ ] todo\n- [x] done\n  - [X] nested\n', '/a.md').blocks
    expect(block).toMatchObject({ source: '- ☐ todo\n- ☑ done\n  - ☑ nested' })
  })

  test('standalone images become image rows with their pixel size', () => {
    const dir = mkdtempSync(join(tmpdir(), 'mdow-md-'))
    // 1x1 PNG
    writeFileSync(
      join(dir, 'dot.png'),
      Buffer.from(
        'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
        'base64',
      ),
    )
    const doc = parseMarkdown(
      '![Dot](dot.png)\n\n![Gone](missing.png)\n\nText ![inline](dot.png)\n',
      join(dir, 'a.md'),
    )
    expect(doc.blocks[0]).toMatchObject({
      kind: 'image',
      src: join(dir, 'dot.png'),
      width: 1,
      height: 1,
    })
    expect(doc.blocks[1]).toMatchObject({ kind: 'image', src: null, alt: 'Gone' })
    expect(doc.blocks[2]).toMatchObject({ kind: 'markdown' })
  })

  test('raw HTML blocks are converted instead of shown as tags', () => {
    const doc = parseMarkdown(
      '<div align="center"><h2>Hi</h2><script>x()</script></div>\n',
      '/a.md',
    )
    expect(doc.blocks).toEqual([expect.objectContaining({ kind: 'heading', text: 'Hi' })])
  })

  test('code block language is normalized from the info string', () => {
    const [block] = parseMarkdown('```TypeScript title="x"\nlet a\n```\n', '/a.md').blocks
    expect(block).toMatchObject({ kind: 'code', language: 'typescript' })
  })

  test('parses HTML documents through the same pipeline', () => {
    const doc = parseHtml(
      '<html><head><title>x</title></head><body><h1>Page</h1><p>Hi</p></body></html>',
      '/site/index.html',
    )
    expect(doc.blocks.map((block) => block.kind)).toEqual(['heading', 'markdown'])
    expect(doc.outline[0]?.text).toBe('Page')
  })

  test('takes the HTML title when there is no frontmatter title', () => {
    const doc = parseHtml('<!DOCTYPE html><title>Report &amp; Notes</title><p>x</p>', '/a.html')
    expect(doc.title).toBe('Report & Notes')
    expect(doc.blocks.map((block) => block.text)).toEqual(['x'])
  })
})

describe('splitting large blocks into rows', () => {
  test('a long list becomes joined rows that keep their numbering', () => {
    const items = Array.from({ length: LIST_CHUNK_ITEMS * 2 + 3 }, (_, i) => `${i + 1}. item ${i}`)
    const blocks = parseMarkdown(items.join('\n') + '\n', '/a.md').blocks
    expect(blocks).toHaveLength(3)
    expect(blocks.map((block) => block.joinNext)).toEqual([true, true, false])
    expect(blocks[1]).toMatchObject({ kind: 'markdown' })
    expect((blocks[1] as { source: string }).source.startsWith(`${LIST_CHUNK_ITEMS + 1}. `)).toBe(
      true,
    )
    expect(blocks.map((block) => block.text).join('\n')).toContain(
      `item ${LIST_CHUNK_ITEMS * 2 + 2}`,
    )
  })

  test('a long code fence is cut at blank lines and copies whole', () => {
    const lines = Array.from({ length: CODE_CHUNK_LINES * 3 }, (_, i) =>
      i % 10 === 9 ? '' : `x${i}`,
    )
    const code = lines.join('\n')
    const blocks = parseMarkdown('```ts\n' + code + '\n```\n', '/a.md').blocks
    expect(blocks.length).toBeGreaterThan(1)
    expect(blocks.map((block) => block.kind === 'code' && block.part)).toEqual([
      'first',
      ...Array(blocks.length - 2).fill('middle'),
      'last',
    ])
    expect(blocks[0]).toMatchObject({ copy: code })
    expect(blocks.map((block) => (block as { code: string }).code).join('\n')).toBe(code)
    // Seams land after a blank line.
    for (const block of blocks.slice(0, -1)) {
      expect(
        (block as { code: string }).code.endsWith('\n') ||
          (block as { code: string }).code.split('\n').at(-1) === '',
      ).toBe(true)
    }
  })

  test('mermaid fences are never split', () => {
    const body = Array.from({ length: CODE_CHUNK_LINES * 3 }, (_, i) => `  a${i} --> a${i + 1}`)
    const blocks = parseMarkdown(
      '```mermaid\nflowchart TD\n' + body.join('\n') + '\n```\n',
      '/a.md',
    ).blocks
    expect(blocks).toHaveLength(1)
    expect(blocks[0]).toMatchObject({ kind: 'code', language: 'mermaid', part: 'whole' })
  })

  test('a long table becomes table rows with one header and shared widths', () => {
    const rows = Array.from(
      { length: TABLE_CHUNK_ROWS * 2 + 1 },
      (_, i) => `| ${i} | row **${i}** |`,
    )
    const blocks = parseMarkdown(
      `| id | description |\n|---:|---|\n${rows.join('\n')}\n`,
      '/a.md',
    ).blocks
    expect(blocks.map((block) => block.kind)).toEqual(['table', 'table', 'table'])
    const [first, second] = blocks as Extract<(typeof blocks)[number], { kind: 'table' }>[]
    expect(first!.header).toEqual(['id', 'description'])
    expect(second!.header).toBeNull()
    expect(first!.widths).toEqual(second!.widths)
    expect(first!.widths[1]!).toBeGreaterThan(first!.widths[0]!)
    expect(first!.align).toEqual(['right', null])
    expect(second!.rows[0]).toEqual([`${TABLE_CHUNK_ROWS}`, `row **${TABLE_CHUNK_ROWS}**`])
    expect(second!.text).toContain(`row ${TABLE_CHUNK_ROWS}`)
  })

  test('small tables stay native markdown', () => {
    expect(kinds('| a | b |\n|---|---|\n| 1 | 2 |\n')).toEqual(['markdown'])
  })

  test('data URL images get their size', () => {
    const svg = Buffer.from(
      '<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"/>',
    ).toString('base64')
    const [block] = parseMarkdown(`![Chart](<data:image/svg+xml;base64,${svg}>)\n`, '/a.md').blocks
    expect(block).toMatchObject({ kind: 'image', width: 40, height: 20, alt: 'Chart' })
  })
})
