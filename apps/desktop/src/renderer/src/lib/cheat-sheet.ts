import { isMacPlatform } from './utils'

export const CHEAT_SHEET_HOLD_MS = 700

export interface CheatSheetRow {
  label: string
  keys: string[]
}

export interface CheatSheetSection {
  heading: string
  items: CheatSheetRow[]
}

export interface CheatSheetColumn {
  sections: CheatSheetSection[]
}

export function cheatSheetColumns(isMac: boolean): CheatSheetColumn[] {
  const mod = isMac ? '⌘' : 'Ctrl'
  const alt = isMac ? '⌥' : 'Alt'
  const fullScreen = isMac ? ['⌃', '⌘', 'F'] : ['F11']

  return [
    {
      sections: [
        {
          heading: 'Navigation',
          items: [
            { label: 'Command palette', keys: [mod, 'K'] },
            { label: 'Find in document', keys: [mod, 'F'] },
            { label: 'Switch tab', keys: [mod, '1–9'] },
            { label: 'Next tab', keys: [mod, alt, '→'] },
            { label: 'Previous tab', keys: [mod, alt, '←'] },
            { label: 'Close tab', keys: [mod, 'W'] },
          ],
        },
      ],
    },
    {
      sections: [
        {
          heading: 'View',
          items: [
            { label: 'Toggle sidebar', keys: [mod, 'B'] },
            { label: 'Zoom in', keys: [mod, '+'] },
            { label: 'Zoom out', keys: [mod, '−'] },
            { label: 'Reset zoom', keys: [mod, '0'] },
            { label: 'Full screen', keys: fullScreen },
            { label: 'All shortcuts', keys: [mod, '/'] },
          ],
        },
      ],
    },
    {
      sections: [
        {
          heading: 'Files',
          items: [
            { label: 'Open file', keys: [mod, 'O'] },
            { label: 'Open folder', keys: [mod, '⇧', 'O'] },
          ],
        },
        {
          heading: 'App',
          items: [{ label: 'Settings', keys: [mod, ','] }],
        },
      ],
    },
  ]
}

export function isCheatSheetHoldKey(event: KeyboardEvent): boolean {
  if (event.repeat || event.altKey || event.shiftKey) return false
  if (isMacPlatform()) return event.key === 'Meta' && !event.ctrlKey
  return event.key === 'Control' && !event.metaKey
}
