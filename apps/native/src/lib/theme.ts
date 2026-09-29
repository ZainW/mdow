import type { GpuixTheme, SyntaxTheme } from '@gpuix/react'
import type { AlertKind } from './markdown'

export type ColorScheme = 'light' | 'dark'

export interface Theme {
  scheme: ColorScheme
  background: string
  foreground: string
  muted: string
  mutedForeground: string
  primary: string
  accent: string
  destructive: string
  border: string
  borderSubtle: string
  sidebar: string
  sidebarAccent: string
  surfaceRaised: string
  surfaceWell: string
  selection: string
  findMatch: string
  findActive: string
  syntax: SyntaxTheme
  alerts: Record<AlertKind, string>
  /** Mermaid "ink on paper" colours, from the desktop's --md-* tokens. */
  diagram: { paper: string; paperWarm: string; ink: string; inkMuted: string; line: string }
}

/** The same colour at `alpha` opacity. Theme colours are `hsl()` strings or hex. */
export function withAlpha(color: string, alpha: number) {
  if (color.startsWith('hsl(') && !color.includes('/')) return color.replace(/\)$/, ` / ${alpha})`)
  if (/^#[0-9a-f]{6}$/i.test(color)) {
    return `${color}${Math.round(alpha * 255)
      .toString(16)
      .padStart(2, '0')}`
  }
  return color
}

/** GPUI hsla takes hue as a 0–1 turn; keep the Rust build's numbers verbatim. */
function hsla(h: number, s: number, l: number, a = 1) {
  const hsl = `${(h * 360).toFixed(2)} ${(s * 100).toFixed(2)}% ${(l * 100).toFixed(2)}%`
  return a === 1 ? `hsl(${hsl})` : `hsl(${hsl} / ${a})`
}

function githubSyntax(
  fg: string,
  comment: string,
  keyword: string,
  string: string,
  entity: string,
  constant: string,
): SyntaxTheme {
  return {
    comment,
    keyword,
    string,
    stringSpecial: string,
    escape: constant,
    number: constant,
    boolean: constant,
    constant,
    typeName: entity,
    typeBuiltin: keyword,
    constructor: entity,
    function: entity,
    functionBuiltin: entity,
    macroName: entity,
    property: constant,
    variable: fg,
    variableSpecial: constant,
    parameter: fg,
    operator: keyword,
    punctuation: fg,
    tag: keyword,
    attribute: constant,
    label: entity,
    invalid: keyword,
  }
}

export const LIGHT: Theme = {
  scheme: 'light',
  background: hsla(0.08672199, 0.39970066, 0.97152986),
  foreground: hsla(0.04368636, 0.6948904, 0.03135708),
  muted: hsla(0.08673897, 0.24669178, 0.9449269),
  mutedForeground: hsla(0.05796655, 0.08543156, 0.33432802),
  primary: hsla(0.60388106, 0.64902184, 0.5053445),
  accent: hsla(0.08304337, 1.0, 0.40092257),
  destructive: hsla(0.99228718, 0.6827012, 0.47648946),
  border: hsla(0.08681399, 0.15087865, 0.85268928),
  borderSubtle: hsla(0.0867741, 0.1893196, 0.90505948),
  sidebar: hsla(0.08672199, 0.39970066, 0.97152986),
  sidebarAccent: hsla(0.08677273, 0.21983283, 0.918019),
  surfaceRaised: hsla(0.08672199, 0.28, 0.992),
  surfaceWell: hsla(0.08673897, 0.24669178, 0.93),
  selection: hsla(0.60388106, 0.64902184, 0.5053445, 0.22),
  // The desktop's ::highlight(mdow-search) colours.
  findMatch: '#ffe5ab',
  findActive: '#96c5ff',
  syntax: githubSyntax('#24292f', '#6e7781', '#cf222e', '#0a3069', '#8250df', '#0550ae'),
  alerts: {
    note: '#0969da',
    tip: '#1a7f37',
    important: '#8250df',
    warning: '#9a6700',
    caution: '#cf222e',
  },
  diagram: {
    paper: '#ffffff',
    paperWarm: '#faf9f5',
    ink: '#3b3b3b',
    inkMuted: '#5e5e58',
    line: '#d8d7d0',
  },
}

