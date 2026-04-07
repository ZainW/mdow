import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { FeatureRow } from '../feature-row'

describe('FeatureRow', () => {
  it('renders title and description', () => {
    render(
      <FeatureRow title="Hello" description="World" align="left">
        <div data-testid="visual">visual</div>
      </FeatureRow>,
    )
    expect(screen.getByText('Hello')).toBeInTheDocument()
    expect(screen.getByText('World')).toBeInTheDocument()
    expect(screen.getByTestId('visual')).toBeInTheDocument()
  })

  it('reverses order when align is right', () => {
    const { container } = render(
      <FeatureRow title="t" description="d" align="right">
        <div>v</div>
      </FeatureRow>,
    )
    expect(container.querySelector('.md\\:flex-row-reverse')).not.toBeNull()
  })
})
