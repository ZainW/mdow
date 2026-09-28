import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { DropOverlay } from './DropOverlay'

describe('DropOverlay', () => {
  it('renders nothing when no files are being dragged', () => {
    const { container } = render(<DropOverlay summary={null} />)
    expect(container).toBeEmptyDOMElement()
  })

  it('counts what is being dropped', () => {
    render(
      <DropOverlay summary={{ markdown: 3, html: 0, folders: 1, unknown: 0, unsupported: 0 }} />,
    )
    expect(screen.getByText('Drop to open')).toBeInTheDocument()
    expect(screen.getByRole('status')).toHaveTextContent('3 Markdown files · 1 folder')
    expect(screen.getByTestId('drop-overlay')).toHaveAttribute('data-openable', 'true')
  })

  it('says when nothing can be opened', () => {
    render(
      <DropOverlay summary={{ markdown: 0, html: 0, folders: 0, unknown: 0, unsupported: 2 }} />,
    )
    expect(screen.getByRole('status')).toHaveTextContent('Nothing Mdow can open')
    expect(screen.getByTestId('drop-overlay')).toHaveAttribute('data-openable', 'false')
  })
})
