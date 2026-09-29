import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { NativeDownloadSection } from '../native-download-section'

describe('NativeDownloadSection', () => {
  it('renders mac and linux GPUI beta downloads', () => {
    const macUrl = 'https://example.test/MdowNative-mac-beta.zip'
    const linuxUrl = 'https://example.test/MdowNative-linux-beta.AppImage'

    render(<NativeDownloadSection macUrl={macUrl} linuxUrl={linuxUrl} />)

    expect(screen.getByRole('heading', { name: 'Mdow Native' })).toBeInTheDocument()
    expect(
      screen.getByText(
        'A GPU-rendered beta for Apple Silicon Macs and x64 Linux, built with gpuix.',
      ),
    ).toBeInTheDocument()
    expect(screen.getByText('Runs alongside the regular Mdow app.')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Download Mdow Native (.zip)' })).toHaveAttribute(
      'href',
      macUrl,
    )
    expect(screen.getByRole('link', { name: 'Download Mdow Native (.AppImage)' })).toHaveAttribute(
      'href',
      linuxUrl,
    )
  })
})
