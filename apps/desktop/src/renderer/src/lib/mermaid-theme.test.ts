import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  MERMAID_PALETTES,
  getMermaidInitConfig,
  getMermaidPaletteId,
  getMermaidThemeVariables,
  readMermaidTokens,
  resolveMermaidPaletteId,
} from './mermaid-theme'

const INDEX_CSS = readFileSync(resolve('src/renderer/src/assets/styles/index.css'), 'utf8')
const MARKDOWN_CSS = readFileSync(resolve('src/renderer/src/assets/styles/markdown.css'), 'utf8')

describe('mermaid theme lock', () => {
  it('locks the default palette to the mdow paper tokens', () => {
    expect(MERMAID_PALETTES.light).toMatchObject({
      paper: '#ffffff',
      paperWarm: '#faf9f5',
      ink: '#3b3b3b',
      muted: '#5e5e58',
      line: '#d8d7d0',
      teal: '#0f8080',
      fontSize: '13.5px',
      radius: '6px',
    })
    expect(MERMAID_PALETTES.light.fontFamily).toContain('IBM Plex Mono')
  })

  it('locks Fog & Pine tokens when that palette is selected', () => {
    expect(MERMAID_PALETTES['fog-pine']).toMatchObject({
      paper: '#f1f3ed',
      paperWarm: '#e7eae2',
      ink: '#1d2620',
      muted: '#4e5c55',
      line: '#9aa399',
      teal: '#2b5c55',
    })
  })

  it('reads Fog & Pine from data-palette without treating it as dark', () => {
    const root = document.createElement('html')
    root.dataset.palette = 'fog-pine'
    expect(getMermaidPaletteId(root)).toBe('fog-pine')
  })

  it('initializes Mermaid with the base theme, not the stock rainbow themes', () => {
    const config = getMermaidInitConfig('light')
    expect(config.theme).toBe('base')
    expect(config.themeVariables.primaryColor).toBe('#faf9f5')
    expect(config.themeVariables.primaryBorderColor).toBe('#d8d7d0')
    expect(config.themeVariables.lineColor).toBe('#5e5e58')
    expect(config.themeVariables.background).toBe('transparent')
    expect(config.flowchart.htmlLabels).toBe(false)
    expect(config.flowchart.useMaxWidth).toBe(false)
  })

  it('uses teal only as a single accent, not the default node fill', () => {
    const variables = getMermaidThemeVariables(MERMAID_PALETTES.light)
    expect(variables.primaryColor).not.toBe(MERMAID_PALETTES.light.teal)
    expect(variables.pie1).toBe(MERMAID_PALETTES.light.teal)
    expect(variables.pie2).toBe(MERMAID_PALETTES.light.paperWarm)
    expect(variables.cScale1).toBe(MERMAID_PALETTES.light.teal)
    expect(variables.cScale0).toBe(MERMAID_PALETTES.light.paperWarm)
  })

  it('prefers CSS custom properties when the app theme has set them', () => {
    document.documentElement.style.setProperty('--md-ink', '#111111')
    document.documentElement.style.setProperty('--md-paper-warm', '#eeeeee')
    const tokens = readMermaidTokens('light', document.documentElement)
    expect(tokens.ink).toBe('#111111')
    expect(tokens.paperWarm).toBe('#eeeeee')
    document.documentElement.removeAttribute('style')
  })

  it('resolves an explicit isDark flag for tests and theme sync', () => {
    expect(resolveMermaidPaletteId(true)).toBe('dark')
    expect(resolveMermaidPaletteId(false)).toBe('light')
  })

  it('declares matching CSS custom properties for both palettes', () => {
    expect(INDEX_CSS).toContain('--md-paper: #ffffff')
    expect(INDEX_CSS).toContain('--md-paper-warm: #faf9f5')
    expect(INDEX_CSS).toContain('--md-ink: #3b3b3b')
    expect(INDEX_CSS).toContain('--md-teal: #0f8080')
    expect(INDEX_CSS).toContain("[data-palette='fog-pine']")
    expect(INDEX_CSS).toContain('--md-paper: #f1f3ed')
    expect(INDEX_CSS).toContain('IBM Plex Mono')
  })

  it('locks diagram CSS to paper-warm fills, muted edges, and IBM Plex Mono', () => {
    expect(MARKDOWN_CSS).toContain('var(--md-paper-warm)')
    expect(MARKDOWN_CSS).toContain('var(--md-ink-muted)')
    expect(MARKDOWN_CSS).toContain('var(--md-diagram-font)')
    expect(MARKDOWN_CSS).toContain('var(--md-teal)')
    expect(MARKDOWN_CSS).not.toContain('Newsreader')
  })
})
