import { describe, expect, it } from 'vitest'
import { applyUnifiedPatch } from './apply-patch'

const doc = '# Plan\n\nWe ship in Q4.\n\n## Risks\n\n- Hiring\n'

describe('applyUnifiedPatch', () => {
  it('applies a hunk with context', () => {
    const patch =
      '--- plan.md\n+++ plan.md\n@@ -2,3 +2,3 @@\n \n-We ship in Q4.\n+We ship in Q1.\n \n'
    expect(applyUnifiedPatch(doc, patch)).toBe(doc.replace('Q4', 'Q1'))
  })

  it('applies several hunks, including pure additions', () => {
    const patch = [
      '@@ -1,1 +1,2 @@',
      ' # Plan',
      '+Draft',
      '@@ -7 +8,2 @@',
      ' - Hiring',
      '+- Scope',
    ].join('\n')
    expect(applyUnifiedPatch(doc, patch)).toBe(
      '# Plan\nDraft\n\nWe ship in Q4.\n\n## Risks\n\n- Hiring\n- Scope\n',
    )
  })

  it('tolerates hunks that drifted from their line numbers', () => {
    const patch = '@@ -1 +1 @@\n-We ship in Q4.\n+We ship in Q1.\n'
    expect(applyUnifiedPatch(doc, patch)).toBe(doc.replace('Q4', 'Q1'))
  })

  it('refuses a patch that no longer matches', () => {
    expect(applyUnifiedPatch(doc, '@@ -3 +3 @@\n-Gone line\n+New\n')).toBeNull()
    expect(applyUnifiedPatch(doc, '')).toBeNull()
  })

  it('keeps Windows line endings', () => {
    expect(applyUnifiedPatch('a\r\nb\r\n', '@@ -2 +2 @@\n-b\n+c\n')).toBe('a\r\nc\r\n')
  })
})
