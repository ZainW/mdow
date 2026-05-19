import { render, screen, fireEvent } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { rovingTabIndex, useRovingFocus } from './useRovingFocus'

function Harness({
  active,
  orientation = 'both',
}: {
  active: number
  orientation?: 'horizontal' | 'vertical' | 'both'
}) {
  const { containerRef, onKeyDown } = useRovingFocus({ orientation })
  return (
    <div ref={containerRef} role="radiogroup" aria-label="test" tabIndex={-1} onKeyDown={onKeyDown}>
      {[0, 1, 2].map((i) => (
        <button
          key={i}
          type="button"
          // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- harness for custom-styled radios
          role="radio"
          aria-checked={active === i}
          tabIndex={rovingTabIndex(active === i)}
          data-testid={`opt-${i}`}
        >
          opt {i}
        </button>
      ))}
    </div>
  )
}

describe('useRovingFocus', () => {
  it('gives the active option tabIndex=0 and the rest tabIndex=-1', () => {
    render(<Harness active={1} />)
    expect(screen.getByTestId('opt-0').getAttribute('tabindex')).toBe('-1')
    expect(screen.getByTestId('opt-1').getAttribute('tabindex')).toBe('0')
    expect(screen.getByTestId('opt-2').getAttribute('tabindex')).toBe('-1')
  })

  it('ArrowRight rotates focus forward', () => {
    render(<Harness active={0} />)
    const first = screen.getByTestId('opt-0')
    first.focus()
    fireEvent.keyDown(first, { key: 'ArrowRight' })
    expect(document.activeElement).toBe(screen.getByTestId('opt-1'))
  })

  it('ArrowLeft rotates focus backward', () => {
    render(<Harness active={1} />)
    const middle = screen.getByTestId('opt-1')
    middle.focus()
    fireEvent.keyDown(middle, { key: 'ArrowLeft' })
    expect(document.activeElement).toBe(screen.getByTestId('opt-0'))
  })

  it('loops from the last option back to the first on ArrowRight', () => {
    render(<Harness active={2} />)
    const last = screen.getByTestId('opt-2')
    last.focus()
    fireEvent.keyDown(last, { key: 'ArrowRight' })
    expect(document.activeElement).toBe(screen.getByTestId('opt-0'))
  })

  it('Home / End focus first / last', () => {
    render(<Harness active={1} />)
    const middle = screen.getByTestId('opt-1')
    middle.focus()
    fireEvent.keyDown(middle, { key: 'Home' })
    expect(document.activeElement).toBe(screen.getByTestId('opt-0'))
    fireEvent.keyDown(screen.getByTestId('opt-0'), { key: 'End' })
    expect(document.activeElement).toBe(screen.getByTestId('opt-2'))
  })

  it('respects the orientation flag', () => {
    render(<Harness active={0} orientation="horizontal" />)
    const first = screen.getByTestId('opt-0')
    first.focus()
    // ArrowDown should be ignored in horizontal mode
    fireEvent.keyDown(first, { key: 'ArrowDown' })
    expect(document.activeElement).toBe(first)
    // ArrowRight should still work
    fireEvent.keyDown(first, { key: 'ArrowRight' })
    expect(document.activeElement).toBe(screen.getByTestId('opt-1'))
  })
})
