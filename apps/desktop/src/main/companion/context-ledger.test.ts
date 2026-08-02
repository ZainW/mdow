import { describe, expect, it } from 'vitest'
import { ContextLedger } from './context-ledger'

describe('Companion context ledger', () => {
  it('marks unchanged content as already sent and changed content as new', () => {
    const ledger = new ContextLedger()

    expect(ledger.record('/docs/a.md', 'one').alreadySent).toBe(false)
    expect(ledger.record('/docs/a.md', 'one').alreadySent).toBe(true)
    expect(ledger.record('/docs/a.md', 'two').alreadySent).toBe(false)
    ledger.clear()
    expect(ledger.record('/docs/a.md', 'two').alreadySent).toBe(false)
  })

  it('does not conflate identical content at different paths', () => {
    const ledger = new ContextLedger()

    const first = ledger.record('/docs/a.md', 'same')
    const second = ledger.record('/docs/b.md', 'same')

    expect(first.hash).toBe(second.hash)
    expect(second.alreadySent).toBe(false)
    expect(ledger.has('/docs/a.md', first.hash)).toBe(true)
    expect(ledger.has('/docs/b.md', first.hash)).toBe(true)
  })
})
