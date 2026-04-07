import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { BrowserFrame } from '../browser-frame'

describe('BrowserFrame', () => {
  it('renders children inside the frame', () => {
    render(
      <BrowserFrame>
        <div>hello</div>
      </BrowserFrame>,
    )
    expect(screen.getByText('hello')).toBeInTheDocument()
  })

  it('renders the title in the chrome when provided', () => {
    render(<BrowserFrame title="readme.md">content</BrowserFrame>)
    expect(screen.getByText('readme.md')).toBeInTheDocument()
  })

  it('applies extra className to the outer wrapper', () => {
    const { container } = render(<BrowserFrame className="custom-x">x</BrowserFrame>)
    expect(container.firstChild).toHaveClass('custom-x')
  })
})
