import { fireEvent, render } from '@testing-library/react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it, vi } from 'vitest'
import { CompanionMarkdown, markdownToHtml } from './markdown'

const resolve = (reference: string) =>
  reference === 'docs/plan.md' || reference === 'plan.md' ? '/work/docs/plan.md' : null

describe('CompanionMarkdown', () => {
  it('renders the current stream chunk on the first render', () => {
    const html = renderToStaticMarkup(<CompanionMarkdown text="**Ready now**" streaming />)

    expect(html).toContain('<strong>Ready now</strong>')
  })

  it('renders tables, quotes and rules', () => {
    const html = markdownToHtml('| A | B |\n| --- | --- |\n| 1 | 2 |\n\n> quoted\n\n---')
    expect(html).toContain('<th class="px-2 py-1 text-left font-medium">A</th>')
    expect(html).toContain('<td class="px-2 py-1 align-top">2</td>')
    expect(html).toContain('<blockquote')
    expect(html).toContain('<hr')
  })

  it('leaves markup inside code spans alone', () => {
    expect(markdownToHtml('Use `**not bold**` and <b>')).toBe(
      '<p class="leading-6">Use <code class="rounded bg-muted px-1 py-px text-[0.85em]">**not bold**</code> and &lt;b&gt;</p>',
    )
  })

  it('links documents the folder contains and nothing else', () => {
    const html = markdownToHtml(
      'See [the plan](docs/plan.md), `plan.md` and `missing.md`.',
      resolve,
    )
    expect(html.match(/data-doc-path="\/work\/docs\/plan.md"/g)).toHaveLength(2)
    expect(html).toContain('missing.md</code>')
    expect(markdownToHtml('[x](javascript:alert(1))', resolve)).not.toContain('href')
  })

  it('opens a linked document in Mdow when clicked', () => {
    const listener = vi.fn()
    window.addEventListener('mdow:open-document-link', listener)
    const { getByText } = render(
      <CompanionMarkdown text="Open `plan.md`" resolveDocument={resolve} />,
    )
    fireEvent.click(getByText('plan.md'))
    window.removeEventListener('mdow:open-document-link', listener)

    expect((listener.mock.calls[0][0] as CustomEvent).detail).toEqual({
      path: '/work/docs/plan.md',
    })
  })
})
