import { describe, expect, it } from 'vitest'
import { fuzzySearch } from './fuzzy-search'

const items = [
  { path: 'src/components/App.tsx', name: 'App.tsx' },
  { path: 'src/lib/utils.ts', name: 'utils.ts' },
  { path: 'src/components/Sidebar.tsx', name: 'Sidebar.tsx' },
  { path: 'docs/readme.md', name: 'readme.md' },
  { path: 'src/main/folder-service.ts', name: 'folder-service.ts' },
  { path: 'src/renderer/src/lib/fuzzy-search.ts', name: 'fuzzy-search.ts' },
  { path: 'package.json', name: 'package.json' },
]

describe('fuzzySearch', () => {
  describe('basic matching', () => {
    it('returns all items (up to maxResults) for empty query', () => {
      const results = fuzzySearch('', items)
      expect(results).toHaveLength(items.length)
      results.forEach((r) => expect(r.score).toBe(0))
    })

    it('returns all items for whitespace-only query', () => {
      const results = fuzzySearch('   ', items)
      expect(results).toHaveLength(items.length)
    })

    it('returns empty array when nothing matches', () => {
      const results = fuzzySearch('zzzzz', items)
      expect(results).toHaveLength(0)
    })

    it('matches exact filename', () => {
      const results = fuzzySearch('utils.ts', items)
      expect(results.length).toBeGreaterThan(0)
      expect(results[0].name).toBe('utils.ts')
    })

    it('matches partial filename', () => {
      const results = fuzzySearch('util', items)
      expect(results.length).toBeGreaterThan(0)
      expect(results[0].name).toBe('utils.ts')
    })
  })

  describe('case insensitivity', () => {
    it('matches regardless of case', () => {
      const upper = fuzzySearch('APP', items)
      const lower = fuzzySearch('app', items)
      expect(upper.length).toBeGreaterThan(0)
      expect(upper[0].name).toBe(lower[0].name)
    })
  })

  describe('scoring and ranking', () => {
    it('ranks exact name match above path-only match', () => {
      const results = fuzzySearch('App', items)
      expect(results[0].name).toBe('App.tsx')
    })

    it('ranks consecutive character matches higher', () => {
      const results = fuzzySearch('side', items)
      expect(results[0].name).toBe('Sidebar.tsx')
    })

    it('gives bonus for matches at path boundaries', () => {
      // "folder" starts after a / boundary in "folder-service.ts"
      const results = fuzzySearch('folder', items)
      expect(results[0].name).toBe('folder-service.ts')
    })

    it('gives bonus for matches at word boundaries (dash, underscore, dot)', () => {
      // "service" starts after a - boundary
      const results = fuzzySearch('service', items)
      expect(results[0].name).toBe('folder-service.ts')
    })

    it('scores are always positive for matches', () => {
      const results = fuzzySearch('s', items)
      results.forEach((r) => expect(r.score).toBeGreaterThan(0))
    })
  })

  describe('result shape', () => {
    it('returns SearchResult objects with path, name, and score', () => {
      const results = fuzzySearch('app', items)
      expect(results[0]).toHaveProperty('path')
      expect(results[0]).toHaveProperty('name')
      expect(results[0]).toHaveProperty('score')
    })

    it('results are sorted by score descending', () => {
      const results = fuzzySearch('s', items)
      for (let i = 1; i < results.length; i++) {
        expect(results[i - 1].score).toBeGreaterThanOrEqual(results[i].score)
      }
    })
  })

  describe('maxResults', () => {
    it('limits results to maxResults', () => {
      const results = fuzzySearch('s', items, 2)
      expect(results).toHaveLength(2)
    })

    it('returns fewer than maxResults if not enough matches', () => {
      const results = fuzzySearch('package', items, 50)
      expect(results.length).toBeLessThan(50)
    })

    it('defaults to 50 max results', () => {
      const manyItems = Array.from({ length: 100 }, (_, i) => ({
        path: `src/file${i}.ts`,
        name: `file${i}.ts`,
      }))
      const results = fuzzySearch('file', manyItems)
      expect(results).toHaveLength(50)
    })

    it('limits empty query results to maxResults', () => {
      const manyItems = Array.from({ length: 100 }, (_, i) => ({
        path: `src/file${i}.ts`,
        name: `file${i}.ts`,
      }))
      const results = fuzzySearch('', manyItems, 10)
      expect(results).toHaveLength(10)
    })
  })

  describe('edge cases', () => {
    it('handles single character query', () => {
      const results = fuzzySearch('r', items)
      expect(results.length).toBeGreaterThan(0)
    })

    it('handles items with empty names', () => {
      const withEmpty = [{ path: 'src/', name: '' }]
      const results = fuzzySearch('src', withEmpty)
      expect(results.length).toBeGreaterThan(0)
    })

    it('handles empty items array', () => {
      const results = fuzzySearch('test', [])
      expect(results).toHaveLength(0)
    })

    it('handles query longer than any target', () => {
      const results = fuzzySearch('this-is-a-very-long-query-that-wont-match', items)
      expect(results).toHaveLength(0)
    })

    it('preserves original item data in results', () => {
      const results = fuzzySearch('readme', items)
      const match = results.find((r) => r.name === 'readme.md')
      expect(match).toBeDefined()
      expect(match!.path).toBe('docs/readme.md')
    })
  })
})
