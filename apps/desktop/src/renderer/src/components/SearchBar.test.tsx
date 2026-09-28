import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SearchBar, formatMatchCount } from './SearchBar'

describe('formatMatchCount', () => {
  it('shows "current / total"', () => {
    expect(formatMatchCount('match', 3, 1)).toBe('2 / 3')
  })

  it('is empty without a query and says when nothing matched', () => {
    expect(formatMatchCount('', 0, 0)).toBe('')
    expect(formatMatchCount('zzz', 0, 0)).toBe('No results')
  })
})

describe('SearchBar', () => {
  function renderBar(matchCount = 3) {
    const props = {
      matchCount,
      currentIndex: 1,
      onNext: vi.fn(),
      onPrev: vi.fn(),
      onClose: vi.fn(),
      onQueryChange: vi.fn(),
    }
    render(<SearchBar {...props} />)
    return props
  }

  it('focuses the field and reports the count after typing', () => {
    const props = renderBar()
    const input = screen.getByRole('textbox', { name: 'Search in document' })
    expect(input).toHaveFocus()
    fireEvent.change(input, { target: { value: 'match' } })
    expect(props.onQueryChange).toHaveBeenCalledWith('match')
    expect(screen.getByText('2 / 3')).toHaveClass('tabular-nums')
  })

  it('steps with Enter / Shift+Enter and closes on Escape', () => {
    const props = renderBar()
    const input = screen.getByRole('textbox', { name: 'Search in document' })
    fireEvent.keyDown(input, { key: 'Enter' })
    fireEvent.keyDown(input, { key: 'Enter', shiftKey: true })
    fireEvent.keyDown(input, { key: 'Escape' })
    expect(props.onNext).toHaveBeenCalledOnce()
    expect(props.onPrev).toHaveBeenCalledOnce()
    expect(props.onClose).toHaveBeenCalledOnce()
  })

  it('disables stepping when there are no matches', () => {
    renderBar(0)
    expect(screen.getByRole('button', { name: 'Previous match' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Next match' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Close search' })).toBeEnabled()
  })
})
