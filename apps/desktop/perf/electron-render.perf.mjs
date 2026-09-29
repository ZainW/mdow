import { _electron as electron } from 'playwright'
import electronPath from 'electron'
import { appendFile, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const appDir = resolve(__dirname, '..')
const fixturesDir = join(__dirname, 'fixtures')
const smallFixture = join(fixturesDir, 'small.md')
const superFixture = join(fixturesDir, 'super.md')

// Override to stress absurd sizes, e.g. MDOW_PERF_HUGE_REPS=10000 (~30MB). The huge budget only
// applies at the default size.
const DEFAULT_HUGE_REPS = 1_000
const hugeReps = Number(process.env.MDOW_PERF_HUGE_REPS) || DEFAULT_HUGE_REPS

const budgets = {
  smallFirstContentMs: 2_500,
  inAppOpenMs: 1_000,
  superFirstContentMs: 3_500,
  superMermaidReadyMs: 8_000,
  // ~3MB: the super fixture repeated 1,000 times. Measured from launch to the last block in the DOM.
  hugeFullRenderMs: 2_500,
  themeToggleStableMs: 64,
  scrollP95FrameMs: 35,
  scrollLongFrames: 8,
}

const SMALL_SAMPLE_COUNT = 5

function median(values) {
  const sorted = values.toSorted((a, b) => a - b)
  return sorted[Math.floor(sorted.length / 2)]
}

function formatSamples(values) {
  return values.map((value) => value.toFixed(1)).join(', ')
}

async function createLargeFixture(workDir) {
  const source = await readFile(superFixture, 'utf8')
  const sections = Array.from({ length: 32 }, (_, i) =>
    [`# Super Fixture Iteration ${i + 1}`, source].join('\n\n'),
  )
  const target = join(workDir, 'super-large.md')
  await writeFile(target, sections.join('\n\n---\n\n'))
  return target
}

async function createHugeFixture(workDir) {
  const source = await readFile(superFixture, 'utf8')
  const sections = Array.from({ length: hugeReps }, (_, i) =>
    [`# Huge Fixture Iteration ${i + 1}`, source].join('\n\n'),
  )
  const target = join(workDir, 'huge.md')
  await writeFile(target, [...sections, '# Huge Fixture End'].join('\n\n---\n\n'))
  return target
}

// Pixels between the scroller's top edge and a heading's top.
function headingOffset(page, id) {
  return page.evaluate((headingId) => {
    const scroller = document.querySelector('[data-markdown-scroller]')
    const heading = document.getElementById(headingId)
    if (!scroller || !heading) return Number.NaN
    return heading.getBoundingClientRect().top - scroller.getBoundingClientRect().top
  }, id)
}

function assertLandsOn(name, offset) {
  // Headings sit a margin below the edge they were scrolled to; anything else means we missed.
  if (!(offset >= -2 && offset < 64)) {
    throw new Error(`${name} landed ${offset}px from the target heading`)
  }
}

// Long documents use estimated block heights until blocks render, so navigation has to land on
// the right content regardless. Checks outline jumps, tab switches, and live reloads.
async function checkHugeNavigation(run, hugeFixture) {
  const { app, page } = run
  await page.getByRole('radio', { name: 'Outline' }).click()
  // Follow an in-document link through the app's own click handler.
  await page.evaluate(() => {
    const link = document.createElement('a')
    link.href = '#huge-fixture-iteration-900'
    document.querySelector('.markdown-body .comark-content')?.prepend(link)
    link.click()
    link.remove()
  })
  await page.waitForTimeout(800)
  assertLandsOn('anchor jump', await headingOffset(page, 'huge-fixture-iteration-900'))
  const activeOutline = await page
    .locator('a.outline-link[data-active="true"]')
    .getAttribute('href')
  // The fixture's own title sits right under the iteration heading, inside the scroll-spy line.
  const expectedActive = ['#huge-fixture-iteration-900', '#super-markdown-performance-fixture-899']
  if (!expectedActive.includes(activeOutline ?? '')) {
    throw new Error(`outline highlighted ${activeOutline} after the jump`)
  }

  const other = join(dirname(hugeFixture), 'other.md')
  await writeFile(other, '# Other Document\n')
  await app.evaluate(
    ({ BrowserWindow }, payload) =>
      BrowserWindow.getAllWindows()[0]?.webContents.send('file:opened', payload),
    { path: other, content: '# Other Document\n' },
  )
  await page.waitForSelector('#other-document')
  await page.locator('[data-tab]').first().click()
  await page.waitForSelector('#huge-fixture-end', { state: 'attached' })
  await page.waitForTimeout(800)
  assertLandsOn('tab switch restore', await headingOffset(page, 'huge-fixture-iteration-900'))

  await appendFile(hugeFixture, '\n\n# Live Reload Marker\n')
  await page.waitForSelector('#live-reload-marker', { state: 'attached', timeout: 10_000 })
  await page.waitForTimeout(800)
  assertLandsOn('live reload', await headingOffset(page, 'huge-fixture-iteration-900'))
  console.log('huge navigation: anchor jump, outline, tab switch, and live reload land on target')
}

async function measureHugeRun(hugeFixture, { checkNavigation = false } = {}) {
  const run = await launchForFile(hugeFixture, 'huge')
  try {
    await run.page.waitForSelector('#huge-fixture-end', { state: 'attached', timeout: 30_000 })
    const fullRenderMs = performance.now() - run.startedAt
    if (checkNavigation) await checkHugeNavigation(run, hugeFixture)
    return fullRenderMs
  } finally {
    await closeRun(run)
  }
}

async function launchForFile(filePath, label) {
  const userDataDir = await mkdtemp(join(tmpdir(), `mdow-${label}-user-data-`))
  const startedAt = performance.now()
  const app = await electron.launch({
    executablePath: electronPath,
    cwd: appDir,
    args: [`--user-data-dir=${userDataDir}`, appDir, filePath],
  })
  const page = await app.firstWindow()
  await page.waitForSelector('.markdown-body h1', { timeout: 30_000 })
  const firstContentMs = performance.now() - startedAt

  return { app, page, userDataDir, firstContentMs, startedAt }
}

async function waitForMermaid(page) {
  const blocks = page.locator('.mermaid-container')
  if ((await blocks.count()) === 0) return 0

  const startedAt = performance.now()
  await blocks.first().scrollIntoViewIfNeeded()
  await page.waitForFunction(
    () => Boolean(document.querySelector('.mermaid-container svg')),
    undefined,
    { timeout: 10_000 },
  )
  return performance.now() - startedAt
}

async function openFileInRunningApp(app, page, filePath) {
  const content = await readFile(filePath, 'utf8')
  const startedAt = performance.now()
  await app.evaluate(
    ({ BrowserWindow }, payload) => {
      const win = BrowserWindow.getAllWindows()[0]
      if (!win) throw new Error('Missing Electron window')
      win.webContents.send('file:opened', payload)
    },
    { path: filePath, content },
  )
  await page.waitForFunction(
    (targetPath) => {
      const fileName = targetPath.split('/').pop() ?? ''
      const activeTabButton = Array.from(
        document.querySelectorAll('button[aria-selected="true"], button[aria-pressed="true"]'),
      ).find((button) => button.getAttribute('aria-label')?.includes(fileName))
      return Boolean(activeTabButton)
    },
    filePath,
    { timeout: 10_000 },
  )
  await page.waitForSelector('.markdown-body h1', { timeout: 10_000 })
  return performance.now() - startedAt
}

async function measureThemeToggle(page) {
  return page.evaluate(async () => {
    const body = document.querySelector('.markdown-body')
    if (!body) throw new Error('Missing markdown body')
    const childCount = body.childElementCount
    if (childCount === 0) throw new Error('Markdown body is empty')

    const samples = []
    for (let i = 0; i < 5; i += 1) {
      const startedAt = performance.now()
      document.documentElement.classList.toggle('dark')

      await new Promise((resolve) => {
        function frame() {
          if (body.childElementCount >= childCount) {
            resolve()
            return
          }
          requestAnimationFrame(frame)
        }
        requestAnimationFrame(frame)
      })
      samples.push(performance.now() - startedAt)
    }

    const sorted = samples.toSorted((a, b) => a - b)
    return {
      medianMs: sorted[Math.floor(sorted.length / 2)] ?? 0,
      samples,
    }
  })
}

async function measureScroll(page) {
  return page.evaluate(async () => {
    const scroller = document.querySelector('.group\\/content')
    if (!(scroller instanceof HTMLElement)) throw new Error('Missing markdown scroller')

    scroller.scrollTop = 0
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame))

    const frameDeltas = []
    let last = performance.now()
    const startedAt = last
    const durationMs = 1_500
    const distance = Math.max(scroller.scrollHeight - scroller.clientHeight, 1)
    const maxScroll = Math.min(distance, 4_000)

    await new Promise((resolveScroll) => {
      function step(now) {
        frameDeltas.push(now - last)
        last = now

        const progress = Math.min((now - startedAt) / durationMs, 1)
        scroller.scrollTop = progress * maxScroll

        if (progress < 1) requestAnimationFrame(step)
        else resolveScroll()
      }
      requestAnimationFrame(step)
    })

    const sorted = frameDeltas.toSorted((a, b) => a - b)
    const p95Index = Math.max(0, Math.ceil(sorted.length * 0.95) - 1)
    return {
      frames: frameDeltas.length,
      minFrameMs: sorted[0] ?? 0,
      medianFrameMs: sorted[Math.floor(sorted.length / 2)] ?? 0,
      p95FrameMs: sorted[p95Index] ?? 0,
      maxFrameMs: sorted[sorted.length - 1] ?? 0,
      longFrames: frameDeltas.filter((ms) => ms > 50).length,
      scrollHeight: scroller.scrollHeight,
    }
  })
}

