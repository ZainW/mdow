/**
 * Launch the real app once per diagram in a Mermaid gallery and save a screenshot cropped to it.
 * Usage: MDOW_CAPTURE_THEME=light bun scripts/capture-mermaid.ts <out-dir> [gallery.md]
 */
// oxlint-disable no-await-in-loop -- one app launch at a time, in order.
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { launch } from '@gpuix/react/automation'

const [outArg = 'tmp/mermaid', galleryArg] = process.argv.slice(2)
const outDir = resolve(outArg)
const gallery = resolve(
  galleryArg ?? join(import.meta.dir, '../../desktop/perf/fixtures/mermaid-gallery.md'),
)
const theme = process.env.MDOW_CAPTURE_THEME ?? 'light'

// Each `## Title` followed by a ```mermaid fence becomes its own one-diagram document.
const diagrams = [
  ...readFileSync(gallery, 'utf8').matchAll(/^## (.+)\n+(```mermaid\n[\s\S]*?\n```)/gm),
].map((m) => ({ title: m[1]!.trim(), fence: m[2]! }))

mkdirSync(outDir, { recursive: true })
const work = mkdtempSync(join(tmpdir(), 'mdow-mermaid-'))

async function pngWidth(path: string): Promise<number> {
  const out = await Bun.$`sips -g pixelWidth ${path}`.text()
  return Number(/pixelWidth: (\d+)/.exec(out)?.[1] ?? 0)
}

const report: { title: string; file: string; error: boolean }[] = []
for (const [index, { title, fence }] of diagrams.entries()) {
  const slug = `${String(index).padStart(2, '0')}-${title.toLowerCase().replace(/\W+/g, '-')}`
  const doc = join(work, `${slug}.md`)
  writeFileSync(doc, `${fence}\n`)
  const state = join(work, `state-${slug}.json`)
  writeFileSync(
    state,
    JSON.stringify({ theme, windowBounds: { x: 0, y: 0, width: 1000, height: 1400 } }),
  )
  const app = await launch({
    command: 'bun',
    args: [resolve(import.meta.dir, '../src/main.tsx'), doc],
    env: { GPUIX_BACKGROUND: '1', MDOW_STATE_PATH: state, MDOW_DISABLE_UPDATES: '1' },
  })
  try {
    // Diagrams lay out in a worker after the document opens.
    await Bun.sleep(2500)
    const pane = await app.getByTestId('reader-pane').bounds()
    const images = await app.getByType('img').all()
    const img = images.find((node) => node.bounds && node.bounds.x >= pane.x)?.bounds
    const full = join(work, `full-${slug}.png`)
    await app.screenshot({ path: full })
    const scale = (await pngWidth(full)) / (pane.x + pane.width)
    // A diagram crops to its image; an error crops to the top of the reader.
    const box = img
      ? { x: pane.x, y: img.y - 12, width: pane.width, height: img.height + 24 }
      : { x: pane.x, y: pane.y, width: pane.width, height: 420 }
    const top = Math.max(box.y, pane.y)
    const bottom = Math.min(box.y + box.height, pane.y + pane.height)
    const file = `${slug}-${theme}.png`
    await Bun.$`sips -c ${Math.round((bottom - top) * scale)} ${Math.round(box.width * scale)} --cropOffset ${Math.round(top * scale)} ${Math.round(box.x * scale)} ${full} --out ${join(outDir, file)}`.quiet()
    report.push({ title, file, error: !img })
    console.log(`${slug}: ${img ? 'diagram' : 'no diagram'}`)
  } finally {
    await app.close()
  }
}
writeFileSync(join(outDir, `report-${theme}.json`), JSON.stringify(report, null, 2))
console.log(`Saved to ${outDir}`)
