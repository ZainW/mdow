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
  test('resolves custom properties and color-mix to literals', () => {
    const svg = resolveCss(
      '<svg style="--bg:#000000;--fg:#ffffff"><style>svg { --_line: var(--line, color-mix(in srgb, var(--fg) 50%, var(--bg))); }</style><path stroke="var(--_line)"/></svg>',
    )
    expect(svg).toContain('stroke="#808080"')
    expect(svg).not.toContain('var(')
    expect(svg).not.toContain('color-mix')
  })

  test('reads the theme hsl colours', () => {
    expect(parseColor('hsl(0 0% 100%)')).toEqual([255, 255, 255, 255])
    expect(parseColor('#abc')).toEqual([170, 187, 204, 255])
    expect(mermaidPalette(DARK, 'Inter').bg).toMatch(/^#[0-9a-f]{6}$/)
  })

  test('renders flowcharts and sequence diagrams to sized SVG with no CSS variables', () => {
    for (const source of [
      'flowchart LR\n  a[Open] --> b[Read]',
      'sequenceDiagram\n  A->>B: hi\n  B-->>A: hello',
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

  test('reports unsupported diagrams instead of throwing', () => {
    const result = renderMermaid('pie\n  "a": 1', mermaidPalette(LIGHT, 'Inter'))
    expect(result.ok).toBe(false)
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
