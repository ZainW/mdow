import { createHighlighterCore } from 'shiki/core'
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript'
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

import { init as initMd4x, renderToHtml } from 'md4x/wasm'

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

let highlighter: Awaited<ReturnType<typeof createHighlighterCore>> | null = null
let initialized = false

export async function initMarkdown(): Promise<void> {
  if (initialized) return

  await initMd4x()

  highlighter = await createHighlighterCore({
    themes: [githubLight, githubDark],
    langs,
    engine: createJavaScriptRegexEngine(),
  })

  initialized = true
}

function highlightCode(code: string, lang: string | null): string {
  if (!highlighter) {
    const escaped = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    return `<pre><code>${escaped}</code></pre>`
  }

  const loaded = highlighter.getLoadedLanguages()

  if (lang && loaded.includes(lang)) {
    return highlighter.codeToHtml(code, {
      lang,
      themes: { light: 'github-light', dark: 'github-dark' },
    })
  }

  const escaped = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  return `<pre><code>${escaped}</code></pre>`
}

export interface RenderResult {
  html: string
  mermaidBlocks: { id: string; code: string }[]
}

let mermaidCounter = 0

function wrapWithCopyButton(preElement: Element, rawCode: string): HTMLDivElement {
  const container = document.createElement('div')
  container.className = 'code-block-wrapper group/code relative'
  const copyBtn = document.createElement('button')
  copyBtn.className = 'copy-code-btn'
  copyBtn.setAttribute('data-code', btoa(encodeURIComponent(rawCode)))
  copyBtn.textContent = 'Copy'
  container.appendChild(preElement)
  container.appendChild(copyBtn)
  return container
}

export function renderMarkdown(text: string): RenderResult {
  const rawHtml = renderToHtml(text)

  const wrapper = document.createElement('div')
  wrapper.innerHTML = rawHtml

  const mermaidBlocks: { id: string; code: string }[] = []
  mermaidCounter = 0

  const codeBlocks = wrapper.querySelectorAll('pre > code')
  for (const block of codeBlocks) {
    const langClass = [...block.classList].find((c) => c.startsWith('language-'))
    const lang = langClass ? langClass.replace('language-', '') : null
    const code = block.textContent || ''

    if (lang === 'mermaid') {
      const id = `mermaid-${mermaidCounter++}`
      const mermaidDiv = document.createElement('div')
      mermaidDiv.className = 'mermaid'
      mermaidDiv.id = id
      block.closest('pre')!.replaceWith(mermaidDiv)
      mermaidBlocks.push({ id, code })
    } else {
      const highlighted = highlightCode(code, lang)
      const temp = document.createElement('div')
      temp.innerHTML = highlighted
      const newPre = temp.firstElementChild
      if (newPre) {
        const wrapped = wrapWithCopyButton(newPre, code)
        block.closest('pre')!.replaceWith(wrapped)
      }
    }
  }

  return { html: wrapper.innerHTML, mermaidBlocks }
}
