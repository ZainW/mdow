import { render, screen, fireEvent, within } from '@testing-library/react'
import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest'
import { SettingsDialog } from './SettingsDialog'
import { useAppStore } from '../store/app-store'

function renderOpen() {
  return render(<SettingsDialog open onOpenChange={() => {}} />)
}

describe('SettingsDialog accessibility', () => {
  // Track and restore window.api so tests stay isolated from each other.
  let originalApi: unknown

  beforeEach(() => {
    originalApi = globalThis.window.api
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

  afterEach(() => {
    if (originalApi === undefined) {
      // @ts-expect-error — minimal stub teardown
      delete globalThis.window.api
    } else {
      globalThis.window.api = originalApi as typeof globalThis.window.api
    }
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

  it('ArrowRight rotates focus within the Theme radiogroup', () => {
    renderOpen()
    const themeGroup = screen.getByRole('radiogroup', { name: 'Theme' })
    const system = within(themeGroup).getByRole('radio', { name: /System/ })
    const light = within(themeGroup).getByRole('radio', { name: /Light/ })
    system.focus()
    fireEvent.keyDown(system, { key: 'ArrowRight' })
    expect(document.activeElement).toBe(light)
  })

  it('only the active theme option has tabIndex=0', () => {
    renderOpen()
    const group = screen.getByRole('radiogroup', { name: 'Theme' })
    const radios = Array.from(group.querySelectorAll('[role="radio"]'))
    // theme=system in beforeEach → System has tabIndex=0
    expect(radios[0].getAttribute('tabindex')).toBe('0')
    expect(radios[1].getAttribute('tabindex')).toBe('-1')
    expect(radios[2].getAttribute('tabindex')).toBe('-1')
  })
})
