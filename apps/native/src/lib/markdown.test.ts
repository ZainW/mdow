import { describe, expect, test } from 'bun:test'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { parseHtml, parseMarkdown, slugify, stripFrontmatter } from './markdown'

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
})
