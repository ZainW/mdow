import { describe, expect, it } from 'vitest'
import { parseUnifiedDiff } from './unified-diff'

describe('parseUnifiedDiff', () => {
  it('numbers lines on both sides of each hunk', () => {
    const patch = [
      'Index: notes.md',
      '===================================================================',
      '--- notes.md',
      '+++ notes.md',
      '@@ -1,5 +1,5 @@ # Probe',
      ' # Probe Doc',
      ' ',
      '-This is teh first paragraph.',
      '+This is the first paragraph.',
      ' ',
      ' Second paragraph here.',
      '\\ No newline at end of file',
      '',
    ].join('\n')

    expect(parseUnifiedDiff(patch)).toEqual([
      {
        header: '# Probe',
        lines: [
          { kind: 'context', text: '# Probe Doc', oldLine: 1, newLine: 1 },
          { kind: 'context', text: '', oldLine: 2, newLine: 2 },
          { kind: 'remove', text: 'This is teh first paragraph.', oldLine: 3, newLine: null },
          { kind: 'add', text: 'This is the first paragraph.', oldLine: null, newLine: 3 },
          { kind: 'context', text: '', oldLine: 4, newLine: 4 },
          { kind: 'context', text: 'Second paragraph here.', oldLine: 5, newLine: 5 },
        ],
      },
    ])
  })

  it('handles several hunks and new files', () => {
    const hunks = parseUnifiedDiff('@@ -0,0 +1,2 @@\n+# New\n+Body\n@@ -10 +11 @@\n-a\n+b\n')
    expect(hunks).toHaveLength(2)
    expect(hunks[0].lines.map((line) => line.newLine)).toEqual([1, 2])
    expect(hunks[1].lines[0]).toEqual({ kind: 'remove', text: 'a', oldLine: 10, newLine: null })
  })

  it('returns nothing for an empty patch', () => {
    expect(parseUnifiedDiff('')).toEqual([])
  })
})
