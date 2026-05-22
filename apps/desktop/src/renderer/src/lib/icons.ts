/**
 * Icon tokens — chosen via perf/icon-visual-compare.mjs (Electron + Playwright).
 *
 * Context          | Size | Stroke
 * -----------------|------|--------
 * Breadcrumb       | xs   | default
 * Tab close, banner| sm   | emphasis
 * Rail, tabs, etc. | md   | default
 * Default button   | lg   | default
 * Zoom controls    | md   | emphasis
 * Settings active  | md   | default + fill
 */
export const iconSize = {
  xs: 10,
  sm: 12,
  md: 14,
  lg: 16,
} as const

export const iconStroke = {
  default: 2,
  emphasis: 2.5,
} as const

export function iconActiveProps(active: boolean) {
  return {
    strokeWidth: iconStroke.default,
    fill: active ? 'currentColor' : 'none',
  } as const
}
