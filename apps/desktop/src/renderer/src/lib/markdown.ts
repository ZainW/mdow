import { createParse, type ComarkElement, type ComarkNode, type ComarkPlugin } from 'comark'
import highlight from 'comark/plugins/highlight'
import math from 'comark/plugins/math'
import mermaid from 'comark/plugins/mermaid'
import { defineCachedFunction } from 'ocache'

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
    if (typeof child === 'string' && /\n/.test(child)) {
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
    math({ throwOnError: false }),
    mermaid(),
    breaksOutsideCode,
  ],
})

export async function initMarkdown(): Promise<void> {
  await parse('```ts\n//\n```')
  await parse('Inline $x^2$ math')
}

export interface DocHeading {
  level: number
  text: string
  id: string
}

export interface RenderResult {
  tree: Awaited<ReturnType<typeof parse>>
  mermaidBlocks: { id: string; code: string }[]
  headings: DocHeading[]
  frontmatter: Record<string, unknown>
}

function slugifyHeading(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
}

function isElement(node: ComarkNode): node is ComarkElement {
  return Array.isArray(node) && typeof node[0] === 'string'
}

function isNode(value: unknown): value is ComarkNode {
  return typeof value === 'string' || Array.isArray(value)
}

function getNodeText(node: ComarkNode): string {
  if (typeof node === 'string') return node
  if (!isElement(node)) return ''
  return getChildren(node).map(getNodeText).join('')
}

function getChildren(node: ComarkElement): ComarkNode[] {
  const children: ComarkNode[] = []
  for (let i = 2; i < node.length; i++) {
    const child = node[i]
    if (isNode(child)) children.push(child)
  }
  return children
}

function appendClassName(node: ComarkElement, className: string): void {
  const attrs = node[1]
  const existing = attrs.class
  if (typeof existing === 'string' && existing.length > 0) {
    attrs.class = `${existing} ${className}`
  } else {
    attrs.class = className
  }
}

async function renderMarkdownUncached(
  text: string,
  options?: { bypassCache?: boolean },
): Promise<RenderResult> {
  void options
  const tree = await parse(text)

  const mermaidBlocks: { id: string; code: string }[] = []
  let mermaidCounter = 0
  const headings: DocHeading[] = []
  const slugCounts = new Map<string, number>()

  function visit(node: ComarkNode): void {
    if (!isElement(node)) return

    const tag = node[0]
    const attrs = node[1]

    if (/^h[1-6]$/.test(tag)) {
      const headingText = getNodeText(node).trim()
      if (headingText) {
        let id = typeof attrs.id === 'string' ? attrs.id : ''
        if (!id) {
          const base = slugifyHeading(headingText)
          if (base) {
            const count = slugCounts.get(base) ?? 0
            slugCounts.set(base, count + 1)
            id = count === 0 ? base : `${base}-${count}`
            attrs.id = id
          }
        }
        if (id) headings.push({ level: Number(tag.slice(1)), text: headingText, id })
      }
    }

    if (tag === 'mermaid') {
      const code = typeof attrs.content === 'string' ? attrs.content : ''
      const id =
        typeof attrs.id === 'string' && attrs.id.length > 0
          ? attrs.id
          : `mermaid-${mermaidCounter++}`
      attrs.id = id
      appendClassName(node, 'mermaid mermaid-container')
      mermaidBlocks.push({ id, code })
    }

    for (const child of getChildren(node)) visit(child)
  }

  for (const node of tree.nodes) visit(node)

  return {
    tree,
    mermaidBlocks,
    headings,
    frontmatter: tree.frontmatter ?? {},
  }
}

export const renderMarkdown = defineCachedFunction(renderMarkdownUncached, {
  name: 'renderMarkdown',
  maxAge: 3600,
  getKey: (text: string, options?: { bypassCache?: boolean }) => {
    void options
    return text
  },
  shouldBypassCache: (text: string, options?: { bypassCache?: boolean }) => {
    void text
    return options?.bypassCache === true
  },
})
