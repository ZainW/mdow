import { describe, expect, it } from 'vitest'
import { basename, isMarkdownPath, shortenPath, truncatePathMiddle } from './path-utils'

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

describe('truncatePathMiddle', () => {
  it('returns short paths unchanged', () => {
    expect(truncatePathMiddle('/a/b/file.md', 56)).toBe('/a/b/file.md')
  })

  it('collapses middle segments on a long path', () => {
    const result = truncatePathMiddle('/Users/zain/projects/mdow/apps/desktop/src/file.md', 32)
    expect(result.startsWith('Users/')).toBe(true)
    expect(result.endsWith('/file.md')).toBe(true)
    expect(result).toContain('…')
  })

  it('hard-truncates a basename that exceeds maxLen on its own', () => {
    const long = 'x'.repeat(120)
    const result = truncatePathMiddle(`/a/${long}`, 24)
    expect(result.length).toBeLessThanOrEqual(24)
    expect(result.startsWith('…/')).toBe(true)
  })

  it('keeps short paths unchanged even when they have two segments', () => {
    expect(truncatePathMiddle('/foo/bar', 56)).toBe('/foo/bar')
  })

  it('honours windows separators', () => {
    const result = truncatePathMiddle('C:\\Users\\zain\\projects\\mdow\\apps\\file.md', 24)
    expect(result).toContain('\\…\\')
    expect(result.endsWith('\\file.md')).toBe(true)
  })
})

describe('isMarkdownPath', () => {
  it('accepts supported markdown extensions case-insensitively', () => {
    expect(isMarkdownPath('readme.md')).toBe(true)
    expect(isMarkdownPath('notes.Markdown')).toBe(true)
    expect(isMarkdownPath('doc.MDX')).toBe(true)
  })

  it('rejects unsupported extensions', () => {
    expect(isMarkdownPath('readme.txt')).toBe(false)
    expect(isMarkdownPath('markdown.json')).toBe(false)
  })
})
