// Capture marketing screenshots from the built Electron app.
// Run: pnpm run --filter desktop build && pnpm run --filter desktop screenshots:web

import { _electron as electron } from 'playwright'
import electronPath from 'electron'
import { execFile } from 'node:child_process'
import { mkdir, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'

const execFileAsync = promisify(execFile)

const __dirname = dirname(fileURLToPath(import.meta.url))
const appDir = resolve(__dirname, '..')
const fixturesDir = join(appDir, 'perf/fixtures')
const outDir = resolve(appDir, '../web/public/screenshots')

const WIDTH = 1999
const HEIGHT = 1361

async function delay(ms) {
  return new Promise((r) => setTimeout(r, ms))
}

async function convertPng(name) {
  const png = join(outDir, `${name}.png`)
  const webp = join(outDir, `${name}.webp`)
  const avif = join(outDir, `${name}.avif`)

  await execFileAsync('sips', ['-z', String(HEIGHT), String(WIDTH), png, '--out', png])
  await execFileAsync('cwebp', ['-q', '85', png, '-o', webp])
  await execFileAsync('avifenc', ['--min', '20', '--max', '30', '--speed', '6', png, avif])
}

async function openFile(app, page, filePath) {
  const content = await readFile(filePath, 'utf8')
  await app.evaluate(
    ({ BrowserWindow }, payload) => {
      const win = BrowserWindow.getAllWindows()[0]
      if (!win) throw new Error('Missing Electron window')
      win.webContents.send('file:opened', payload)
    },
    { path: filePath, content },
  )
  await page.waitForSelector('.markdown-body h1', { timeout: 15_000 })
  await delay(500)
}

async function setTheme(page, theme) {
  await page.evaluate((t) => {
    const btns = Array.from(document.querySelectorAll('[role="radio"]'))
    const btn = btns.find((b) => b.textContent?.trim() === t)
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
  }, theme)
  await delay(450)
}

async function openSettings(app) {
  await app.evaluate(({ BrowserWindow }) => {
    const win = BrowserWindow.getAllWindows()[0]
    win.webContents.send('menu:settings')
  })
  await delay(350)
}

async function closeDialog(page) {
  await page.keyboard.press('Escape')
  await delay(250)
}

await rm(outDir, { recursive: true, force: true })
await mkdir(outDir, { recursive: true })

const userDataDir = join(tmpdir(), `mdow-screenshots-${Date.now()}`)
await mkdir(userDataDir, { recursive: true })

const app = await electron.launch({
  executablePath: electronPath,
  cwd: appDir,
  args: [`--user-data-dir=${userDataDir}`, appDir],
})

const page = await app.firstWindow()
await page.setViewportSize({ width: WIDTH, height: HEIGHT }).catch(() => {})
await page.waitForLoadState('domcontentloaded')
await delay(900)

async function capture(name) {
  const path = join(outDir, `${name}.png`)
  await page.screenshot({ path, fullPage: false })
  await convertPng(name)
  console.log(`  ✓ ${name}`)
}

await capture('empty-light')

await openSettings(app)
await setTheme(page, 'Dark')
await closeDialog(page)
await capture('empty-dark')

await openFile(app, page, join(fixturesDir, 'super.md'))
await page.waitForSelector('.mermaid-container svg', { timeout: 20_000 }).catch(() => {})
await delay(600)
await capture('reading-dark')

await openSettings(app)
await setTheme(page, 'Light')
await closeDialog(page)
await delay(400)
await capture('reading-light')

await page.click('[aria-label="Folder"]').catch(() => {})
await delay(300)
await capture('sidebar-light')

await openSettings(app)
await setTheme(page, 'Dark')
await closeDialog(page)
await delay(400)
await capture('sidebar-dark')

await app.close()
await rm(userDataDir, { recursive: true, force: true })

for (const name of [
  'empty-light',
  'empty-dark',
  'reading-light',
  'reading-dark',
  'sidebar-light',
  'sidebar-dark',
]) {
  await rm(join(outDir, `${name}.png`), { force: true })
}

console.log(`\nScreenshots saved to ${outDir}`)
