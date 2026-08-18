import { describe, expect, it } from 'vitest'
import { cheatSheetColumns, isCheatSheetHoldKey } from './cheat-sheet'
import { createMinimalWindowApi, stubWindowApi } from '@renderer/test/stubWindowApi'

stubWindowApi(() => createMinimalWindowApi({ platform: 'linux' }))

describe('cheatSheetColumns', () => {
  it('uses Command glyphs on macOS', () => {
    const navigation = cheatSheetColumns(true)[0].sections[0]
    expect(navigation.items[0].keys).toEqual(['⌘', 'K'])
  })

  it('uses Ctrl labels on Windows and Linux', () => {
    const navigation = cheatSheetColumns(false)[0].sections[0]
    expect(navigation.items[0].keys).toEqual(['Ctrl', 'K'])
    expect(cheatSheetColumns(false)[1].sections[0].items[4].keys).toEqual(['F11'])
  })
})

describe('isCheatSheetHoldKey', () => {
  it('ignores repeat, alt, and shift', () => {
    expect(
      isCheatSheetHoldKey(
        new KeyboardEvent('keydown', { key: 'Control', ctrlKey: true, repeat: true }),
      ),
    ).toBe(false)
    expect(
      isCheatSheetHoldKey(
        new KeyboardEvent('keydown', { key: 'Control', ctrlKey: true, altKey: true }),
      ),
    ).toBe(false)
  })
})
