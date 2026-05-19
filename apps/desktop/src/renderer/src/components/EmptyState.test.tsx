import { render, screen, fireEvent } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { Clock } from '@phosphor-icons/react'
import { EmptyState } from './EmptyState'

describe('EmptyState', () => {
  it('renders the title', () => {
    render(<EmptyState icon={Clock} title="Nothing here" />)
    expect(screen.getByText('Nothing here')).toBeInTheDocument()
  })

  it('renders the hint when provided', () => {
    render(<EmptyState icon={Clock} title="Title" hint="Hint copy" />)
    expect(screen.getByText('Hint copy')).toBeInTheDocument()
  })

  it('omits the hint paragraph when not provided', () => {
    render(<EmptyState icon={Clock} title="Title" />)
    expect(screen.queryByText('Hint copy')).not.toBeInTheDocument()
  })

  it('renders an action node and forwards clicks', () => {
    const onAction = vi.fn()
    render(
      <EmptyState
        icon={Clock}
        title="Title"
        action={
          <button type="button" onClick={onAction}>
            Do it
          </button>
        }
      />,
    )
    fireEvent.click(screen.getByText('Do it'))
    expect(onAction).toHaveBeenCalledOnce()
  })

  it('applies the small variant by sizing the icon down', () => {
    const { container } = render(<EmptyState icon={Clock} title="Title" size="sm" />)
    // small variant uses size-5 (20px) icon; default md uses size-6 inside size-12 well
    expect(container.querySelector('svg.size-5')).not.toBeNull()
  })

  it('uses the medium variant by default', () => {
    const { container } = render(<EmptyState icon={Clock} title="Title" />)
    expect(container.querySelector('.rounded-full.bg-muted')).not.toBeNull()
  })
})
