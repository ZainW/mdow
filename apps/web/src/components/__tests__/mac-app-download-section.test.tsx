import type { ReactNode } from 'react'
import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { MacAppDownloadSection } from '../mac-app-download-section'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: ReactNode }) => <a href="/docs/installation">{children}</a>,
}))

describe('MacAppDownloadSection', () => {
  it('links the zip and shows the version', () => {
    const url = 'https://example.test/Mdow-for-Mac-0.1.0.zip'
    render(
      <MacAppDownloadSection
        download={{ url, version: '0.1.0', publishedAt: '2026-10-18T10:00:00Z' }}
      />,
    )

    expect(screen.getByRole('heading', { name: 'Mdow for Mac' })).toBeInTheDocument()
    expect(
      screen.getByRole('link', { name: 'Download macOS Mdow for Mac (.zip)' }),
    ).toHaveAttribute('href', url)
    expect(screen.getByText(/Version 0\.1\.0 · October 18, 2026/)).toBeInTheDocument()
  })

  it('still offers the download when GitHub is unreachable', () => {
    const url = 'https://example.test/Mdow-for-Mac.zip'
    render(<MacAppDownloadSection download={{ url, version: null, publishedAt: null }} />)

    expect(
      screen.getByRole('link', { name: 'Download macOS Mdow for Mac (.zip)' }),
    ).toHaveAttribute('href', url)
    expect(screen.getByText('Apple Silicon · macOS 15 or newer')).toBeInTheDocument()
  })
})
