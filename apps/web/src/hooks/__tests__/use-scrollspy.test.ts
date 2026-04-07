import { renderHook } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useScrollspy } from '../use-scrollspy'

describe('useScrollspy', () => {
  beforeEach(() => {
    vi.stubGlobal(
      'IntersectionObserver',
      class IntersectionObserverMock {
        observe = vi.fn()
        disconnect = vi.fn()
        unobserve = vi.fn()
        takeRecords = vi.fn()
      },
    )
  })

  it('returns null when no headings provided', () => {
    const { result } = renderHook(() => useScrollspy([]))
    expect(result.current).toBeNull()
  })

  it('observes each heading id', () => {
    document.body.innerHTML = '<h2 id="a"></h2><h2 id="b"></h2>'
    const observe = vi.fn()
    vi.stubGlobal(
      'IntersectionObserver',
      class IntersectionObserverMock {
        observe = observe
        disconnect = vi.fn()
        unobserve = vi.fn()
        takeRecords = vi.fn()
      },
    )
    renderHook(() => useScrollspy(['a', 'b']))
    expect(observe).toHaveBeenCalledTimes(2)
  })
})
