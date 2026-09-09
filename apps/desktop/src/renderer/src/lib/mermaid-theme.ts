export type MermaidPaletteId = 'light' | 'dark' | 'fog-pine'

export interface MermaidTokens {
  paper: string
  paperWarm: string
  ink: string
  muted: string
  line: string
  teal: string
  fontFamily: string
  fontSize: string
  radius: string
}

export const MERMAID_DIAGRAM_FONT = "'IBM Plex Mono', 'Geist Mono', ui-monospace, monospace"

const SHARED_TYPE: Pick<MermaidTokens, 'fontFamily' | 'fontSize' | 'radius'> = {
  fontFamily: MERMAID_DIAGRAM_FONT,
  fontSize: '13.5px',
  radius: '6px',
}

/** Designer lock for the default mdow paper theme. */
export const MERMAID_PALETTES: Record<MermaidPaletteId, MermaidTokens> = {
  light: {
    paper: '#ffffff',
    paperWarm: '#faf9f5',
    ink: '#3b3b3b',
    muted: '#5e5e58',
    line: '#d8d7d0',
    teal: '#0f8080',
    ...SHARED_TYPE,
  },
  'fog-pine': {
    paper: '#f1f3ed',
    paperWarm: '#e7eae2',
    ink: '#1d2620',
    muted: '#4e5c55',
    line: '#9aa399',
    teal: '#2b5c55',
    ...SHARED_TYPE,
  },
  dark: {
    paper: '#141414',
    paperWarm: '#1c1c1c',
    ink: '#e4e4e4',
    muted: '#a6a6a6',
    line: '#3d3d3d',
    teal: '#5a9e98',
    ...SHARED_TYPE,
  },
}

export const MERMAID_CSS_VARS = {
  paper: '--md-paper',
  paperWarm: '--md-paper-warm',
  ink: '--md-ink',
  muted: '--md-ink-muted',
  line: '--md-line',
  teal: '--md-teal',
  fontFamily: '--md-diagram-font',
  fontSize: '--md-diagram-font-size',
  radius: '--md-diagram-radius',
} as const

export function getMermaidPaletteId(
  root: HTMLElement = document.documentElement,
): MermaidPaletteId {
  const palette = root.dataset.palette ?? root.dataset.theme
  if (palette === 'fog-pine') return 'fog-pine'
  if (root.classList.contains('dark')) return 'dark'
  return 'light'
}

function readCssToken(name: string, fallback: string, root: HTMLElement): string {
  if (typeof getComputedStyle !== 'function') return fallback
  const value = getComputedStyle(root).getPropertyValue(name).trim()
  return value.length > 0 ? value : fallback
}

export function readMermaidTokens(
  paletteId: MermaidPaletteId = getMermaidPaletteId(),
  root: HTMLElement = document.documentElement,
): MermaidTokens {
  const fallback = MERMAID_PALETTES[paletteId]
  return {
    paper: readCssToken(MERMAID_CSS_VARS.paper, fallback.paper, root),
    paperWarm: readCssToken(MERMAID_CSS_VARS.paperWarm, fallback.paperWarm, root),
    ink: readCssToken(MERMAID_CSS_VARS.ink, fallback.ink, root),
    muted: readCssToken(MERMAID_CSS_VARS.muted, fallback.muted, root),
    line: readCssToken(MERMAID_CSS_VARS.line, fallback.line, root),
    teal: readCssToken(MERMAID_CSS_VARS.teal, fallback.teal, root),
    fontFamily: readCssToken(MERMAID_CSS_VARS.fontFamily, fallback.fontFamily, root),
    fontSize: readCssToken(MERMAID_CSS_VARS.fontSize, fallback.fontSize, root),
    radius: readCssToken(MERMAID_CSS_VARS.radius, fallback.radius, root),
  }
}

