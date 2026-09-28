import { act, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from '../store/app-store'
import { ZOOM_HUD_VISIBLE_MS, ZoomIndicator } from './ZoomIndicator'

vi.mock('../lib/motion', () => ({ prefersReducedMotion: () => true }))

describe('ZoomIndicator', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    useAppStore.setState({ zoomLevel: 100 })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('stays hidden until the zoom level changes', () => {
    render(<ZoomIndicator />)
    expect(screen.queryByTestId('zoom-indicator')).not.toBeInTheDocument()
  })

  it('appears on a zoom change and hides itself again', () => {
    render(<ZoomIndicator />)
    act(() => useAppStore.setState({ zoomLevel: 110 }))
    const pill = screen.getByTestId('zoom-indicator')
    expect(pill).toHaveTextContent('110%')
    expect(screen.getByRole('button', { name: 'Zoom out' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Reset' })).toBeEnabled()

    // Transient even when the zoom is not 100%.
    act(() => {
      vi.advanceTimersByTime(ZOOM_HUD_VISIBLE_MS)
    })
    expect(screen.queryByTestId('zoom-indicator')).not.toBeInTheDocument()
  })
})
