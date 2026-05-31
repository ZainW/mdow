import {
  createParse,
  type ComarkElement,
  type ComarkNode,
  type ComarkPlugin,
  type ComarkTree,
} from 'comark'
import { defineCachedFunction } from 'ocache'
import type { LanguageRegistration, ThemeRegistration } from 'shiki'
import { configureRendererCacheStorage } from './cache-storage'

configureRendererCacheStorage()

type ParseFn = ReturnType<typeof createParse>
type LanguageLoader = () => Promise<LanguageRegistration | LanguageRegistration[]>

interface ParserFeatures {
  highlight: boolean
  math: boolean
  mermaid: boolean
}

const loadJavascript = () => import('shiki/langs/javascript.mjs').then((m) => m.default)
const loadTypescript = () => import('shiki/langs/typescript.mjs').then((m) => m.default)
const loadPython = () => import('shiki/langs/python.mjs').then((m) => m.default)
const loadRust = () => import('shiki/langs/rust.mjs').then((m) => m.default)
const loadGo = () => import('shiki/langs/go.mjs').then((m) => m.default)
const loadJava = () => import('shiki/langs/java.mjs').then((m) => m.default)
const loadC = () => import('shiki/langs/c.mjs').then((m) => m.default)
const loadCpp = () => import('shiki/langs/cpp.mjs').then((m) => m.default)
const loadCsharp = () => import('shiki/langs/csharp.mjs').then((m) => m.default)
const loadRuby = () => import('shiki/langs/ruby.mjs').then((m) => m.default)
const loadSwift = () => import('shiki/langs/swift.mjs').then((m) => m.default)
const loadKotlin = () => import('shiki/langs/kotlin.mjs').then((m) => m.default)
const loadHtml = () => import('shiki/langs/html.mjs').then((m) => m.default)
const loadCss = () => import('shiki/langs/css.mjs').then((m) => m.default)
const loadJson = () => import('shiki/langs/json.mjs').then((m) => m.default)
const loadYaml = () => import('shiki/langs/yaml.mjs').then((m) => m.default)
const loadToml = () => import('shiki/langs/toml.mjs').then((m) => m.default)
const loadXml = () => import('shiki/langs/xml.mjs').then((m) => m.default)
const loadMarkdown = () => import('shiki/langs/markdown.mjs').then((m) => m.default)
const loadSql = () => import('shiki/langs/sql.mjs').then((m) => m.default)
const loadBash = () => import('shiki/langs/bash.mjs').then((m) => m.default)
const loadShell = () => import('shiki/langs/shellscript.mjs').then((m) => m.default)
const loadDiff = () => import('shiki/langs/diff.mjs').then((m) => m.default)
const loadGraphql = () => import('shiki/langs/graphql.mjs').then((m) => m.default)
const loadDockerfile = () => import('shiki/langs/dockerfile.mjs').then((m) => m.default)
const loadLua = () => import('shiki/langs/lua.mjs').then((m) => m.default)
const loadZig = () => import('shiki/langs/zig.mjs').then((m) => m.default)
const loadElixir = () => import('shiki/langs/elixir.mjs').then((m) => m.default)
const loadHaskell = () => import('shiki/langs/haskell.mjs').then((m) => m.default)
const loadOcaml = () => import('shiki/langs/ocaml.mjs').then((m) => m.default)
const loadJsx = () => import('shiki/langs/jsx.mjs').then((m) => m.default)
const loadTsx = () => import('shiki/langs/tsx.mjs').then((m) => m.default)
const loadPhp = () => import('shiki/langs/php.mjs').then((m) => m.default)

const languageLoaders: Record<string, LanguageLoader> = {
  javascript: loadJavascript,
  js: loadJavascript,
  mjs: loadJavascript,
  cjs: loadJavascript,
  typescript: loadTypescript,
  ts: loadTypescript,
  python: loadPython,
  py: loadPython,
  rust: loadRust,
  rs: loadRust,
  go: loadGo,
  java: loadJava,
  c: loadC,
  cpp: loadCpp,
  cxx: loadCpp,
  'c++': loadCpp,
  csharp: loadCsharp,
  cs: loadCsharp,
  'c#': loadCsharp,
  ruby: loadRuby,
  rb: loadRuby,
  swift: loadSwift,
  kotlin: loadKotlin,
  kt: loadKotlin,
  html: loadHtml,
  css: loadCss,
  json: loadJson,
  yaml: loadYaml,
  yml: loadYaml,
  toml: loadToml,
  xml: loadXml,
  markdown: loadMarkdown,
  md: loadMarkdown,
  mdx: loadMarkdown,
  sql: loadSql,
  bash: loadBash,
  sh: loadShell,
  shell: loadShell,
  shellscript: loadShell,
  zsh: loadShell,
  diff: loadDiff,
  patch: loadDiff,
  graphql: loadGraphql,
  gql: loadGraphql,
  dockerfile: loadDockerfile,
  docker: loadDockerfile,
  lua: loadLua,
  zig: loadZig,
  elixir: loadElixir,
  ex: loadElixir,
  haskell: loadHaskell,
  hs: loadHaskell,
  ocaml: loadOcaml,
  ml: loadOcaml,
  jsx: loadJsx,
  tsx: loadTsx,
  php: loadPhp,
}