export function getMermaidThemeVariables(tokens: MermaidTokens): Record<string, string> {
  const { paper, paperWarm, ink, muted, line, teal, fontFamily, fontSize } = tokens
  return {
    background: 'transparent',
    fontFamily,
    fontSize,
    textColor: ink,
    lineColor: muted,
    primaryColor: paperWarm,
    primaryTextColor: ink,
    primaryBorderColor: line,
    secondaryColor: paperWarm,
    secondaryTextColor: ink,
    secondaryBorderColor: line,
    tertiaryColor: paper,
    tertiaryTextColor: ink,
    tertiaryBorderColor: line,
    mainBkg: paperWarm,
    secondBkg: paper,
    nodeBorder: line,
    clusterBkg: 'transparent',
    clusterBorder: line,
    titleColor: ink,
    edgeLabelBackground: paper,
    nodeTextColor: ink,
    actorBorder: line,
    actorBkg: paperWarm,
    actorTextColor: ink,
    actorLineColor: muted,
    signalColor: muted,
    signalTextColor: ink,
    labelBoxBkgColor: paperWarm,
    labelBoxBorderColor: line,
    labelTextColor: ink,
    loopTextColor: ink,
    noteBorderColor: line,
    noteBkgColor: paperWarm,
    noteTextColor: ink,
    activationBorderColor: line,
    activationBkgColor: paper,
    sequenceNumberColor: ink,
    sectionBkgColor: paperWarm,
    altSectionBkgColor: paper,
    gridColor: line,
    cScale0: paperWarm,
    cScale1: teal,
    cScale2: paperWarm,
    pie1: teal,
    pie2: paperWarm,
    pie3: paperWarm,
    pie4: paperWarm,
    pie5: paperWarm,
    pie6: paperWarm,
    pie7: paperWarm,
    pie8: paperWarm,
    pie9: paperWarm,
    pie10: paperWarm,
    pie11: paperWarm,
    pie12: paperWarm,
    errorBkgColor: paperWarm,
    errorTextColor: ink,
  }
}

export function getMermaidThemeCSS(tokens: MermaidTokens): string {
  const { paperWarm, ink, muted, line, teal, fontFamily, fontSize, radius } = tokens
  return `
    svg { background: transparent !important; }
    .node rect, .node polygon, .node circle, .node ellipse, .node path,
    .actor, .actor-box, .classGroup rect, .labelBox, .note, .note rect,
    .statediagram-state rect, .er.entityBox, .requirement, .quoted {
      fill: ${paperWarm} !important;
      stroke: ${line} !important;
      stroke-width: 1px !important;
      filter: none !important;
    }
    .node rect, .actor, .classGroup rect, .labelBox, .note rect,
    .statediagram-state rect, .er.entityBox {
      rx: ${radius} !important;
      ry: ${radius} !important;
    }
    .edgePath .path, .flowchart-link, .messageLine0, .messageLine1,
    .transition, .relation, .relationLine, .loopLine {
      stroke: ${muted} !important;
      stroke-width: 1px !important;
      fill: none !important;
    }
    marker path, .arrowheadPath, .marker {
      fill: ${muted} !important;
      stroke: ${muted} !important;
    }
    .label, .nodeLabel, .edgeLabel, .actor, .messageText, .loopText, .noteText,
    .titleText, text {
      font-family: ${fontFamily} !important;
      font-size: ${fontSize} !important;
      fill: ${ink} !important;
      color: ${ink} !important;
    }
    .cluster rect { fill: transparent !important; stroke: ${line} !important; }
    .node.selected rect, .node.selected polygon, .node.selected circle,
    .node:hover > rect, .node:hover > polygon, .node:hover > circle,
    .node:hover > ellipse, .node:focus-within > rect {
      stroke: ${teal} !important;
    }
  `
}

export interface MermaidInitConfig {
  startOnLoad: false
  securityLevel: 'loose'
  theme: 'base'
  darkMode: boolean
  fontFamily: string
  themeVariables: Record<string, string>
  themeCSS: string
  flowchart: {
    htmlLabels: false
    curve: 'linear'
    padding: number
    wrappingWidth: number
    useMaxWidth: true
  }
}

export function getMermaidInitConfig(
  paletteId: MermaidPaletteId = getMermaidPaletteId(),
  tokens: MermaidTokens = readMermaidTokens(paletteId),
): MermaidInitConfig {
  return {
    startOnLoad: false,
    securityLevel: 'loose',
    theme: 'base',
    darkMode: paletteId === 'dark',
    fontFamily: tokens.fontFamily,
    themeVariables: getMermaidThemeVariables(tokens),
    themeCSS: getMermaidThemeCSS(tokens),
    flowchart: {
      htmlLabels: false,
      curve: 'linear',
      padding: 12,
      wrappingWidth: 200,
      useMaxWidth: true,
    },
  }
}

export function resolveMermaidPaletteId(isDark?: boolean): MermaidPaletteId {
  if (isDark === true) return 'dark'
  if (isDark === false) return 'light'
  return getMermaidPaletteId()
}
