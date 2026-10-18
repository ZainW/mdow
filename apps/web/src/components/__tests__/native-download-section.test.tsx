import type { ReactNode } from 'react'
import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { NativeDownloadSection } from '../native-download-section'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: ReactNode }) => <a href="/docs/installation">{children}</a>,
}))

describe('NativeDownloadSection', () => {
  it('offers the Linux Native beta only', () => {
    const linuxUrl = 'https://example.test/MdowNative-linux-beta.AppImage'

    render(<NativeDownloadSection linuxUrl={linuxUrl} />)

    expect(screen.getByRole('heading', { name: 'Mdow Native' })).toBeInTheDocument()
    expect(
      screen.getByText(
        'A GPU-rendered beta for x64 Linux, built with gpuix. Runs alongside the regular Mdow app.',
      ),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('link', { name: 'Download Linux Mdow Native (.AppImage)' }),
    ).toHaveAttribute('href', linuxUrl)
    expect(screen.queryByRole('link', { name: /Download macOS/ })).toBeNull()
  })
})
