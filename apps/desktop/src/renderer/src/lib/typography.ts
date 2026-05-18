export const CONTENT_FONTS = [
  { value: 'inter', label: 'Inter', family: "'Inter', system-ui, -apple-system, sans-serif" },
  { value: 'charter', label: 'Charter', family: "Charter, 'Bitstream Charter', Georgia, serif" },
  {
    value: 'system-sans',
    label: 'System',
    family: 'system-ui, -apple-system, sans-serif',
  },
  { value: 'georgia', label: 'Georgia', family: "Georgia, 'Times New Roman', serif" },
] as const

export const CODE_FONTS = [
  { value: 'geist-mono', label: 'Geist', family: "'Geist Mono', ui-monospace, monospace" },
  {
    value: 'sf-mono',
    label: 'SF Mono',
    family: "'SF Mono', SFMono-Regular, ui-monospace, monospace",
  },
  {
    value: 'jetbrains-mono',
    label: 'JetBrains',
    family: "'JetBrains Mono', ui-monospace, monospace",
  },
] as const

export function getContentFontFamily(value: string): string {
  return CONTENT_FONTS.find((f) => f.value === value)?.family ?? CONTENT_FONTS[0].family
}

export function getCodeFontFamily(value: string): string {
  return CODE_FONTS.find((f) => f.value === value)?.family ?? CODE_FONTS[0].family
}
