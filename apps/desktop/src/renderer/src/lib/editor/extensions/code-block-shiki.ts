import { Node } from '@tiptap/core'
import { createHighlighter, type Highlighter } from 'shiki'

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

const LANGS = [
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

let highlighterPromise: Promise<Highlighter> | null = null

function getShiki(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({ themes: [githubLight, githubDark], langs: LANGS })
  }
  return highlighterPromise
}

export const CodeBlockShiki = Node.create({
  name: 'codeBlock',
  group: 'block',
  content: 'text*',
  marks: '',
  defining: true,
  code: true,

  addAttributes() {
    return {
      language: {
        default: null,
        parseHTML: (e) => e.getAttribute('data-language'),
      },
    }
  },

  parseHTML() {
    return [{ tag: 'pre', preserveWhitespace: 'full' }]
  },

  renderHTML({ node, HTMLAttributes }) {
    return ['pre', { 'data-language': node.attrs.language ?? '', ...HTMLAttributes }, ['code', 0]]
  },

  addNodeView() {
    return ({ node }) => {
      const dom = document.createElement('pre')
      const code = document.createElement('code')
      code.textContent = node.textContent
      dom.appendChild(code)

      const language = node.attrs.language as string | null
      if (language) {
        void getShiki()
          .then((shiki) => {
            const isDark = document.documentElement.classList.contains('dark')
            try {
              dom.outerHTML = shiki.codeToHtml(node.textContent, {
                lang: language,
                theme: isDark ? 'github-dark' : 'github-light',
              })
            } catch {
              // unknown language — leave plain
            }
          })
          .catch(() => {
            // highlighter unavailable (e.g. test environment) — leave plain
          })
      }

      return { dom, contentDOM: code }
    }
  },
})
