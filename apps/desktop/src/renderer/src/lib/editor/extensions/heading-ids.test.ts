import { describe, it, expect } from 'vitest'
import { slugifyHeading, dedupeSlug } from './heading-ids'

describe('heading slug helpers', () => {
  it('slugifies basic text', () => {
    expect(slugifyHeading('Hello World')).toBe('hello-world')
  })

  it('strips punctuation', () => {
    expect(slugifyHeading("What's up?")).toBe('whats-up')
  })

  it('dedupes repeated slugs', () => {
    const counts = new Map<string, number>()
    expect(dedupeSlug('intro', counts)).toBe('intro')
    expect(dedupeSlug('intro', counts)).toBe('intro-1')
    expect(dedupeSlug('intro', counts)).toBe('intro-2')
  })
})
