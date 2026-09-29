import { describe, expect, test } from 'bun:test'
import { htmlToMarkdown } from './html'

const md = (html: string) => htmlToMarkdown(html, '/docs/site')

describe('htmlToMarkdown', () => {
  test('drops scripts, styles and the document head', () => {
    const out = md(
      '<head><title>T</title><style>p{}</style></head><h1>Title</h1><script>alert(1)</script><p>Body</p>',
    )
    expect(out).toBe('# Title\n\nBody\n')
  })

  test('never carries event handler attributes', () => {
    const out = md('<p onclick="steal()">Hi <a href="x.md" onmouseover="bad()">there</a></p>')
    expect(out).not.toContain('steal')
    expect(out).not.toContain('bad')
  })

  test('rewrites relative targets and leaves absolute ones', () => {
    const out = md(
      '<a href="guide.md#setup">g</a> <a href="https://x.dev">w</a> <a href="#top">t</a> <img src="img/a.png" alt="A">',
    )
    expect(out).toContain('[g](</docs/site/guide.md#setup>)')
    expect(out).toContain('[w](<https://x.dev>)')
    expect(out).toContain('[t](<#top>)')
    expect(out).toContain('![A](</docs/site/img/a.png>)')
  })

  test('converts lists, quotes, tables, code and inline styles', () => {
    const out = md(`
      <ol start="3"><li>three <strong>bold</strong></li><li>four <em>it</em></li></ol>
      <ul><li>a<ul><li>nested</li></ul></li></ul>
      <blockquote><p>quoted <del>old</del></p></blockquote>
      <table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>x|y</td></tr></table>
      <pre><code class="language-rust">fn main() {}\n</code></pre>
      <p>use <code>npm i</code></p>`)
    expect(out).toContain('3. three **bold**\n4. four *it*')
    expect(out).toContain('- a\n  - nested')
    expect(out).toContain('> quoted ~~old~~')
    expect(out).toContain('| A | B |\n| --- | --- |\n| 1 | x\\|y |')
    expect(out).toContain('```rust\nfn main() {}\n```')
    expect(out).toContain('use `npm i`')
  })

  test('mermaid pre blocks become mermaid fences', () => {
    expect(md('<pre class="mermaid">graph TD; A--&gt;B</pre>')).toBe(
      '```mermaid\ngraph TD; A-->B\n```\n',
    )
  })

  test('unknown tags flatten to their text and entities decode', () => {
    expect(
      md('<section><custom-thing>Hello &amp; welcome&nbsp;home</custom-thing></section>'),
    ).toBe('Hello & welcome home\n')
  })

  test('drops the doctype', () => {
    expect(md('<!DOCTYPE html>\n<p>Hi</p>')).toBe('Hi\n')
  })

  test('an already-drawn div.mermaid keeps its svg instead of becoming a fence', () => {
    const out = md(
      '<div class="mermaid" data-processed="true"><svg viewBox="0 0 10 10"><text>Start</text></svg></div>',
    )
    expect(out).not.toContain('```mermaid')
  })

  test('div.mermaid becomes a mermaid fence', () => {
    expect(md('<div class="mermaid">\nflowchart LR\n  a --&gt; b\n</div>')).toBe(
      '```mermaid\nflowchart LR\n  a --> b\n```\n',
    )
  })

  test('inline SVG becomes a data-URL image', () => {
    const out = md(
      '<figure><svg viewBox="0 0 10 10" aria-label="Dot"><circle r="4"/></svg><figcaption>A dot</figcaption></figure>',
    )
    const match = /^!\[Dot\]\(<data:image\/svg\+xml;base64,([^>]+)>\)/.exec(out)
    expect(match).not.toBeNull()
    expect(Buffer.from(match![1]!, 'base64').toString()).toContain(
      'xmlns="http://www.w3.org/2000/svg"',
    )
    expect(out).toContain('*A dot*')
  })

  test('nav, aside, details and definition lists read as prose', () => {
    const out = md(`
      <nav><a href="#a">A</a><a href="#b">B</a></nav>
      <aside><p>Note</p></aside>
      <details><summary>More</summary><p>Body</p></details>
      <dl><dt>Term</dt><dd>Meaning</dd></dl>`)
    expect(out).toContain('[A](<#a>) · [B](<#b>)')
    expect(out).toContain('> Note')
    expect(out).toContain('**More**\n\nBody')
    expect(out).toContain('**Term**\n\nMeaning')
  })
})
