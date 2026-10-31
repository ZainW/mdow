import { describe, expect, it } from 'vitest'
import { detectPlatform } from './download-links'

describe('detectPlatform', () => {
  it('detects macOS', () => {
    expect(detectPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X)')).toBe('mac')
  })

  it('detects Linux', () => {
    expect(detectPlatform('Mozilla/5.0 (X11; Linux x86_64)')).toBe('linux')
  })

  it('defaults to macOS', () => {
    expect(detectPlatform('Mozilla/5.0 (Windows NT 10.0)')).toBe('mac')
  })
})
