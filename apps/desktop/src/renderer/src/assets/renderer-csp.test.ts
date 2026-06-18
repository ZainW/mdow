import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

describe('renderer content security policy', () => {
  it('allows mdow-local html previews in sandboxed frames', () => {
    const html = readFileSync(resolve(__dirname, '../../index.html'), 'utf-8')
    expect(html).toContain('frame-src')
    expect(html).toContain('mdow-local:')
  })
})
