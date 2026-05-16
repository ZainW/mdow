import { createParse, type ComarkPlugin } from 'comark'
import { renderHTML } from '@comark/html'
import highlight from 'comark/plugins/highlight'
import mermaid from 'comark/plugins/mermaid'

import githubLight from 'shiki/themes/github-light.mjs'
import githubDark from 'shiki/themes/github-dark.mjs'

import langJavascript from 'shiki/langs/javascript.mjs'
import langTypescript from 'shiki/langs/typescript.mjs'
import langPython from 'shiki/langs/python.mjs'
import langRust from 'shiki/langs/rust.mjs'
import langGo from 'shiki/langs/go.mjs'
import langJava from 'shiki/langs/java.mjs'
import langC from 'shiki/langs/c.mjs'
import langCpp from 'shiki/langs/cpp.mjs'
import langCsharp from 'shiki/langs/csharp.mjs'
import langRuby from 'shiki/langs/ruby.mjs'
import langSwift from 'shiki/langs/swift.mjs'
import langKotlin from 'shiki/langs/kotlin.mjs'
import langHtml from 'shiki/langs/html.mjs'
import langCss from 'shiki/langs/css.mjs'
import langJson from 'shiki/langs/json.mjs'
import langYaml from 'shiki/langs/yaml.mjs'
import langToml from 'shiki/langs/toml.mjs'
import langXml from 'shiki/langs/xml.mjs'
import langMarkdown from 'shiki/langs/markdown.mjs'
import langSql from 'shiki/langs/sql.mjs'
import langBash from 'shiki/langs/bash.mjs'
import langShell from 'shiki/langs/shellscript.mjs'
import langDiff from 'shiki/langs/diff.mjs'
import langGraphql from 'shiki/langs/graphql.mjs'
import langDockerfile from 'shiki/langs/dockerfile.mjs'
import langLua from 'shiki/langs/lua.mjs'
import langZig from 'shiki/langs/zig.mjs'
import langElixir from 'shiki/langs/elixir.mjs'
import langHaskell from 'shiki/langs/haskell.mjs'
import langOcaml from 'shiki/langs/ocaml.mjs'
import langJsx from 'shiki/langs/jsx.mjs'
import langTsx from 'shiki/langs/tsx.mjs'
import langPhp from 'shiki/langs/php.mjs'

const langs = [
  langJavascript,
  langTypescript,
  langPython,
  langRust,
  langGo,
  langJava,
  langC,
  langCpp,
  langCsharp,
  langRuby,
  langSwift,
  langKotlin,
  langHtml,
  langCss,
  langJson,
  langYaml,
  langToml,
  langXml,
  langMarkdown,
  langSql,
  langBash,
  langShell,
  langDiff,
  langGraphql,
  langDockerfile,
  langLua,
  langZig,
  langElixir,
  langHaskell,
  langOcaml,
  langJsx,
  langTsx,
  langPhp,
]

// Convert soft line breaks (`\n` inside text nodes) to <br>, GitHub-flavored.
// Skips <pre>/<code> subtrees so the highlight plugin's line separators stay intact.
function walkBreaks(node: unknown): void {
  if (!Array.isArray(node) || node.length <= 2) return
  const arr = node as unknown[]
  const tag = arr[0]
  if (tag === 'pre' || tag === 'code') return
  let modified = false
  const next: unknown[] = []
  for (let i = 2; i < arr.length; i++) {
    const child = arr[i]
    if (typeof child === 'string' && child.includes('\n')) {
      modified = true
      const lines = child.split('\n')
      for (let li = 0; li < lines.length; li++) {
        if (lines[li].length > 0) next.push(lines[li])
        if (li < lines.length - 1) next.push(['br', {}])
      }
    } else {
      next.push(child)
    }
  }
  if (modified) {
    arr.length = 2
    arr.push(...next)
  }
  for (let i = 2; i < arr.length; i++) walkBreaks(arr[i])
}

const breaksOutsideCode: ComarkPlugin = {
  name: 'breaks-outside-code',
  post(state) {
    for (const n of state.tree.nodes) walkBreaks(n)
  },
}

const parse = createParse({
  plugins: [
    highlight({
      registerDefaultThemes: false,
      registerDefaultLanguages: false,
      themes: { light: githubLight, dark: githubDark },
      languages: langs,
    }),
    mermaid(),
    breaksOutsideCode,
  ],
})

export async function initMarkdown(): Promise<void> {
  await parse('```ts\n//\n```')
}

export interface DocHeading {
  level: number
  text: string
  id: string
}

export interface RenderResult {
  html: string
  mermaidBlocks: { id: string; code: string }[]
  headings: DocHeading[]
}

function slugifyHeading(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
}

function escapeAttr(value: string): string {
  return value.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;')
}

export async function renderMarkdown(text: string): Promise<RenderResult> {
  const tree = await parse(text)

  const mermaidBlocks: { id: string; code: string }[] = []
  let mermaidCounter = 0

  const rawHtml = await renderHTML(tree, {
    components: {
      mermaid: ([, attrs]) => {
        const code = typeof attrs.content === 'string' ? attrs.content : ''
        const id = `mermaid-${mermaidCounter++}`
        mermaidBlocks.push({ id, code })
        return `<div class="mermaid" id="${escapeAttr(id)}"></div>`
      },
    },
  })

  // Trusted source: local markdown files rendered by comark, not user-submitted web content.
  const wrapper = document.createElement('div')
  wrapper.innerHTML = rawHtml

  const headings: DocHeading[] = []
  const slugCounts = new Map<string, number>()
  for (const node of wrapper.querySelectorAll('h1, h2, h3, h4')) {
    const headingText = (node.textContent ?? '').trim()
    if (!headingText) continue
    let id = node.id
    if (!id) {
      const base = slugifyHeading(headingText)
      if (!base) continue
      const count = slugCounts.get(base) ?? 0
      slugCounts.set(base, count + 1)
      id = count === 0 ? base : `${base}-${count}`
      node.id = id
    }
    headings.push({ level: Number(node.tagName.slice(1)), text: headingText, id })
  }

  const copyIconSvg =
    '<svg class="copy-icon copy-icon-default" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>'
  const checkIconSvg =
    '<svg class="copy-icon copy-icon-done" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>'

  for (const pre of wrapper.querySelectorAll('pre')) {
    if (pre.parentElement?.classList.contains('code-block-wrapper')) continue
    const code = pre.textContent ?? ''
    const container = document.createElement('div')
    container.className = 'code-block-wrapper relative'
    const copyBtn = document.createElement('button')
    copyBtn.className = 'copy-code-btn'
    copyBtn.type = 'button'
    copyBtn.setAttribute('aria-label', 'Copy code')
    copyBtn.setAttribute('title', 'Copy code')
    copyBtn.setAttribute('data-code', btoa(encodeURIComponent(code)))
    copyBtn.innerHTML = copyIconSvg + checkIconSvg
    pre.replaceWith(container)
    container.appendChild(pre)
    container.appendChild(copyBtn)
  }

  return { html: wrapper.innerHTML, mermaidBlocks, headings }
}
