import { describe, expect, it } from 'vitest'
import { extractCompanionCitations } from './companion-citations'

describe('extractCompanionCitations', () => {
  it('strips valid source markers and returns citation chips', () => {
    const result = extractCompanionCitations('Mdow reads markdown. [[source:src_active]]', [
      { id: 'src_active', title: 'README.md', path: '/docs/README.md' },
    ])

    expect(result.text).toBe('Mdow reads markdown.')
    expect(result.citations).toEqual([
      { sourceId: 'src_active', title: 'README.md', path: '/docs/README.md' },
    ])
  })

  it('removes invalid source markers without creating trusted citations', () => {
    const result = extractCompanionCitations('Claim [[source:unknown]]', [])

    expect(result.text).toBe('Claim')
    expect(result.citations).toEqual([])
  })
})
