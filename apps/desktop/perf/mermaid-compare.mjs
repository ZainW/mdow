// Capture every Mermaid diagram in a fixture from the built Electron app, in light and dark.
// Run: pnpm run --filter desktop build && node perf/mermaid-compare.mjs <out-dir> [fixture.md]

import { _electron as electron } from 'playwright'
import electronPath from 'electron'
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const appDir = resolve(__dirname, '..')
const outDir = resolve(process.argv[2] ?? join(tmpdir(), 'mdow-mermaid-compare'))
const fixture = resolve(process.argv[3] ?? join(__dirname, 'fixtures/mermaid-gallery.md'))

const delay = (ms) => new Promise((r) => setTimeout(r, ms))

async function setTheme(page, theme) {
  await page.evaluate((t) => window.api.setTheme(t), theme)
  await page.waitForFunction(
    (dark) => document.documentElement.classList.contains('dark') === dark,
    theme === 'dark',
  )
  await delay(300)
}

const DIAGRAMS = '.markdown-body .mermaid-container, .markdown-body .mermaid-error'

async function captureDiagrams(page, theme) {
  const diagrams = page.locator(DIAGRAMS)
  const count = await diagrams.count()
  const results = []
  for (let i = 0; i < count; i++) {
    const el = diagrams.nth(i)
    // oxlint-disable-next-line no-await-in-loop -- diagrams render lazily as they scroll in.
    await el.scrollIntoViewIfNeeded()
    // oxlint-disable-next-line no-await-in-loop
    await page
      .waitForFunction(
        ([selector, index]) => {
          const node = document.querySelectorAll(selector)[index]
          return node?.querySelector('svg') || node?.classList.contains('mermaid-error')
        },
        [DIAGRAMS, i],
        { timeout: 10_000 },
      )
      .catch(() => {})
    // oxlint-disable-next-line no-await-in-loop
    await delay(150)
    // oxlint-disable-next-line no-await-in-loop
    const info = await el.evaluate((node) => {
      let heading = node.previousElementSibling
      while (heading && heading.tagName !== 'H2') heading = heading.previousElementSibling
      return {
        title: heading?.textContent?.trim() ?? 'diagram',
        error: node.classList.contains('mermaid-error') ? node.textContent : null,
        hasSvg: Boolean(node.querySelector('svg')),
      }
    })
    const slug = `${String(i).padStart(2, '0')}-${info.title.toLowerCase().replace(/\W+/g, '-')}`
    const file = `${slug}-${theme}.png`
    // oxlint-disable-next-line no-await-in-loop
    await el.screenshot({ path: join(outDir, file) }).catch((e) => {
      info.error = `screenshot failed: ${e.message}`
    })
    results.push({ ...info, slug, file })
  }
  return results
}

await rm(outDir, { recursive: true, force: true })
await mkdir(outDir, { recursive: true })
const userDataDir = join(tmpdir(), `mdow-mermaid-${Date.now()}`)
await mkdir(userDataDir, { recursive: true })

const app = await electron.launch({
  executablePath: electronPath,
  cwd: appDir,
  args: [`--user-data-dir=${userDataDir}`, appDir],
})
const page = await app.firstWindow()
await page.setViewportSize({ width: 1400, height: 1000 }).catch(() => {})
await page.waitForLoadState('domcontentloaded')
await delay(900)

const content = await readFile(fixture, 'utf8')
await app.evaluate(
  ({ BrowserWindow }, payload) => {
    BrowserWindow.getAllWindows()[0].webContents.send('file:opened', payload)
  },
  { path: fixture, content },
)
await page.waitForSelector('.markdown-body h1', { timeout: 15_000 })
await delay(500)

const report = {}
await setTheme(page, 'light')
report.light = await captureDiagrams(page, 'light')
await setTheme(page, 'dark')
await delay(500)
report.dark = await captureDiagrams(page, 'dark')

await writeFile(join(outDir, 'report.json'), JSON.stringify(report, null, 2))
await app.close()
await rm(userDataDir, { recursive: true, force: true })

for (const r of report.light) console.log(`${r.slug}: ${r.error ? `ERROR ${r.error}` : 'ok'}`)
console.log(`\nSaved to ${outDir}`)
