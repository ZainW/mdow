import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { ALERT_TYPES, AlertCallout } from './AlertCallout'

describe('AlertCallout', () => {
  it.each([
    ['note', 'Note'],
    ['tip', 'Tip'],
    ['important', 'Important'],
    ['warning', 'Warning'],
    ['caution', 'Caution'],
  ])('renders a %s callout with an icon + title row above the body', (type, title) => {
    const { container } = render(
      <AlertCallout type={type}>
        <p>Body text</p>
      </AlertCallout>,
    )
    const note = screen.getByRole('note', { name: title })
    expect(note).toHaveClass(`markdown-alert-${type}`)
    const heading = container.querySelector('.markdown-alert-title')
    expect(heading).toHaveTextContent(title)
    expect(heading?.querySelector('svg')).not.toBeNull()
    expect(container.querySelector('.markdown-alert-body')).toHaveTextContent('Body text')
  })

  it('covers all five alert kinds', () => {
    expect(ALERT_TYPES).toHaveLength(5)
  })
})
