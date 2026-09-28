import { render, screen, fireEvent, within } from '@testing-library/react'
import { describe, expect, it, beforeEach, vi } from 'vitest'
import { SettingsDialog } from './SettingsDialog'
import { useAppStore } from '../store/app-store'
import { stubWindowApi } from '../test/stubWindowApi'

function renderOpen() {
  return render(<SettingsDialog open onOpenChange={() => {}} />)
}

const saveAppState = vi.fn()
const checkForUpdates = vi.fn().mockResolvedValue(undefined)
const unsubscribe = () => () => {}

describe('SettingsDialog', () => {
  stubWindowApi(() => ({
    setTheme: vi.fn(),
    saveAppState,
    setAutoUpdateScheduling: vi.fn(),
    getAppVersion: vi.fn().mockResolvedValue('1.10.0'),
    checkForUpdates,
    onUpdateUpToDate: unsubscribe,
    onUpdateAvailable: unsubscribe,
    onUpdateDownloadProgress: unsubscribe,
    onUpdateDownloaded: unsubscribe,
    onUpdateError: unsubscribe,
  }))

  beforeEach(() => {
    saveAppState.mockClear()
    checkForUpdates.mockClear()
    useAppStore.setState({
      theme: 'system',
      contentFont: 'inter',
      codeFont: 'geist-mono',
      interfaceScale: 'compact',
      readingWidth: 'medium',
      zoomLevel: 100,
      autoUpdateEnabled: true,
    })
  })

  it('groups controls under Appearance, Reading and Updates', () => {
    renderOpen()
    for (const name of ['Appearance', 'Reading', 'Updates']) {
      expect(screen.getByRole('region', { name })).toBeInTheDocument()
    }
  })

  it('exposes Theme as a radiogroup with three equal segments', () => {
    renderOpen()
    const themeGroup = screen.getByRole('radiogroup', { name: 'Theme' })
    const options = within(themeGroup).getAllByRole('radio')
    expect(options.map((o) => o.getAttribute('aria-label'))).toEqual(['System', 'Light', 'Dark'])
    expect(themeGroup.className).toContain('auto-cols-fr')
  })

  it('selects a theme option on click', () => {
    renderOpen()
    const dark = screen.getByRole('radio', { name: 'Dark' })
    expect(dark).toHaveAttribute('aria-checked', 'false')
    fireEvent.click(dark)
    expect(dark).toHaveAttribute('aria-checked', 'true')
    expect(useAppStore.getState().theme).toBe('dark')
  })

  it('renders each font option in its own typeface', () => {
    renderOpen()
    const textFonts = screen.getByRole('radiogroup', { name: 'Text font' })
    const charter = within(textFonts).getByRole('radio', { name: 'Charter' })
    expect(charter.style.fontFamily).toContain('Charter')
    const codeFonts = screen.getByRole('radiogroup', { name: 'Code font' })
    const jetbrains = within(codeFonts).getByRole('radio', { name: 'JetBrains' })
    expect(jetbrains.style.fontFamily).toContain('JetBrains Mono')
  })

  it('offers Narrow / Medium / Wide / Full line widths', () => {
    renderOpen()
    const group = screen.getByRole('radiogroup', { name: 'Line width' })
    const options = within(group).getAllByRole('radio')
    expect(options.map((o) => o.getAttribute('aria-label'))).toEqual([
      'Narrow',
      'Medium',
      'Wide',
      'Full',
    ])
    fireEvent.click(within(group).getByRole('radio', { name: 'Full' }))
    expect(useAppStore.getState().readingWidth).toBe('full')
    expect(saveAppState).toHaveBeenCalledWith({ readingWidth: 'full' })
  })

  it('steps text size with the stepper', () => {
    renderOpen()
    const stepper = screen.getByRole('group', { name: 'Text size' })
    expect(stepper).toHaveTextContent('100%')
    fireEvent.click(screen.getByRole('button', { name: 'Increase text size' }))
    expect(useAppStore.getState().zoomLevel).toBe(110)
    expect(stepper).toHaveTextContent('110%')
    fireEvent.click(screen.getByRole('button', { name: 'Decrease text size' }))
    expect(useAppStore.getState().zoomLevel).toBe(100)
  })

  it('shows the version and checks for updates on demand', async () => {
    renderOpen()
    expect(await screen.findByText('Mdow 1.10.0')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Check now' }))
    expect(checkForUpdates).toHaveBeenCalledWith({ manual: true })
    expect(screen.getByText('Checking…')).toBeInTheDocument()
  })

  it('restores defaults from the footer', () => {
    useAppStore.setState({ theme: 'dark', readingWidth: 'full', zoomLevel: 130 })
    renderOpen()
    fireEvent.click(screen.getByRole('button', { name: 'Restore defaults' }))
    const state = useAppStore.getState()
    expect(state.theme).toBe('system')
    expect(state.readingWidth).toBe('medium')
    expect(state.zoomLevel).toBe(100)
    expect(screen.getByText('Changes save automatically')).toBeInTheDocument()
  })

  it('does not expose manual size or line-height controls', () => {
    renderOpen()
    expect(screen.queryByText('Size')).not.toBeInTheDocument()
    expect(screen.queryByText('Line height')).not.toBeInTheDocument()
  })
})
