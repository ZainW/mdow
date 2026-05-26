// Copies raw doc markdown to public/docs/{slug}.md for direct download / LLM access.
// Run: node apps/web/scripts/sync-docs-md.mjs

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const docsDir = join(__dirname, '../content/docs')
const outDir = join(__dirname, '../public/docs')

mkdirSync(outDir, { recursive: true })

for (const file of readdirSync(docsDir)) {
  if (!file.endsWith('.md')) continue
  const raw = readFileSync(join(docsDir, file), 'utf8')
  writeFileSync(join(outDir, file), raw)
  console.log(`  ✓ docs/${file}`)
}

console.log(`\nSynced docs to ${outDir}`)
