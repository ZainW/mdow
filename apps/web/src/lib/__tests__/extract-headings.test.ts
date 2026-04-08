import { describe, it, expect } from 'vitest'
import { extractHeadings } from '../extract-headings'

describe('extractHeadings', () => {
  it('returns h2 and h3 with id, text, and level', () => {
    const html = `
      <h2 id="intro">Intro</h2>
      <p>x</p>
      <h3 id="details">Details</h3>
      <h4 id="ignored">Ignored</h4>
      <h2 id="next">Next</h2>
    `
    const result = extractHeadings(html)
    expect(result).toEqual([
      { id: 'intro', text: 'Intro', level: 2 },
      { id: 'details', text: 'Details', level: 3 },
      { id: 'next', text: 'Next', level: 2 },
    ])
  })

  it('slugifies heading text and assigns id when missing', () => {
    const html = '<h2>Hello World!</h2>'
    const result = extractHeadings(html)
    expect(result).toEqual([{ id: 'hello-world', text: 'Hello World!', level: 2 }])
  })

  it('returns empty array for html with no headings', () => {
    expect(extractHeadings('<p>plain</p>')).toEqual([])
  })
})