function assertBudget(name, actual, budget) {
  if (actual > budget) {
    throw new Error(`${name} ${actual.toFixed(1)}ms exceeded budget ${budget}ms`)
  }
}

function printResult(label, result) {
  console.log(
    [
      `${label}:`,
      `firstContent=${result.firstContentMs.toFixed(1)}ms`,
      result.inAppOpenMs == null ? null : `inAppOpen=${result.inAppOpenMs.toFixed(1)}ms`,
      result.mermaidReadyMs == null ? null : `mermaidReady=${result.mermaidReadyMs.toFixed(1)}ms`,
      result.scroll == null ? null : `scrollP95=${result.scroll.p95FrameMs.toFixed(1)}ms`,
      result.scroll == null ? null : `longFrames=${result.scroll.longFrames}`,
      result.scroll == null ? null : `frames=${result.scroll.frames}`,
      result.scroll == null ? null : `scrollHeight=${result.scroll.scrollHeight}`,
      result.themeToggleStableMs == null
        ? null
        : `themeToggle=${result.themeToggleStableMs.toFixed(1)}ms`,
    ]
      .filter(Boolean)
      .join(' '),
  )
}

async function closeRun(run) {
  await run.app.close()
  await rm(run.userDataDir, { recursive: true, force: true })
}

