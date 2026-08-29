import { afterEach, describe, expect, it, vi } from 'vitest'
import { prefersReducedMotion, scrollBehavior, withoutTransitions } from './motion'

describe('motion helpers', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    document.documentElement.classList.remove('theme-switching')
  })

  it('snaps scroll when the user prefers reduced motion', () => {
    vi.spyOn(window, 'matchMedia').mockReturnValue({
      matches: true,
    } as MediaQueryList)

    expect(prefersReducedMotion()).toBe(true)
    expect(scrollBehavior('travel')).toBe('auto')
    expect(scrollBehavior('snap')).toBe('auto')
  })

  it('uses smooth travel and snapped jumps when motion is allowed', () => {
    vi.spyOn(window, 'matchMedia').mockReturnValue({
      matches: false,
    } as MediaQueryList)

    expect(scrollBehavior('travel')).toBe('smooth')
    expect(scrollBehavior('snap')).toBe('auto')
  })

  it('suppresses transitions for one frame while applying a theme change', () => {
    const raf = vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb) => {
      cb(0)
      return 1
    })

    withoutTransitions(() => {
      expect(document.documentElement.classList.contains('theme-switching')).toBe(true)
    })

    expect(raf).toHaveBeenCalled()
    expect(document.documentElement.classList.contains('theme-switching')).toBe(false)
  })
})