const languagePromises = new Map<string, Promise<LanguageRegistration | LanguageRegistration[]>>()
let themesPromise: Promise<{ light: ThemeRegistration; dark: ThemeRegistration }> | null = null
const parserPromises = new Map<string, Promise<ParseFn>>()

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

const fencePattern = /^ {0,3}(`{3,}|~{3,})([^\n]*)$/gm

function normalizeLanguage(language: string): string {
  return language
    .trim()
    .toLowerCase()
    .replace(/^language-/, '')
    .split(/\s+/)[0]
}

function detectParserFeatures(text: string): ParserFeatures {
  const infos: string[] = []
  for (const match of text.matchAll(fencePattern)) {
    infos.push(match[2]?.trim() ?? '')
  }

  return {
    highlight: infos.some((info) => {
      const language = normalizeLanguage(info)
      return language.length > 0 && language !== 'mermaid'
    }),
    math: /(^|[^\\])\$/.test(text),
    mermaid: infos.some((info) => normalizeLanguage(info) === 'mermaid'),
  }
}

function parserKey(features: ParserFeatures): string {
  return [
    features.highlight ? 'highlight' : 'plain',
    features.math ? 'math' : 'no-math',
    features.mermaid ? 'mermaid' : 'no-mermaid',
  ].join(':')
}

async function loadHighlightThemes(): Promise<{
  light: ThemeRegistration
  dark: ThemeRegistration
}> {
  themesPromise ??= Promise.all([
    import('shiki/themes/github-light.mjs'),
    import('shiki/themes/github-dark.mjs'),
  ]).then(([light, dark]) => ({ light: light.default, dark: dark.default }))
  return themesPromise
}

function loadLanguage(
  language: string,
): Promise<LanguageRegistration | LanguageRegistration[]> | null {
  const normalized = normalizeLanguage(language)
  const loader = languageLoaders[normalized]
  if (!loader) return null

  let promise = languagePromises.get(normalized)
  if (!promise) {
    promise = loader()
    languagePromises.set(normalized, promise)
  }
  return promise
}

function collectCodeLanguages(tree: ComarkTree): string[] {
  const languages = new Set<string>()

  function visit(node: ComarkNode): void {
    if (!isElement(node)) return

    if (node[0] === 'pre') {
      const language = node[1].language
      if (typeof language === 'string' && normalizeLanguage(language) !== 'mermaid') {
        languages.add(language)
      }
    }

    for (const child of getChildren(node)) visit(child)
  }

  for (const node of tree.nodes) visit(node)
  return [...languages]
}

const lazyHighlight: ComarkPlugin = {
  name: 'lazy-highlight',
  async post(state) {
    const codeLanguages = collectCodeLanguages(state.tree)
    if (codeLanguages.length === 0) return

    const languagePromisesToLoad = codeLanguages
      .map((language) => loadLanguage(language))
      .filter((promise): promise is Promise<LanguageRegistration | LanguageRegistration[]> =>
        Boolean(promise),
      )
    if (languagePromisesToLoad.length === 0) return

    const languageResults = await Promise.all(languagePromisesToLoad)
    const languages = languageResults.filter(
      (language): language is LanguageRegistration | LanguageRegistration[] => Boolean(language),
    )
    const themes = await loadHighlightThemes()
    const { highlightCodeBlocks } = await import('comark/plugins/highlight')

    state.tree = await highlightCodeBlocks(state.tree, {
      registerDefaultThemes: false,
      registerDefaultLanguages: false,
      themes,
      languages,
    })
  },
}

async function createParser(features: ParserFeatures): Promise<ParseFn> {
  const plugins: ComarkPlugin[] = []

  if (features.highlight) {
    plugins.push(lazyHighlight)
  }
  if (features.math) {
    const { default: math } = await import('comark/plugins/math')
    plugins.push(math({ throwOnError: false }))
  }
  if (features.mermaid) {
    const { default: mermaid } = await import('comark/plugins/mermaid')
    plugins.push(mermaid())
  }

  plugins.push(breaksOutsideCode)

  return createParse({ plugins })
}

async function getParser(text: string): Promise<ParseFn> {
  const features = detectParserFeatures(text)
  const key = parserKey(features)

  let promise = parserPromises.get(key)
  if (!promise) {
    promise = createParser(features)
    parserPromises.set(key, promise)
  }
  return promise
}

export async function initMarkdown(): Promise<void> {
  const parse = await getParser('plain')
  await parse('```ts\n//\n```')
}

export interface DocHeading {
  level: number
  text: string
  id: string
}

export interface RenderResult {
  tree: ComarkTree
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
  const parse = await getParser(text)
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
