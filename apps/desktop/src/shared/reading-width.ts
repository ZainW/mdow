export const READING_WIDTH_VALUES = ['narrow', 'medium', 'wide', 'full'] as const
export type ReadingWidth = (typeof READING_WIDTH_VALUES)[number]

export const DEFAULT_READING_WIDTH: ReadingWidth = 'medium'

export function isReadingWidth(value: unknown): value is ReadingWidth {
  return typeof value === 'string' && READING_WIDTH_VALUES.some((width) => width === value)
}

// Before v1.11 the column width had three presets and "full width" was a separate wide-mode
// toggle. Both fold into the single Line width setting.
const LEGACY_READING_WIDTHS: Record<string, ReadingWidth> = {
  standard: 'medium',
  comfortable: 'wide',
}

/** Map a persisted width (current or legacy) plus the legacy wide-mode flag to a Line width. */
export function migrateReadingWidth(stored: unknown, legacyWideMode?: unknown): ReadingWidth {
  if (legacyWideMode === true) return 'full'
  if (isReadingWidth(stored)) return stored
  if (typeof stored === 'string' && stored in LEGACY_READING_WIDTHS) {
    return LEGACY_READING_WIDTHS[stored]
  }
  return DEFAULT_READING_WIDTH
}
