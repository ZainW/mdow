import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { DownloadCard } from '../download-card'

describe('DownloadCard', () => {
  it('hides its decorative platform icon from assistive technology', () => {
    render(<DownloadCard platform="macOS" icon="🍎" formats={[]} />)

    expect(screen.getByText('🍎')).toHaveAttribute('aria-hidden', 'true')
  })
})
