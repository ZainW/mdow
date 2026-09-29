/**
 * Launch the real app in the background and save screenshots of its main states.
 * Usage: bun scripts/capture.ts <out-dir> [file-or-folder…]
 */
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { launch } from '@gpuix/react/automation'

const [outDir = 'tmp/shots', ...paths] = process.argv.slice(2)
const theme = process.env.MDOW_CAPTURE_THEME ?? 'dark'
const state = join(mkdtempSync(join(tmpdir(), 'mdow-capture-')), 'state.json')
writeFileSync(state, JSON.stringify({ theme }))
const app = await launch({
  command: 'bun',
  args: [resolve(import.meta.dir, '../src/main.tsx'), ...paths.map((path) => resolve(path))],
  env: { GPUIX_BACKGROUND: '1', MDOW_STATE_PATH: state, MDOW_DISABLE_UPDATES: '1' },
})
const shot = (name: string) => app.screenshot({ path: join(outDir, `${theme}-${name}.png`) })
try {
  await Bun.sleep(1500)
  await shot('main')
  await app.getByTestId('reader-pane').wheel(0, -4000)
  await Bun.sleep(500)
  await shot('scrolled')
  await app.getByTestId('palette-button').click()
  await Bun.sleep(400)
  await app.getByTestId('palette-input').fill('zoom')
  await Bun.sleep(400)
  await shot('palette')
  await app.getByTestId('palette-input').press('escape')
  await Bun.sleep(300)
  await app.getByTestId('find-button').click()
  await Bun.sleep(400)
  await app.getByTestId('find-input').fill('task')
  await Bun.sleep(600)
  await shot('find')
  await app.getByTestId('find-input').press('escape')
  await app.getByTestId('sidebar-mode-outline').click()
  await Bun.sleep(300)
  await shot('outline')
  await app.getByTestId('palette-button').click()
  await app.getByTestId('palette-input').fill('settings')
  await app.getByTestId('palette-input').press('enter')
  await Bun.sleep(400)
  await shot('settings')
} finally {
  await app.close()
}
