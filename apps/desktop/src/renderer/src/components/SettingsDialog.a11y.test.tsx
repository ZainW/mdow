import { render, screen, fireEvent, within } from '@testing-library/react'
import { describe, expect, it, beforeEach, vi } from 'vitest'
import { SettingsDialog } from './SettingsDialog'
import { useAppStore } from '../store/app-store'
import { stubWindowApi } from '../test/stubWindowApi'

function renderOpen() {
  return render(<SettingsDialog open onOpenChange={() => {}} />)
}

describe('SettingsDialog accessibility', () => {
  stubWindowApi(() => ({
    setTheme: vi.fn(),
    saveAppState: vi.fn(),
    setAutoUpdateScheduling: vi.fn(),
  }))

  beforeEach(() => {
    useAppStore.setState({
      theme: 'system',
      contentFont: 'inter',
      codeFont: 'geist-mono',
    })
  })

  it('exposes a Theme toggle group with three options', () => {
    renderOpen()
    const themeGroup = screen.getByRole('group', { name: 'Theme' })
    expect(themeGroup).toBeInTheDocument()
    expect(within(themeGroup).getByRole('button', { name: 'System' })).toBeInTheDocument()
    expect(within(themeGroup).getByRole('button', { name: 'Light' })).toBeInTheDocument()
    expect(within(themeGroup).getByRole('button', { name: 'Dark' })).toBeInTheDocument()
  })

  it('toggles aria-pressed when a theme option is selected', () => {
    renderOpen()
    const dark = screen.getByRole('button', { name: 'Dark' })
    expect(dark.getAttribute('aria-pressed')).toBe('false')
    fireEvent.click(dark)
    expect(dark.getAttribute('aria-pressed')).toBe('true')
    expect(useAppStore.getState().theme).toBe('dark')
  })

  it('exposes Content font and Code font as separate radiogroups', () => {
    renderOpen()
    expect(screen.getByRole('radiogroup', { name: 'Content font' })).toBeInTheDocument()
    expect(screen.getByRole('radiogroup', { name: 'Code font' })).toBeInTheDocument()
  })

  it('selects a theme option on click', () => {
    renderOpen()
    const light = screen.getByRole('button', { name: 'Light' })
    fireEvent.click(light)
    expect(light.getAttribute('aria-pressed')).toBe('true')
    expect(useAppStore.getState().theme).toBe('light')
  })

  it('marks the active theme option as pressed', () => {
    renderOpen()
    const group = screen.getByRole('group', { name: 'Theme' })
    const system = within(group).getByRole('button', { name: 'System' })
    const light = within(group).getByRole('button', { name: 'Light' })
    expect(system.getAttribute('aria-pressed')).toBe('true')
    expect(light.getAttribute('aria-pressed')).toBe('false')
  })
})
