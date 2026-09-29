import type { EventPayload } from '@gpuix/react'

export type CommandId =
  | 'open-file'
  | 'open-folder'
  | 'close-tab'
  | 'next-tab'
  | 'previous-tab'
  | 'toggle-sidebar'
  | 'sidebar-recents'
  | 'sidebar-folder'
  | 'sidebar-outline'
  | 'toggle-wide-mode'
  | 'column-standard'
  | 'column-comfortable'
  | 'column-wide'
  | 'theme-system'
  | 'theme-light'
  | 'theme-dark'
  | 'zoom-in'
  | 'zoom-out'
  | 'zoom-reset'
  | 'find'
  | 'find-next'
  | 'find-previous'
  | 'command-palette'
  | 'settings'
  | 'shortcuts'
  | 'check-for-updates'
  | 'dismiss'

export interface CommandSpec {
  id: CommandId
  title: string
  keys?: string
  /** Shown in the palette. Palette-internal and navigation commands stay out. */
  palette?: boolean
}

export const COMMANDS: CommandSpec[] = [
  { id: 'open-file', title: 'Open File', keys: '⌘O', palette: true },
  { id: 'open-folder', title: 'Open Folder', keys: '⇧⌘O', palette: true },
  { id: 'close-tab', title: 'Close Tab', keys: '⌘W', palette: true },
  { id: 'next-tab', title: 'Next Tab', keys: '⌃⇥' },
  { id: 'previous-tab', title: 'Previous Tab', keys: '⌃⇧⇥' },
  { id: 'toggle-sidebar', title: 'Toggle Sidebar', keys: '⌘B', palette: true },
  { id: 'sidebar-recents', title: 'Sidebar: Recents', keys: '⌃1', palette: true },
  { id: 'sidebar-folder', title: 'Sidebar: Folder', keys: '⌃2', palette: true },
  { id: 'sidebar-outline', title: 'Sidebar: Outline', keys: '⌃3', palette: true },
  { id: 'toggle-wide-mode', title: 'Toggle Wide Mode', keys: '⇧⌘W', palette: true },
  { id: 'column-standard', title: 'Reading Width: Standard', palette: true },
  { id: 'column-comfortable', title: 'Reading Width: Comfortable', palette: true },
  { id: 'column-wide', title: 'Reading Width: Wide', palette: true },
  { id: 'theme-system', title: 'Theme: System', palette: true },
  { id: 'theme-light', title: 'Theme: Light', palette: true },
  { id: 'theme-dark', title: 'Theme: Dark', palette: true },
  { id: 'zoom-in', title: 'Zoom In', keys: '⌘=', palette: true },
  { id: 'zoom-out', title: 'Zoom Out', keys: '⌘-', palette: true },
  { id: 'zoom-reset', title: 'Actual Size', keys: '⌘0', palette: true },
  { id: 'find', title: 'Find…', keys: '⌘F', palette: true },
  { id: 'find-next', title: 'Find Next', keys: '⌘G' },
  { id: 'find-previous', title: 'Find Previous', keys: '⇧⌘G' },
  { id: 'command-palette', title: 'Command Palette', keys: '⌘K' },
  { id: 'settings', title: 'Settings…', keys: '⌘,', palette: true },
  { id: 'shortcuts', title: 'Keyboard Shortcuts', keys: '⌘/', palette: true },
  { id: 'check-for-updates', title: 'Check for Updates…', palette: true },
  { id: 'dismiss', title: 'Dismiss', keys: 'Esc' },
]

export const COMMAND_BY_ID = new Map(COMMANDS.map((command) => [command.id, command]))

interface Binding {
  key: string
  cmd?: boolean
  shift?: boolean
  ctrl?: boolean
  alt?: boolean
}

const BINDINGS: [Binding, CommandId][] = [
  [{ key: 'o', cmd: true }, 'open-file'],
  [{ key: 'o', cmd: true, shift: true }, 'open-folder'],
  [{ key: 'w', cmd: true }, 'close-tab'],
  [{ key: 'w', cmd: true, shift: true }, 'toggle-wide-mode'],
  [{ key: 'tab', ctrl: true }, 'next-tab'],
  [{ key: 'tab', ctrl: true, shift: true }, 'previous-tab'],
  [{ key: ']', cmd: true, shift: true }, 'next-tab'],
  [{ key: '[', cmd: true, shift: true }, 'previous-tab'],
  [{ key: 'b', cmd: true }, 'toggle-sidebar'],
  [{ key: '1', ctrl: true }, 'sidebar-recents'],
  [{ key: '2', ctrl: true }, 'sidebar-folder'],
  [{ key: '3', ctrl: true }, 'sidebar-outline'],
  [{ key: '=', cmd: true }, 'zoom-in'],
  [{ key: '=', cmd: true, shift: true }, 'zoom-in'],
  [{ key: '+', cmd: true }, 'zoom-in'],
  [{ key: '+', cmd: true, shift: true }, 'zoom-in'],
  [{ key: '-', cmd: true }, 'zoom-out'],
  [{ key: '0', cmd: true }, 'zoom-reset'],
  [{ key: 'f', cmd: true }, 'find'],
  [{ key: 'g', cmd: true }, 'find-next'],
  [{ key: 'g', cmd: true, shift: true }, 'find-previous'],
  [{ key: 'k', cmd: true }, 'command-palette'],
  [{ key: 'p', cmd: true, shift: true }, 'command-palette'],
  [{ key: ',', cmd: true }, 'settings'],
  [{ key: '/', cmd: true }, 'shortcuts'],
  [{ key: 'escape' }, 'dismiss'],
]

/** Primary modifier: ⌘ on macOS, Ctrl elsewhere (Ctrl bindings stay Ctrl everywhere). */
const PRIMARY_IS_CTRL = process.platform !== 'darwin'

export function commandForKey(event: Pick<EventPayload, 'key' | 'modifiers'>): CommandId | null {
  const key = event.key?.toLowerCase()
  if (!key) return null
  const mods = event.modifiers ?? { cmd: false, ctrl: false, shift: false, alt: false }
  for (const [binding, command] of BINDINGS) {
    if (binding.key !== key) continue
    if (PRIMARY_IS_CTRL) {
      if (!!(binding.cmd || binding.ctrl) !== mods.ctrl || mods.cmd) continue
    } else if (!!binding.cmd !== mods.cmd || !!binding.ctrl !== mods.ctrl) continue
    if (!!binding.shift !== mods.shift) continue
    if (!!binding.alt !== mods.alt) continue
    return command
  }
  return null
}

export function displayKeys(keys: string) {
  if (!PRIMARY_IS_CTRL) return keys
  return keys.replace('⌘', 'Ctrl+').replace('⇧', 'Shift+').replace('⌃', 'Ctrl+')
}

/** Subsequence match with bonuses for runs and a matching first character. */
export function subsequenceScore(query: string, candidate: string): number | null {
  const needle = query.toLowerCase()
  const haystack = candidate.toLowerCase()
  let score = 0
  let from = 0
  let run = 0
  for (const char of needle) {
    const pos = haystack.indexOf(char, from)
    if (pos === -1) return null
    if (pos === from) {
      run++
      score += 8 + run
    } else {
      run = 0
      score += 1
    }
    if (from === 0 && pos === 0) score += 12
    from = pos + char.length
  }
  return score
}
