import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import { CompanionMarkdown } from './markdown'

describe('CompanionMarkdown', () => {
  it('renders the current stream chunk on the first render', () => {
    const html = renderToStaticMarkup(<CompanionMarkdown text="**Ready now**" streaming />)

    expect(html).toContain('<strong>Ready now</strong>')
  })
})
