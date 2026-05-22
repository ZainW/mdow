import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { _electron as electron } from 'playwright'
import electronPath from 'electron'

const __dirname = dirname(fileURLToPath(import.meta.url))
const appDir = resolve(__dirname, '..')
const outDir = process.env.MDOW_ICON_COMPARE_OUT || join(__dirname, 'output', 'icons')

function delay(ms) {
  return new Promise((r) => setTimeout(r, ms))
}

async function shot(page, name) {
  const path = join(outDir, `${name}.png`)
  await page.screenshot({ path, fullPage: true })
  return path
}

await rm(outDir, { recursive: true, force: true })
await mkdir(outDir, { recursive: true })

const userDataDir = await mkdtemp(join(tmpdir(), 'mdow-icon-compare-'))

const app = await electron.launch({
  executablePath: electronPath,
  cwd: appDir,
  args: [`--user-data-dir=${userDataDir}`, appDir],
  env: {
    ...process.env,
    VITE_ICON_LAB: 'true',
  },
})

const page = await app.firstWindow()
await page.setViewportSize({ width: 1280, height: 900 }).catch(() => {})
await page.waitForSelector('h1:text("Icon Lab")', { timeout: 15_000 })
await delay(400)

await shot(page, 'icon-lab-light')

await page.click('button:text("Dark")')
await delay(500)
await shot(page, 'icon-lab-dark')

const probes = await page.evaluate(() => ({
  sampleCount: document.querySelectorAll('table tbody tr').length,
  themeDark: document.documentElement.classList.contains('dark'),
}))

await writeFile(join(outDir, 'probes.json'), JSON.stringify(probes, null, 2))

await app.close()
await rm(userDataDir, { recursive: true, force: true })

console.log(`Icon compare complete. Output: ${outDir}`)
console.log(`  icon-lab-light.png`)
console.log(`  icon-lab-dark.png`)

if (probes.sampleCount < 20) {
  console.error(`Expected at least 20 sample icons, got ${probes.sampleCount}`)
  process.exit(1)
}
