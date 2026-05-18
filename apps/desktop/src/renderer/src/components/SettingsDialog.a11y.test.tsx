import { render, screen, fireEvent } from '@testing-library/react'
import { describe, expect, it, beforeEach, vi } from 'vitest'
import { SettingsDialog } from './SettingsDialog'
import { useAppStore } from '../store/app-store'

function renderOpen() {
  return render(<SettingsDialog open onOpenChange={() => {}} />)
}

describe('SettingsDialog accessibility', () => {
  beforeEach(() => {
    // Stub the IPC bridge — actions call window.api.setTheme et al.
    // @ts-expect-error — minimal stub for unit tests
    globalThis.window.api = {
      setTheme: vi.fn(),
      saveAppState: vi.fn(),
      setAutoUpdateScheduling: vi.fn(),
    }
    useAppStore.setState({
      theme: 'system',
      contentFont: 'inter',
      codeFont: 'geist-mono',
    })
  })

  it('exposes a Theme radiogroup with three options', () => {
    renderOpen()
    const themeGroup = screen.getByRole('radiogroup', { name: 'Theme' })
    expect(themeGroup).toBeInTheDocument()
    const options = screen
      .getAllByRole('radio')
      .filter(
        (r) =>
          r.textContent?.trim() === 'System' ||
          r.textContent?.trim() === 'Light' ||
          r.textContent?.trim() === 'Dark',
      )
    expect(options).toHaveLength(3)
  })

  it('toggles aria-checked when a theme option is selected', () => {
    renderOpen()
    const dark = screen.getByRole('radio', { name: /Dark/ })
    expect(dark.getAttribute('aria-checked')).toBe('false')
    fireEvent.click(dark)
    expect(dark.getAttribute('aria-checked')).toBe('true')
    expect(useAppStore.getState().theme).toBe('dark')
  })

  it('exposes Content font and Code font as separate radiogroups', () => {
    renderOpen()
    expect(screen.getByRole('radiogroup', { name: 'Content font' })).toBeInTheDocument()
    expect(screen.getByRole('radiogroup', { name: 'Code font' })).toBeInTheDocument()
  })
})
