import { describe, expect, it } from 'vitest'
import { basename, shortenPath } from './path-utils'

describe('basename', () => {
  it('extracts filename from unix path', () => {
    expect(basename('/Users/zain/docs/readme.md')).toBe('readme.md')
  })

  it('extracts filename from windows path', () => {
    expect(basename('C:\\Users\\zain\\docs\\readme.md')).toBe('readme.md')
  })

  it('handles mixed separators', () => {
    expect(basename('/Users/zain\\docs/readme.md')).toBe('readme.md')
  })

  it('returns the string itself when no separator', () => {
    expect(basename('readme.md')).toBe('readme.md')
  })

  it('handles trailing separator by returning fallback', () => {
    // pop() returns '' which is falsy, so falls back to original path
    expect(basename('/Users/zain/docs/')).toBe('/Users/zain/docs/')
  })

  it('handles single segment path', () => {
    expect(basename('file.txt')).toBe('file.txt')
  })

  it('handles empty string', () => {
    expect(basename('')).toBe('')
  })
})

describe('shortenPath', () => {
  it('returns path unchanged when within maxLen', () => {
    const short = '/Users/docs/file.md'
    expect(shortenPath(short, 40)).toBe(short)
  })

  it('truncates long path to last two segments', () => {
    const long = '/Users/zain/projects/very-long-directory-name/docs/readme.md'
    expect(shortenPath(long, 40)).toBe('.../docs/readme.md')
  })

  it('uses default maxLen of 40', () => {
    const short = '/Users/docs/file.md'
    expect(shortenPath(short)).toBe(short)

    const long = '/Users/zain/projects/some-really-long-path/deeply/nested/file.md'
    expect(shortenPath(long)).toBe('.../nested/file.md')
  })

  it('returns path unchanged when 2 or fewer segments', () => {
    const twoSegment = 'a/' + 'x'.repeat(50)
    expect(shortenPath(twoSegment, 10)).toBe(twoSegment)
  })

  it('handles path exactly at maxLen boundary', () => {
    const exact = 'a'.repeat(40)
    expect(shortenPath(exact, 40)).toBe(exact)
  })

  it('handles path one char over maxLen', () => {
    // This is a single segment so it stays unchanged (<=2 parts)
    const over = 'a'.repeat(41)
    expect(shortenPath(over, 40)).toBe(over)
  })

  it('handles windows-style paths', () => {
    const long = 'C:\\Users\\zain\\projects\\very-long-directory-name\\docs\\readme.md'
    // split on /[\\/]/ produces multiple segments
    expect(shortenPath(long, 20)).toBe('.../docs/readme.md')
  })
})
