import type { ReactNode } from 'react'
import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { NativeDownloadSection } from '../native-download-section'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: ReactNode }) => <a href="/docs/installation">{children}</a>,
}))

describe('NativeDownloadSection', () => {
  it('renders mac and linux Native beta downloads', () => {
    const macUrl = 'https://example.test/MdowNative-mac-beta.zip'
    const linuxUrl = 'https://example.test/MdowNative-linux-beta.AppImage'

    render(<NativeDownloadSection macUrl={macUrl} linuxUrl={linuxUrl} />)

    expect(screen.getByRole('heading', { name: 'Mdow Native' })).toBeInTheDocument()
    expect(
      screen.getByText(
        'A GPU-rendered beta for Apple Silicon Macs and x64 Linux, built with gpuix. Runs alongside the regular Mdow app.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Download macOS Mdow Native (.zip)' })).toHaveAttribute(
      'href',
      macUrl,
    )
    expect(
      screen.getByRole('link', { name: 'Download Linux Mdow Native (.AppImage)' }),
    ).toHaveAttribute('href', linuxUrl)
  })

  it('omits the linux row when there is no linux build', () => {
    render(<NativeDownloadSection macUrl="https://example.test/mac.zip" linuxUrl={null} />)

    expect(screen.getAllByRole('link', { name: /Download .* Mdow Native/ })).toHaveLength(1)
  })
})
