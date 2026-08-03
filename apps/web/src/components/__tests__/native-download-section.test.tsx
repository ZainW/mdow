import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { NativeDownloadSection } from '../native-download-section'

describe('NativeDownloadSection', () => {
  it('renders the GPUI beta download message and link', () => {
    const betaUrl = 'https://example.test/MdowNative-mac-beta.zip'

    render(<NativeDownloadSection betaUrl={betaUrl} />)

    expect(screen.getByRole('heading', { name: 'Mdow Native' })).toBeInTheDocument()
    expect(
      screen.getByText('A GPUI beta for Apple Silicon Macs running macOS 14 or newer.'),
    ).toBeInTheDocument()
    expect(screen.getByText('Runs alongside the regular Mdow app.')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Download Mdow Native (.zip)' })).toHaveAttribute(
      'href',
      betaUrl,
    )
  })
})
