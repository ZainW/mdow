import { describe, expect, it } from 'vitest'
import { cn, formatShortcut } from './utils'

describe('cn', () => {
  it('merges class names', () => {
    expect(cn('foo', 'bar')).toBe('foo bar')
  })

  it('handles conditional classes', () => {
    const condition = false
    expect(cn('foo', condition && 'bar', 'baz')).toBe('foo baz')
  })

  it('deduplicates tailwind conflicts', () => {
    expect(cn('p-4', 'p-2')).toBe('p-2')
  })
})

describe('formatShortcut', () => {
  it('uses macOS symbols on Mac', () => {
    expect(formatShortcut('O', { mac: true })).toBe('⌘O')
    expect(formatShortcut('O', { shift: true, mac: true })).toBe('⇧⌘O')
  })

  it('spells out Ctrl on Windows and Linux', () => {
    expect(formatShortcut('O', { mac: false })).toBe('Ctrl+O')
    expect(formatShortcut('O', { shift: true, mac: false })).toBe('Ctrl+Shift+O')
  })
})
