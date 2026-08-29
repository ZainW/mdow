import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { NativeDownloadSection } from '../native-download-section'

describe('NativeDownloadSection', () => {
  it('renders mac and linux GPUI beta downloads', () => {
    const macUrl = 'https://example.test/MdowNative-mac-beta.zip'
    const linuxUrl = 'https://example.test/MdowNative-linux-beta.zip'

    render(<NativeDownloadSection macUrl={macUrl} linuxUrl={linuxUrl} />)

    expect(screen.getByRole('heading', { name: 'Mdow Native' })).toBeInTheDocument()
    expect(
      screen.getByText('A GPUI beta for Apple Silicon Macs and x64 Linux.'),
    ).toBeInTheDocument()
    expect(screen.getByText('Runs alongside the regular Mdow app.')).toBeInTheDocument()
    const downloads = screen.getAllByRole('link', { name: 'Download Mdow Native (.zip)' })
    expect(downloads).toHaveLength(2)
    expect(downloads[0]).toHaveAttribute('href', macUrl)
    expect(downloads[1]).toHaveAttribute('href', linuxUrl)
  })
})
