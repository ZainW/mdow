import { describe, expect, it } from 'vitest'
import { formatShortcut } from '../use-mod-key'

describe('formatShortcut', () => {
  it('joins mod key and letter', () => {
    expect(formatShortcut('⌘', 'K')).toBe('⌘K')
    expect(formatShortcut('Ctrl', 'F')).toBe('CtrlF')
  })

  it('returns key alone when mod is empty', () => {
    expect(formatShortcut('', 'K')).toBe('K')
  })
})