async function measureSmallRun(largeFixture, sampleIndex) {
  const smallRun = await launchForFile(smallFixture, `small-${sampleIndex}`)
  try {
    return {
      firstContentMs: smallRun.firstContentMs,
      inAppOpenMs: await openFileInRunningApp(smallRun.app, smallRun.page, largeFixture),
    }
  } finally {
    await closeRun(smallRun)
  }
}

const workDir = await mkdtemp(join(tmpdir(), 'mdow-electron-perf-'))

try {
  const largeFixture = await createLargeFixture(workDir)

  const smallSamples = []
  for (let i = 0; i < SMALL_SAMPLE_COUNT; i += 1) {
    smallSamples.push(await measureSmallRun(largeFixture, i + 1))
  }
  const smallFirstContentSamples = smallSamples.map((sample) => sample.firstContentMs)
  const inAppOpenSamples = smallSamples.map((sample) => sample.inAppOpenMs)
  const small = {
    firstContentMs: median(smallFirstContentSamples),
    inAppOpenMs: median(inAppOpenSamples),
  }
  printResult('small median', small)
  console.log(`small samples: firstContent=[${formatSamples(smallFirstContentSamples)}]`)
  console.log(`small samples: inAppOpen=[${formatSamples(inAppOpenSamples)}]`)
  assertBudget('small median first content', small.firstContentMs, budgets.smallFirstContentMs)
  assertBudget('median in-app open', small.inAppOpenMs, budgets.inAppOpenMs)

  const hugeFixture = await createHugeFixture(workDir)
  const hugeSamples = [
    await measureHugeRun(hugeFixture),
    await measureHugeRun(hugeFixture, { checkNavigation: true }),
  ]
  const hugeFullRenderMs = Math.min(...hugeSamples)
  const hugeMb = (await stat(hugeFixture)).size / 1_000_000
  console.log(`huge (${hugeMb.toFixed(1)}MB): fullRender=${hugeFullRenderMs.toFixed(1)}ms`)
  if (hugeReps === DEFAULT_HUGE_REPS) {
    assertBudget('huge full render', hugeFullRenderMs, budgets.hugeFullRenderMs)
  }

  const superRun = await launchForFile(largeFixture, 'super')
  try {
    const superResult = {
      firstContentMs: superRun.firstContentMs,
      mermaidReadyMs: await waitForMermaid(superRun.page),
      themeToggle: await measureThemeToggle(superRun.page),
      scroll: await measureScroll(superRun.page),
    }
    superResult.themeToggleStableMs = superResult.themeToggle.medianMs
    printResult('super', superResult)
    console.log(`super samples: themeToggle=[${formatSamples(superResult.themeToggle.samples)}]`)
    assertBudget('super first content', superResult.firstContentMs, budgets.superFirstContentMs)
    assertBudget('super Mermaid ready', superResult.mermaidReadyMs, budgets.superMermaidReadyMs)
    assertBudget(
      'super theme toggle stable',
      superResult.themeToggleStableMs,
      budgets.themeToggleStableMs,
    )
    assertBudget('super scroll p95 frame', superResult.scroll.p95FrameMs, budgets.scrollP95FrameMs)
    if (superResult.scroll.longFrames > budgets.scrollLongFrames) {
      throw new Error(
        `super scroll longFrames ${superResult.scroll.longFrames} exceeded budget ${budgets.scrollLongFrames}`,
      )
    }
  } finally {
    await closeRun(superRun)
  }
} finally {
  await rm(workDir, { recursive: true, force: true })
}
