import { describe, expect, it } from 'vitest'
import { getContentFontFamily, getCodeFontFamily } from './SettingsDialog'

describe('getContentFontFamily', () => {
  it('returns Inter family for "inter"', () => {
    expect(getContentFontFamily('inter')).toContain('Inter')
  })

  it('returns Charter family for "charter"', () => {
    expect(getContentFontFamily('charter')).toContain('Charter')
  })

  it('returns system-ui family for "system-sans"', () => {
    expect(getContentFontFamily('system-sans')).toContain('system-ui')
  })

  it('returns Georgia family for "georgia"', () => {
    expect(getContentFontFamily('georgia')).toContain('Georgia')
  })

  it('returns default (Inter) for unknown value', () => {
    expect(getContentFontFamily('unknown-font')).toContain('Inter')
  })
})

describe('getCodeFontFamily', () => {
  it('returns Geist Mono family for "geist-mono"', () => {
    expect(getCodeFontFamily('geist-mono')).toContain('Geist Mono')
  })

  it('returns SF Mono family for "sf-mono"', () => {
    expect(getCodeFontFamily('sf-mono')).toContain('SF Mono')
  })

  it('returns JetBrains Mono family for "jetbrains-mono"', () => {
    expect(getCodeFontFamily('jetbrains-mono')).toContain('JetBrains Mono')
  })

  it('returns default (Geist Mono) for unknown value', () => {
    expect(getCodeFontFamily('unknown-font')).toContain('Geist Mono')
  })
})
