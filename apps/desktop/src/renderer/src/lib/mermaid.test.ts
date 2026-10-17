import { beforeEach, describe, expect, it } from 'vitest'
import { renderMermaidBlock } from './mermaid'

function mount(id: string): HTMLElement {
  const el = document.createElement('div')
  el.id = id
  document.body.append(el)
  return el
}

beforeEach(() => {
  document.body.replaceChildren()
})

describe('mermaid renderer', () => {
  it('renders a diagram to inline SVG', async () => {
    const el = mount('diagram-1')

    await renderMermaidBlock({ id: 'diagram-1', code: 'flowchart TD\n  A --> B' })

    const svg = el.querySelector('svg')
    expect(svg).toBeInTheDocument()
    expect(svg).toHaveClass('pele')
    expect(el.querySelector('iframe')).toBeNull()
    expect(el.getAttribute('role')).toBe('img')
    expect(el.getAttribute('aria-label')).toBe('Mermaid diagram')
  })

  it('shows the parse error with its location', async () => {
    const el = mount('diagram-3')

    await renderMermaidBlock({ id: 'diagram-3', code: 'flowchart TD\n  A -->\n  B -->> {oops' })

    expect(el).toHaveClass('mermaid-error')
    expect(el.querySelector('svg')).toBeNull()
    expect(el.getAttribute('role')).toBeNull()
    expect(el.textContent).toMatch(/^Mermaid diagram error: Parse error on line 3/)
  })

  it('ignores blocks that are no longer in the document', async () => {
    await expect(
      renderMermaidBlock({ id: 'missing', code: 'flowchart TD\n  A --> B' }),
    ).resolves.toBeUndefined()
  })
})
