import { describe, expect, it } from 'vitest'
import { createDocumentResolver } from './document-resolver'

const docs = [
  '/work/README.md',
  '/work/docs/plan.md',
  '/work/docs/site/index.html',
  '/work/a/notes.md',
  '/work/b/notes.md',
]

describe('createDocumentResolver', () => {
  const resolve = createDocumentResolver(docs, '/work', '/work/docs/plan.md')

  it('accepts absolute and folder-relative paths', () => {
    expect(resolve('/work/README.md')).toBe('/work/README.md')
    expect(resolve('docs/site/index.html')).toBe('/work/docs/site/index.html')
    expect(resolve('./docs/plan.md')).toBe('/work/docs/plan.md')
  })

  it('accepts paths relative to the viewed document', () => {
    expect(resolve('site/index.html')).toBe('/work/docs/site/index.html')
  })

  it('accepts bare names only when they are unambiguous', () => {
    expect(resolve('plan.md')).toBe('/work/docs/plan.md')
    expect(resolve('notes.md')).toBeNull()
    expect(resolve('missing.md')).toBeNull()
  })
})
