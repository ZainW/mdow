import { describe, expect, test } from 'bun:test'
import { DARK, LIGHT } from './theme'
import {
  cachedMermaid,
  mermaidPalette,
  parseColor,
  renderMermaid,
  renderMermaidAsync,
  resolveCss,
} from './mermaid'

const svgOf = (src: string) => Buffer.from(src.split(',')[1]!, 'base64').toString()

describe('mermaid', () => {
  test('resolves custom properties to literals and moves CSS rx onto the attribute', () => {
    const svg = resolveCss(
      '<svg style="max-width:100%;--_fg:var(--pele-fg,#1f1f1f);--_r:var(--pele-radius,4px);color:var(--_fg)" fill="var(--_fg)"><rect style="rx:var(--_r)"/></svg>',
      new Map([['--pele-fg', '#123456']]),
    )
    expect(svg).toContain('fill="#123456"')
    expect(svg).toContain('style="max-width:100%;color:#123456"')
    expect(svg).toContain('<rect rx="4"/>')
    expect(svg).not.toContain('var(')
    expect(svg).not.toContain('--_')
  })

  test('reads the theme hsl colours', () => {
    expect(parseColor('hsl(0 0% 100%)')).toEqual([255, 255, 255, 255])
    expect(parseColor('#abc')).toEqual([170, 187, 204, 255])
    expect(mermaidPalette(DARK, 'Inter').bg).toMatch(/^#[0-9a-f]{6}$/)
  })

  test('renders every diagram type to sized SVG with no CSS variables', () => {
    for (const source of [
      'flowchart LR\n  a[Open] --> b[Read]',
      'sequenceDiagram\n  A->>B: hi\n  B-->>A: hello',
      'pie\n  "a": 1\n  "b": 2',
      'gantt\n  dateFormat YYYY-MM-DD\n  section S\n    Task :2026-10-17, 3d',
      'mindmap\n  root((mdow))\n    Desktop\n    Native',
    ]) {
      const result = renderMermaid(source, mermaidPalette(LIGHT, 'Inter'))
      expect(result.ok).toBe(true)
      if (!result.ok) continue
      expect(result.width).toBeGreaterThan(0)
      const svg = svgOf(result.src)
      expect(svg).not.toContain('var(')
      expect(svg).not.toContain('@import')
    }
  })

  test('reports parse errors with their location instead of throwing', () => {
    const result = renderMermaid(
      'flowchart TD\n  A -->\n  B -->> {oops',
      mermaidPalette(LIGHT, 'Inter'),
    )
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.error).toMatch(/^Parse error on line 3/)
  })

  test('paints with the theme palette', () => {
    const result = renderMermaid('flowchart LR\n  a --> b', mermaidPalette(DARK, 'Menlo'))
    expect(result.ok).toBe(true)
    if (!result.ok) return
    const svg = svgOf(result.src)
    expect(svg).toContain(mermaidPalette(DARK, 'Menlo').surface)
    expect(svg).toContain('font-family:Menlo')
  })

  test('lays out off the main thread and caches the result', async () => {
    const palette = mermaidPalette(DARK, 'Inter')
    const source = 'flowchart TD\n  worker[Worker] --> cache[Cache]'
    const [first, second] = await Promise.all([
      renderMermaidAsync(source, palette),
      renderMermaidAsync(source, palette),
    ])
    expect(first.ok).toBe(true)
    expect(second).toBe(first)
    expect(cachedMermaid(source, palette)).toBe(first)
  })
})
