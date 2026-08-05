// Generates public/changelog/rss.xml from content/changelog.md
// Run: node apps/web/scripts/generate-rss.mjs

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { generateRss } from './rss-generator.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '..')
const changelogPath = join(root, 'content/changelog.md')
const outDir = join(root, 'public/changelog')
const outPath = join(outDir, 'rss.xml')

const raw = readFileSync(changelogPath, 'utf8')
const existingRss = existsSync(outPath) ? readFileSync(outPath, 'utf8') : ''
const sourceDateEpoch = Number(process.env.SOURCE_DATE_EPOCH)
const now = Number.isFinite(sourceDateEpoch) ? new Date(sourceDateEpoch * 1000) : new Date()
const rss = generateRss(raw, existingRss, now)

mkdirSync(outDir, { recursive: true })
writeFileSync(outPath, rss)
console.log(`RSS feed saved to ${outPath}`)
