import modelJsonUrl from '@vscode/vscode-languagedetection/model/model.json?url'
import weightsUrl from '@vscode/vscode-languagedetection/model/group1-shard1of1.bin?url'

// vscode-languagedetection emits short language IDs; map them to the shiki
// lang names we have preloaded in markdown.ts. Anything not in this map is
// treated as "no confident detection" and the block stays plaintext.
const ID_TO_SHIKI: Record<string, string> = {
  bat: 'bash',
  c: 'c',
  cpp: 'cpp',
  cs: 'csharp',
  css: 'css',
  dockerfile: 'dockerfile',
  ex: 'elixir',
  go: 'go',
  hs: 'haskell',
  html: 'html',
  java: 'java',
  js: 'javascript',
  json: 'json',
  kt: 'kotlin',
  lua: 'lua',
  md: 'markdown',
  ml: 'ocaml',
  php: 'php',
  ps1: 'bash',
  py: 'python',
  rb: 'ruby',
  rs: 'rust',
  sh: 'bash',
  sql: 'sql',
  swift: 'swift',
  toml: 'toml',
  ts: 'typescript',
  xml: 'xml',
  yaml: 'yaml',
}

const MIN_CONFIDENCE = 0.5
const MIN_LENGTH = 20

const cache = new Map<string, string | null>()

type ModelOps = {
  runModel: (content: string) => Promise<{ languageId: string; confidence: number }[]>
}

let modelPromise: Promise<ModelOps> | null = null

async function getModel(): Promise<ModelOps> {
  if (!modelPromise) {
    modelPromise = (async () => {
      const { ModelOperations } = await import('@vscode/vscode-languagedetection')
      return new ModelOperations({
        modelJsonLoaderFunc: async () => {
          const r = await fetch(modelJsonUrl)
          // eslint-disable-next-line @typescript-eslint/no-unsafe-return, @typescript-eslint/no-unsafe-type-assertion, @typescript-eslint/no-explicit-any -- library expects { [key: string]: any }, untyped fetch JSON
          return r.json() as Promise<{ [k: string]: any }>
        },
        weightsLoaderFunc: async () => {
          const r = await fetch(weightsUrl)
          return r.arrayBuffer()
        },
      })
    })()
  }
  return modelPromise
}

export async function detectLanguage(code: string): Promise<string | null> {
  if (code.length < MIN_LENGTH) return null
  const cached = cache.get(code)
  if (cached !== undefined) return cached

  try {
    const model = await getModel()
    const results = await model.runModel(code)
    const top = results[0]
    const lang =
      top && top.confidence >= MIN_CONFIDENCE ? (ID_TO_SHIKI[top.languageId] ?? null) : null
    cache.set(code, lang)
    return lang
  } catch (err) {
    console.error('Language detection failed:', err)
    cache.set(code, null)
    return null
  }
}
