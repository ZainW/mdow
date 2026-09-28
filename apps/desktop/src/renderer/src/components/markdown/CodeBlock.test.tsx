import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { CodeBlock } from './CodeBlock'

describe('CodeBlock', () => {
  it('puts the language and a labelled Copy button in a header above the code', () => {
    const { container } = render(
      <CodeBlock language="rust">
        <code>fn main() {}</code>
      </CodeBlock>,
    )
    const header = container.querySelector('.code-block-header')
    expect(header).not.toBeNull()
    expect(header).toHaveTextContent('rust')
    expect(header).toContainElement(screen.getByRole('button', { name: 'Copy code' }))
    // The header is a sibling before <pre>, never overlaid on the code.
    expect(header?.nextElementSibling?.tagName).toBe('PRE')
  })

  it('keeps the header (and Copy) when the fence has no language', () => {
    const { container } = render(
      <CodeBlock>
        <code>plain</code>
      </CodeBlock>,
    )
    expect(container.querySelector('.code-block-header')).not.toBeNull()
    expect(screen.getByRole('button', { name: 'Copy code' })).toBeInTheDocument()
  })
})
