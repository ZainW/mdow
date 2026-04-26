import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { UpdateBanner } from './UpdateBanner'

type Cb = (...args: unknown[]) => void

interface MockApi {
  checkForUpdates: ReturnType<typeof vi.fn>
  downloadUpdate: ReturnType<typeof vi.fn>
  installUpdate: ReturnType<typeof vi.fn>
  onUpdateAvailable: (cb: Cb) => () => void
  onUpdateUpToDate: (cb: Cb) => () => void
  onUpdateDownloadProgress: (cb: Cb) => () => void
  onUpdateDownloaded: (cb: Cb) => () => void
  onUpdateError: (cb: Cb) => () => void
  onMenuCheckForUpdates: (cb: Cb) => () => void
}

let listeners: Record<string, Cb> = {}

function subscriber(name: string) {
  return (cb: Cb) => {
    listeners[name] = cb
    return () => {
      delete listeners[name]
    }
  }
}

beforeEach(() => {
  listeners = {}
  const api: MockApi = {
    checkForUpdates: vi.fn().mockResolvedValue(undefined),
    downloadUpdate: vi.fn().mockResolvedValue(undefined),
    installUpdate: vi.fn().mockResolvedValue(undefined),
    onUpdateAvailable: subscriber('available'),
    onUpdateUpToDate: subscriber('upToDate'),
    onUpdateDownloadProgress: subscriber('progress'),
    onUpdateDownloaded: subscriber('downloaded'),
    onUpdateError: subscriber('error'),
    onMenuCheckForUpdates: subscriber('menuCheck'),
  }
  // Assign mock API to window for the component under test.
  Object.defineProperty(globalThis, 'window', {
    value: { api },
    writable: true,
    configurable: true,
  })
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('UpdateBanner', () => {
  it('renders nothing initially', () => {
    const { container } = render(<UpdateBanner />)
    expect(container).toBeEmptyDOMElement()
  })

  it('shows the available banner with version when an update arrives', () => {
    render(<UpdateBanner />)
    act(() => listeners.available({ version: '1.2.3' }))
    expect(screen.getByText(/1\.2\.3/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /download/i })).toBeInTheDocument()
  })

  it('shows progress with tabular-nums during download', () => {
    render(<UpdateBanner />)
    act(() => listeners.progress({ percent: 42 }))
    const pct = screen.getByText(/42/)
    expect(pct).toBeInTheDocument()
    expect(pct.className).toMatch(/tabular-nums/)
  })

  it('shows ready state with a Restart button when downloaded', () => {
    render(<UpdateBanner />)
    act(() => listeners.downloaded())
    expect(screen.getByRole('button', { name: /restart/i })).toBeInTheDocument()
  })

  it('does NOT show "up to date" for non-manual checks', () => {
    render(<UpdateBanner />)
    act(() => listeners.upToDate({ wasManual: false }))
    expect(screen.queryByText(/up to date|latest version/i)).not.toBeInTheDocument()
  })

  it('shows "up to date" when triggered manually', () => {
    render(<UpdateBanner />)
    act(() => listeners.upToDate({ wasManual: true }))
    expect(screen.getByText(/up to date|latest version/i)).toBeInTheDocument()
  })

  it('surfaces a brief failure message when an error event fires', () => {
    render(<UpdateBanner />)
    act(() => listeners.error('boom'))
    expect(screen.getByText(/couldn't check for updates/i)).toBeInTheDocument()
  })

  it('triggers a manual check when the menu signal fires', () => {
    render(<UpdateBanner />)
    act(() => listeners.menuCheck())
    expect(window.api.checkForUpdates).toHaveBeenCalledWith({ manual: true })
  })
})