export const DARK: Theme = {
  scheme: 'dark',
  background: hsla(0, 0, 0.03545248),
  foreground: hsla(0, 0, 0.8955769),
  muted: hsla(0, 0, 0.07734101),
  mutedForeground: hsla(0, 0, 0.56073545),
  primary: hsla(0.60397774, 0.86814313, 0.6641645),
  accent: hsla(0.1145878, 0.7915325, 0.48821926),
  destructive: hsla(0.9978404, 0.7151559, 0.5523152),
  border: hsla(0, 0, 0.15033225),
  borderSubtle: hsla(0, 0, 0.10395742),
  sidebar: hsla(0, 0, 0.03545248),
  sidebarAccent: hsla(0, 0, 0.0861042),
  surfaceRaised: hsla(0, 0, 0.09),
  surfaceWell: hsla(0, 0, 0.06),
  selection: hsla(0.60397774, 0.86814313, 0.6641645, 0.28),
  findMatch: '#6a5118',
  findActive: '#2a5397',
  syntax: githubSyntax('#e6edf3', '#8b949e', '#ff7b72', '#a5d6ff', '#d2a8ff', '#79c0ff'),
  alerts: {
    note: '#4493f8',
    tip: '#3fb950',
    important: '#ab7df8',
    warning: '#d29922',
    caution: '#f85149',
  },
  diagram: {
    paper: '#141414',
    paperWarm: '#1c1c1c',
    ink: '#e4e4e4',
    inkMuted: '#a6a6a6',
    line: '#3d3d3d',
  },
}

export function themeFor(scheme: ColorScheme) {
  return scheme === 'dark' ? DARK : LIGHT
}

export interface ReaderMetrics {
  fontSize: number
  lineHeight: number
  contentFont: string
  codeFont: string
}

/** Desktop heading scale (em of the body size) and line heights, h1 to h6. */
export const HEADING_SCALE = [1.875, 1.5, 1.15, 1, 0.95, 0.875]
const HEADING_LINE = [1.2, 1.25, 1.3, 1.4, 1.4, 1.4]

/**
 * The native text-component theme: markdown, code and inputs share it.
 *
 * gpuix paints list markers and the blockquote bar in `accent` and link underlines in
 * `textMuted`, so those carry the desktop's muted markers and blue links.
 */
export function nativeTheme(theme: Theme, reader: ReaderMetrics): GpuixTheme {
  const { fontSize, lineHeight } = reader
  const codeSize = Math.round(fontSize * 0.875 * 100) / 100
  return {
    appearance: theme.scheme,
    bg: theme.background,
    border: theme.border,
    text: theme.foreground,
    textMuted: theme.primary,
    textFaint: theme.mutedForeground,
    textDim: theme.mutedForeground,
    accent: theme.mutedForeground,
    caret: theme.primary,
    codeText: theme.foreground,
    codeWash: theme.muted,
    fontSans: reader.contentFont,
    fontMono: reader.codeFont,
    syntax: theme.syntax,
    metrics: {
      mdTextSize: fontSize,
      mdLineHeight: Math.round(fontSize * lineHeight * 10) / 10,
      mdBlockGap: Math.round(fontSize * 0.35),
      mdHeadingSizes: HEADING_SCALE.map((scale) => Math.round(fontSize * scale * 10) / 10),
      mdHeadingLineHeights: HEADING_SCALE.map(
        (scale, index) => Math.round(fontSize * scale * HEADING_LINE[index]! * 10) / 10,
      ),
      mdInlineCodeRadius: 4,
      mdCodeRadius: 10,
      codeTextSize: codeSize,
      codeLineHeight: Math.round(codeSize * 1.6 * 10) / 10,
    },
  }
}
