export type ThemeMode = 'system' | 'light' | 'dark'
export type ContentFont = 'inter' | 'charter' | 'system-sans' | 'georgia'
export type CodeFont = 'geist-mono' | 'system-mono' | 'sf-mono' | 'jetbrains-mono'
export type InterfaceScale = 'compact' | 'comfortable' | 'large'
export type ColumnWidth = 'standard' | 'comfortable' | 'wide'
export type SidebarMode = 'recents' | 'folder' | 'outline'

export interface Prefs {
  theme: ThemeMode
  contentFont: ContentFont
  codeFont: CodeFont
  interfaceScale: InterfaceScale
  readingWidth: ColumnWidth
  wideMode: boolean
  zoomLevel: number
  sidebarMode: SidebarMode
}

export const DEFAULT_PREFS: Prefs = {
  theme: 'system',
  contentFont: 'inter',
  codeFont: 'geist-mono',
  interfaceScale: 'compact',
  readingWidth: 'standard',
  wideMode: false,
  zoomLevel: 100,
  sidebarMode: 'folder',
}

export const THEME_MODES: ThemeMode[] = ['system', 'light', 'dark']
export const CONTENT_FONTS: ContentFont[] = ['inter', 'charter', 'system-sans', 'georgia']
export const CODE_FONTS: CodeFont[] = ['geist-mono', 'system-mono', 'sf-mono', 'jetbrains-mono']
export const INTERFACE_SCALES: InterfaceScale[] = ['compact', 'comfortable', 'large']
export const COLUMN_WIDTHS: ColumnWidth[] = ['standard', 'comfortable', 'wide']
export const SIDEBAR_MODES: SidebarMode[] = ['recents', 'folder', 'outline']

export const READER_FONT_SIZE = 16
export const READER_LINE_HEIGHT = 1.75
export const ZOOM_MIN = 60
export const ZOOM_MAX = 200
export const ZOOM_STEP = 10

export const COLUMN_PX: Record<ColumnWidth, number> = {
  standard: 768,
  comfortable: 896,
  wide: 1088,
}

export const LABELS = {
  theme: { system: 'System', light: 'Light', dark: 'Dark' },
  contentFont: { inter: 'Inter', charter: 'Charter', 'system-sans': 'System', georgia: 'Georgia' },
  codeFont: {
    'geist-mono': 'Geist',
    'system-mono': 'System',
    'sf-mono': 'SF Mono',
    'jetbrains-mono': 'JetBrains',
  },
  interfaceScale: { compact: 'Compact', comfortable: 'Comfortable', large: 'Large' },
  readingWidth: { standard: 'Standard', comfortable: 'Comfortable', wide: 'Wide' },
  sidebarMode: { recents: 'Recents', folder: 'Folder', outline: 'Outline' },
} as const

const IS_MAC = process.platform === 'darwin'

export const CONTENT_FONT_FAMILY: Record<ContentFont, string> = {
  inter: 'Inter Variable',
  charter: 'Charter',
  'system-sans': IS_MAC ? '.SystemUIFont' : 'sans-serif',
  georgia: 'Georgia',
}

export const CODE_FONT_FAMILY: Record<CodeFont, string> = {
  'geist-mono': 'Geist Mono',
  'system-mono': IS_MAC ? 'Menlo' : 'monospace',
  'sf-mono': 'SF Mono',
  'jetbrains-mono': 'JetBrains Mono',
}

export interface ScaleTokens {
  controlFont: number
  controlXsFont: number
  buttonHeight: number
  buttonXsHeight: number
}

export const SCALE_TOKENS: Record<InterfaceScale, ScaleTokens> = {
  compact: { controlFont: 12, controlXsFont: 10, buttonHeight: 28, buttonXsHeight: 20 },
  comfortable: { controlFont: 13, controlXsFont: 11, buttonHeight: 32, buttonXsHeight: 24 },
  large: { controlFont: 14, controlXsFont: 12, buttonHeight: 36, buttonXsHeight: 28 },
}

export function clampZoom(percent: number) {
  const stepped = Math.round(percent / ZOOM_STEP) * ZOOM_STEP
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, stepped))
}

export function readerMaxWidth(prefs: Pick<Prefs, 'readingWidth' | 'wideMode'>): number | null {
  return prefs.wideMode ? null : COLUMN_PX[prefs.readingWidth]
}
