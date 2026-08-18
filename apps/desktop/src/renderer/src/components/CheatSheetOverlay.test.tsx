import { act, fireEvent, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { CheatSheetOverlay } from './CheatSheetOverlay'
import { CHEAT_SHEET_HOLD_MS } from '@renderer/lib/cheat-sheet'
import { useAppStore } from '@renderer/store/app-store'
import { createMinimalWindowApi, stubWindowApi } from '@renderer/test/stubWindowApi'
import { renderWithProviders } from '@renderer/test/renderWithProviders'

stubWindowApi(() => createMinimalWindowApi({ platform: 'linux' }))

function holdControl() {
  fireEvent.keyDown(window, { key: 'Control', ctrlKey: true, bubbles: true })
}

function releaseControl() {
  fireEvent.keyUp(window, { key: 'Control', bubbles: true })
}

describe('CheatSheetOverlay', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    useAppStore.setState({
      commandPaletteOpen: false,
      searchOpen: false,
      shortcutsDialogOpen: false,
      settingsOpen: false,
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('appears after holding Control and lists shortcuts', () => {
    renderWithProviders(<CheatSheetOverlay />)
    holdControl()
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })

    expect(screen.getByRole('dialog', { name: 'Keyboard cheat sheet' })).toBeInTheDocument()
    expect(screen.getByText('Command palette')).toBeInTheDocument()
    expect(screen.getByText(/release to dismiss/)).toBeInTheDocument()
    expect(screen.getByText(/for full list/)).toBeInTheDocument()
  })

  it('does not appear if Control is released before the hold delay', () => {
    renderWithProviders(<CheatSheetOverlay />)
    holdControl()
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS - 1)
    })
    releaseControl()
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })

    expect(screen.queryByRole('dialog', { name: 'Keyboard cheat sheet' })).not.toBeInTheDocument()
  })

  it('cancels the pending hold when another key is pressed', () => {
    renderWithProviders(<CheatSheetOverlay />)
    holdControl()
    fireEvent.keyDown(window, { key: 'k', ctrlKey: true, bubbles: true })
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })

    expect(screen.queryByRole('dialog', { name: 'Keyboard cheat sheet' })).not.toBeInTheDocument()
  })

  it('dismisses on Control release', () => {
    renderWithProviders(<CheatSheetOverlay />)
    holdControl()
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })
    expect(screen.getByRole('dialog', { name: 'Keyboard cheat sheet' })).toBeInTheDocument()

    releaseControl()
    expect(screen.queryByRole('dialog', { name: 'Keyboard cheat sheet' })).not.toBeInTheDocument()
  })

  it('dismisses on Escape and stays closed until Control is released', () => {
    renderWithProviders(<CheatSheetOverlay />)
    holdControl()
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })

    fireEvent.keyDown(window, { key: 'Escape', bubbles: true })
    expect(screen.queryByRole('dialog', { name: 'Keyboard cheat sheet' })).not.toBeInTheDocument()

    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })
    expect(screen.queryByRole('dialog', { name: 'Keyboard cheat sheet' })).not.toBeInTheDocument()

    releaseControl()
    holdControl()
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })
    expect(screen.getByRole('dialog', { name: 'Keyboard cheat sheet' })).toBeInTheDocument()
  })

  it('does not appear while a modal is open', () => {
    useAppStore.setState({ commandPaletteOpen: true })
    renderWithProviders(<CheatSheetOverlay />)
    holdControl()
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })

    expect(screen.queryByRole('dialog', { name: 'Keyboard cheat sheet' })).not.toBeInTheDocument()
  })

  it('hides when a modal opens while the overlay is visible', () => {
    renderWithProviders(<CheatSheetOverlay />)
    holdControl()
    act(() => {
      vi.advanceTimersByTime(CHEAT_SHEET_HOLD_MS)
    })
    expect(screen.getByRole('dialog', { name: 'Keyboard cheat sheet' })).toBeInTheDocument()

    act(() => {
      useAppStore.setState({ shortcutsDialogOpen: true })
    })
    expect(screen.queryByRole('dialog', { name: 'Keyboard cheat sheet' })).not.toBeInTheDocument()
  })
})
