import { _electron as electron } from 'playwright'
import electronPath from 'electron'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const appDir = resolve(__dirname, '..')
const fixturesDir = join(__dirname, 'fixtures')
const smallFixture = join(fixturesDir, 'small.md')
const superFixture = join(fixturesDir, 'super.md')

const budgets = {
  smallFirstContentMs: 2_500,
  inAppOpenMs: 1_000,
  superFirstContentMs: 3_500,
  superMermaidReadyMs: 8_000,
  themeToggleStableMs: 32,
  scrollP95FrameMs: 35,
  scrollLongFrames: 8,
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

async function launchForFile(filePath, label) {
  const userDataDir = await mkdtemp(join(tmpdir(), `mdow-${label}-user-data-`))
  const startedAt = performance.now()
  const app = await electron.launch({
    executablePath: electronPath,
    cwd: appDir,
    args: [`--user-data-dir=${userDataDir}`, appDir, filePath],
  })
  const page = await app.firstWindow()
  await page.waitForSelector('.markdown-body h1', { timeout: 10_000 })
  const firstContentMs = performance.now() - startedAt

  return { app, page, userDataDir, firstContentMs }
}

async function waitForMermaid(page) {
  const startedAt = performance.now()
  await page.waitForFunction(
    () => {
      const blocks = document.querySelectorAll('.mermaid-container')
      if (blocks.length === 0) return true
      return Array.from(blocks).every((block) => block.querySelector('svg'))
    },
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

    return performance.now() - startedAt
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

const workDir = await mkdtemp(join(tmpdir(), 'mdow-electron-perf-'))

try {
  const largeFixture = await createLargeFixture(workDir)

  const smallRun = await launchForFile(smallFixture, 'small')
  try {
    const small = {
      firstContentMs: smallRun.firstContentMs,
      inAppOpenMs: await openFileInRunningApp(smallRun.app, smallRun.page, largeFixture),
    }
    printResult('small', small)
    assertBudget('small first content', small.firstContentMs, budgets.smallFirstContentMs)
    assertBudget('in-app open', small.inAppOpenMs, budgets.inAppOpenMs)
  } finally {
    await closeRun(smallRun)
  }

  const superRun = await launchForFile(largeFixture, 'super')
  try {
    const superResult = {
      firstContentMs: superRun.firstContentMs,
      mermaidReadyMs: await waitForMermaid(superRun.page),
      themeToggleStableMs: await measureThemeToggle(superRun.page),
      scroll: await measureScroll(superRun.page),
    }
    printResult('super', superResult)
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
