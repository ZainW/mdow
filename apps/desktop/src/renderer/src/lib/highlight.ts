import type { CSSProperties } from 'react'
import type { HighlighterCore, LanguageRegistration, ThemedToken } from 'shiki/core'

type LanguageModule = LanguageRegistration | LanguageRegistration[]
type LanguageLoader = () => Promise<LanguageModule>

export interface HighlightToken {
  content: string
  style?: CSSProperties
}

export type HighlightedLines = HighlightToken[][]

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
  mts: loadTypescript,
  cts: loadTypescript,
  python: loadPython,
  py: loadPython,
  rust: loadRust,
  rs: loadRust,
  go: loadGo,
  golang: loadGo,
  java: loadJava,
  c: loadC,
  h: loadC,
  cpp: loadCpp,
  cxx: loadCpp,
  'c++': loadCpp,
  hpp: loadCpp,
  csharp: loadCsharp,
  cs: loadCsharp,
  'c#': loadCsharp,
  ruby: loadRuby,
  rb: loadRuby,
  swift: loadSwift,
  kotlin: loadKotlin,
  kt: loadKotlin,
  html: loadHtml,
  htm: loadHtml,
  css: loadCss,
  json: loadJson,
  jsonc: loadJson,
  yaml: loadYaml,
  yml: loadYaml,
  toml: loadToml,
  xml: loadXml,
  svg: loadXml,
  markdown: loadMarkdown,
  md: loadMarkdown,
  mdx: loadMarkdown,
  sql: loadSql,
  bash: loadShell,
  sh: loadShell,
  shell: loadShell,
  shellscript: loadShell,
  zsh: loadShell,
  console: loadShell,
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
  exs: loadElixir,
  haskell: loadHaskell,
  hs: loadHaskell,
  ocaml: loadOcaml,
  ml: loadOcaml,
  jsx: loadJsx,
  tsx: loadTsx,
  php: loadPhp,
}

const LIGHT_THEME = 'github-light'
const DARK_THEME = 'github-dark'
const MAX_CACHED_BLOCKS = 400

let highlighterPromise: Promise<HighlighterCore> | null = null
// Maps a normalized fence language to the grammar name Shiki registered, or null when the
// grammar failed to load. Pending loads live in `languageLoads`.
const resolvedLanguages = new Map<string, string | null>()
const languageLoads = new Map<string, Promise<string | null>>()
const highlightCache = new Map<string, HighlightedLines>()
const styleCache = new Map<string, CSSProperties>()

export function normalizeLanguage(language: string): string {
  return language
    .trim()
    .toLowerCase()
    .replace(/^language-/, '')
    .split(/\s+/)[0]
}

export function isHighlightableLanguage(language: string | undefined): language is string {
  return typeof language === 'string' && normalizeLanguage(language) in languageLoaders
}

function getHighlighter(): Promise<HighlighterCore> {
  highlighterPromise ??= Promise.all([
    import('shiki/core'),
    import('shiki/engine/javascript'),
    import('shiki/themes/github-light.mjs'),
    import('shiki/themes/github-dark.mjs'),
  ]).then(([{ createHighlighterCore }, { createJavaScriptRegexEngine }, light, dark]) =>
    createHighlighterCore({
      themes: [light.default, dark.default],
      langs: [],
      engine: createJavaScriptRegexEngine({ forgiving: true }),
    }),
  )
  return highlighterPromise
}

function ensureLanguage(language: string): Promise<string | null> {
  const normalized = normalizeLanguage(language)
  const resolved = resolvedLanguages.get(normalized)
  if (resolved !== undefined) return Promise.resolve(resolved)

  let pending = languageLoads.get(normalized)
  if (!pending) {
    const loader = languageLoaders[normalized]
    pending = loader
      ? Promise.all([getHighlighter(), loader()])
          .then(async ([highlighter, grammar]) => {
            await highlighter.loadLanguage(grammar)
            // Shiki language modules list embedded grammars first and the main grammar last.
            const main = Array.isArray(grammar) ? grammar.at(-1) : grammar
            return main?.name ?? null
          })
          .catch(() => null)
      : Promise.resolve(null)
    pending = pending.then((name) => {
      resolvedLanguages.set(normalized, name)
      languageLoads.delete(normalized)
      return name
    })
    languageLoads.set(normalized, pending)
  }
  return pending
}

function toCamelCase(property: string): string {
  return property.replace(/-([a-z])/g, (_, char: string) => char.toUpperCase())
}

function tokenStyle(token: ThemedToken): CSSProperties | undefined {
  const htmlStyle = token.htmlStyle
  if (!htmlStyle) return token.color ? { color: token.color } : undefined

  let key = ''
  for (const property in htmlStyle) key += `${property}:${htmlStyle[property]};`
  if (!key) return undefined

  let style = styleCache.get(key)
  if (!style) {
    const next: Record<string, string> = {}
    for (const property in htmlStyle) {
      next[property.startsWith('--') ? property : toCamelCase(property)] = htmlStyle[property]
    }
    styleCache.set(key, next)
    style = next
  }
  return style
}

const WHITESPACE = /^\s+$/

function toLines(tokens: ThemedToken[][]): HighlightedLines {
  return tokens.map((line) => {
    const out: HighlightToken[] = []
    let carry = ''
    for (let i = 0; i < line.length; i++) {
      const token = line[i]
      // Fold whitespace-only tokens into the next token; it keeps the span count down.
      if (i + 1 < line.length && WHITESPACE.test(token.content)) {
        carry += token.content
        continue
      }
      out.push({ content: carry + token.content, style: tokenStyle(token) })
      carry = ''
    }
    if (carry) out.push({ content: carry })
    return out
  })
}

function cacheKey(language: string, code: string): string {
  return `${normalizeLanguage(language)}\u0000${code}`
}

export function getCachedHighlight(code: string, language: string): HighlightedLines | undefined {
  return highlightCache.get(cacheKey(language, code))
}

/**
 * Highlights one code block. Resolves to null when the language has no grammar, so callers can
 * leave the plain text in place.
 */
export async function highlightCode(
  code: string,
  language: string,
): Promise<HighlightedLines | null> {
  const key = cacheKey(language, code)
  const cached = highlightCache.get(key)
  if (cached) return cached

  const lang = await ensureLanguage(language)
  if (!lang) return null

  const highlighter = await getHighlighter()
  const { tokens } = highlighter.codeToTokens(code, {
    lang,
    themes: { light: LIGHT_THEME, dark: DARK_THEME },
  })
  const lines = toLines(tokens)

  highlightCache.set(key, lines)
  if (highlightCache.size > MAX_CACHED_BLOCKS) {
    const oldest = highlightCache.keys().next().value
    if (oldest !== undefined) highlightCache.delete(oldest)
  }
  return lines
}

/** Loads the engine, themes, and the most common grammars ahead of the first code block. */
export async function warmHighlighter(languages: readonly string[] = ['ts', 'bash']) {
  await Promise.all(languages.map((language) => ensureLanguage(language)))
}
