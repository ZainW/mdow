import { describe, expect, test } from 'bun:test'
import { commandForKey, subsequenceScore } from './commands'

const mods = (partial: Partial<Record<'cmd' | 'ctrl' | 'shift' | 'alt', boolean>> = {}) => ({
  cmd: false,
  ctrl: false,
  shift: false,
  alt: false,
  ...partial,
})
const primary = process.platform === 'darwin' ? 'cmd' : 'ctrl'

describe('commandForKey', () => {
  test('maps primary-modifier shortcuts', () => {
    expect(commandForKey({ key: 'o', modifiers: mods({ [primary]: true }) })).toBe('open-file')
    expect(commandForKey({ key: 'o', modifiers: mods({ [primary]: true, shift: true }) })).toBe(
      'open-folder',
    )
    expect(commandForKey({ key: 'w', modifiers: mods({ [primary]: true }) })).toBe('close-tab')
    expect(commandForKey({ key: 'W', modifiers: mods({ [primary]: true, shift: true }) })).toBe(
      'toggle-wide-mode',
    )
  })

  test('maps Ctrl sidebar modes and Escape', () => {
    expect(commandForKey({ key: '2', modifiers: mods({ ctrl: true }) })).toBe('sidebar-folder')
    expect(commandForKey({ key: 'escape', modifiers: mods() })).toBe('dismiss')
  })

  test('ignores plain keys and extra modifiers', () => {
    expect(commandForKey({ key: 'o', modifiers: mods() })).toBeNull()
    expect(commandForKey({ key: 'o', modifiers: mods({ [primary]: true, alt: true }) })).toBeNull()
  })
})

describe('subsequenceScore', () => {
  test('rejects non-subsequences', () => {
    expect(subsequenceScore('xyz', 'Open File')).toBeNull()
  })

  test('prefers prefix and contiguous matches', () => {
    const prefix = subsequenceScore('open', 'Open File')!
    const scattered = subsequenceScore('ofe', 'Open File')!
    expect(prefix).toBeGreaterThan(scattered)
    expect(subsequenceScore('zoom', 'Zoom In')!).toBeGreaterThan(
      subsequenceScore('zoom', 'Actual Zoom')!,
    )
  })
})
