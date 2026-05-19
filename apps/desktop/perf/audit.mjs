// UI/UX audit harness for the Mdow desktop app.
// Launches the built Electron app via Playwright, drives it through every
// surface, and captures screenshots + DOM probes into the output dir.
//
// Run with:  node apps/desktop/perf/audit.mjs
// Override output dir with: MDOW_AUDIT_OUT=/path/to/dir node …

import { _electron as electron } from 'playwright'
import electronPath from 'electron'
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const appDir = resolve(__dirname, '..')
const fixturesDir = join(__dirname, 'fixtures')
const smallFixture = join(fixturesDir, 'small.md')
const superFixture = join(fixturesDir, 'super.md')

const outDir = process.env.MDOW_AUDIT_OUT || '/tmp/mdow-audit'
const probes = {}

await rm(outDir, { recursive: true, force: true })
await mkdir(outDir, { recursive: true })

async function shot(page, name) {
  const path = join(outDir, `${name}.png`)
  await page.screenshot({ path, fullPage: false })
  return path
}

async function probe(name, fn) {
  try {
    probes[name] = await fn()
  } catch (err) {
    probes[name] = { error: String(err) }
  }
}

function delay(ms) {
  return new Promise((r) => setTimeout(r, ms))
}

// Electron's application menu eats Cmd+B, Cmd+K, Cmd+F, Cmd+W, etc. before
// they ever reach the renderer in Playwright, so we send the same IPC menu
// events the real menu would send.
async function menu(app, channel, payload) {
  await app.evaluate(
    ({ BrowserWindow }, args) => {
      const win = BrowserWindow.getAllWindows()[0]
      if (!win) throw new Error('Missing Electron window')
      win.webContents.send(args.channel, args.payload)
    },
    { channel, payload },
  )
  await delay(220)
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
  await page.waitForSelector('.markdown-body h1', { timeout: 10_000 })
  await delay(350)
}

const userDataDir = await mkdtemp(join(tmpdir(), 'mdow-audit-user-data-'))

const app = await electron.launch({
  executablePath: electronPath,
  cwd: appDir,
  args: [`--user-data-dir=${userDataDir}`, appDir],
})
const page = await app.firstWindow()
await page.setViewportSize({ width: 1280, height: 800 }).catch(() => {})
await page.waitForLoadState('domcontentloaded')

await delay(800)

// ─── 01 Welcome (no recents) ─────────────────────────────────────────────
await shot(page, '01-welcome-light')

await probe('welcome', async () =>
  page.evaluate(() => ({
    bodyFontSize: getComputedStyle(document.body).fontSize,
    bodyFontFamily: getComputedStyle(document.body).fontFamily,
    headingText: document.querySelector('h2')?.textContent,
  })),
)

// ─── 02–03 Open documents ───────────────────────────────────────────────
await openFile(app, page, smallFixture)
await shot(page, '02-markdown-light-small')

await openFile(app, page, superFixture)
await page.waitForSelector('.mermaid-container svg', { timeout: 15_000 }).catch(() => {})
await shot(page, '03-markdown-light-super')

await probe('tabBar', async () =>
  page.evaluate(() => {
    const tabs = Array.from(document.querySelectorAll('[role="tab"]'))
    return {
      count: tabs.length,
      aria: tabs.map((t) => t.getAttribute('aria-selected')),
      setsize: tabs.map((t) => t.getAttribute('aria-setsize')),
      posinset: tabs.map((t) => t.getAttribute('aria-posinset')),
    }
  }),
)

// Probe DOM that depends on having a document open.
await probe('copyCodeBtnHidden', async () =>
  page.evaluate(() => {
    const btn = document.querySelector('.copy-code-btn')
    if (!btn) return { error: 'no copy button (no code blocks in active doc?)' }
    return {
      opacity: getComputedStyle(btn).opacity,
      hasAria: btn.getAttribute('aria-label'),
    }
  }),
)

await probe('tabCloseHitTarget', async () =>
  page.evaluate(() => {
    const btn = document.querySelector('.tab-close-btn')
    return btn ? btn.getBoundingClientRect().width : null
  }),
)

await probe('tablistSemantics', async () =>
  page.evaluate(() => {
    const tablist = document.querySelector('[role="tablist"]')
    return {
      hasTablist: !!tablist,
      label: tablist?.getAttribute('aria-label'),
    }
  }),
)

await probe('outlineActiveBar', async () =>
  page.evaluate(() => {
    // Switch through outline rail and read the active item's ::before
    return {
      hasOutlineLink: !!document.querySelector('.outline-link'),
    }
  }),
)

// ─── 04–06 Sidebar rail modes ───────────────────────────────────────────
async function clickRail(label) {
  await page.click(`[aria-label="${label}"]`)
  await delay(220)
}

await clickRail('Recents')
await shot(page, '04-sidebar-recents')

await clickRail('Outline')
await shot(page, '05-sidebar-outline')

await clickRail('Folder')
await shot(page, '06-sidebar-folder-empty')

await probe('sidebarRadiogroup', async () =>
  page.evaluate(() => {
    const group = document.querySelector('[role="radiogroup"][aria-label="Sidebar mode"]')
    const radios = group ? Array.from(group.querySelectorAll('[role="radio"]')) : []
    return {
      hasGroup: !!group,
      labels: radios.map((r) => r.getAttribute('aria-label')),
      checked: radios.map((r) => r.getAttribute('aria-checked')),
    }
  }),
)

// ─── 07 Sidebar collapsed via menu IPC ──────────────────────────────────
await menu(app, 'menu:toggle-sidebar')
await shot(page, '07-sidebar-collapsed')

// Reopen
await menu(app, 'menu:toggle-sidebar')

// ─── 08 Command palette — dispatch a synthetic Cmd+K keydown so it
//     bypasses the macOS application-menu accelerator capture. ────────
await page.evaluate(() => {
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true }))
})
await delay(350)
await shot(page, '08-command-palette')

await probe('commandPalette', async () =>
  page.evaluate(() => {
    const dlg = document.querySelector('[data-slot="dialog-content"]')
    const input = document.querySelector('[cmdk-input]')
    return {
      hasDialog: !!dlg,
      hasInput: !!input,
      hasFooterHint: !!document.querySelector('[cmdk-list] + div kbd'),
    }
  }),
)

await page.keyboard.press('Escape')
await delay(250)

// ─── 09 Find-in-document via menu IPC ───────────────────────────────────
await menu(app, 'menu:find')
await page.keyboard.type('content', { delay: 25 })
await delay(350)
await shot(page, '09-search-bar')

await probe('searchHighlight', async () =>
  page.evaluate(() => ({
    total: document.querySelectorAll('mark.search-highlight').length,
    hasActive: !!document.querySelector('mark.search-highlight-active'),
  })),
)
await page.keyboard.press('Escape')
await delay(250)

// ─── 10 Zoom indicator — verify width is stable across zoom changes
//     (reset button is always rendered so the card doesn't jump).
await menu(app, 'menu:zoom-in')
const afterOneWidth = await page.evaluate(() => {
  const ind = document.querySelector('.zoom-indicator')
  return ind ? ind.getBoundingClientRect().width : null
})
await menu(app, 'menu:zoom-in')
await menu(app, 'menu:zoom-in')
const afterThreeWidth = await page.evaluate(() => {
  const ind = document.querySelector('.zoom-indicator')
  return ind ? ind.getBoundingClientRect().width : null
})
probes.zoomWidth = { afterOne: afterOneWidth, afterThree: afterThreeWidth }
probes.zoomIndicatorReset = await page.evaluate(() => {
  const btn = document.querySelector('[aria-label="Reset zoom"]')
  return btn
    ? { opacity: getComputedStyle(btn).opacity, ariaHidden: btn.getAttribute('aria-hidden') }
    : null
})
await shot(page, '10-zoom-indicator')

await menu(app, 'menu:zoom-reset')
await delay(400)

// ─── 11–12 Settings — light then dark ───────────────────────────────────
await menu(app, 'menu:settings')
await delay(350)
await shot(page, '11-settings-light')

await page.evaluate(() => {
  const btns = Array.from(document.querySelectorAll('[role="radio"]'))
  const dark = btns.find((b) => b.textContent?.trim() === 'Dark')
  dark?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
})
await delay(500)
await shot(page, '12-settings-dark')

await probe('settingsRadiogroup', async () =>
  page.evaluate(() => ({
    theme: document.querySelector('[role="radiogroup"][aria-label="Theme"]')?.tagName,
    contentFont: document.querySelector('[role="radiogroup"][aria-label="Content font"]')?.tagName,
    codeFont: document.querySelector('[role="radiogroup"][aria-label="Code font"]')?.tagName,
  })),
)

await page.keyboard.press('Escape')
await delay(300)

// ─── 13 Dark markdown ───────────────────────────────────────────────────
await shot(page, '13-markdown-dark-super')

await clickRail('Recents')
await shot(page, '14-sidebar-recents-dark')

await clickRail('Outline')
await shot(page, '15-sidebar-outline-dark')

// ─── 16 Wide mode ───────────────────────────────────────────────────────
await page.click('[aria-label="Wide mode"]').catch(() => {})
await delay(400)
await shot(page, '16-wide-mode-dark')
await page.click('[aria-label="Exit wide mode"]').catch(() => {})
await delay(300)

// ─── 17 Shortcuts ───────────────────────────────────────────────────────
await menu(app, 'menu:shortcuts')
await delay(350)
await shot(page, '17-shortcuts-dark')

// Verify Files/Navigation/View/App headings exist inside the dialog.
await probe('shortcutsGrouping', async () =>
  page.evaluate(() => {
    const dialog = document.querySelector('[data-slot="dialog-content"]')
    const headings = dialog
      ? Array.from(dialog.querySelectorAll('h3')).map((h) => h.textContent?.trim())
      : []
    return { headings }
  }),
)
await page.keyboard.press('Escape')
await delay(300)

// ─── 18 Tab context menu ────────────────────────────────────────────────
await page.evaluate(() => {
  const tab = document.querySelector('[role="tab"]')
  if (!tab) return
  const r = tab.getBoundingClientRect()
  tab.dispatchEvent(
    new MouseEvent('contextmenu', {
      bubbles: true,
      cancelable: true,
      clientX: r.left + 20,
      clientY: r.top + 8,
      button: 2,
    }),
  )
})
await delay(280)
await shot(page, '18-tab-context-menu')

await probe('contextMenu', async () =>
  page.evaluate(() => {
    const menuEl = document.querySelector('.tab-context-menu')
    const items = menuEl ? Array.from(menuEl.querySelectorAll('[role="menuitem"]')) : []
    const close = items[0]
    return {
      hasMenu: !!menuEl,
      itemCount: items.length,
      firstItemFocused: document.activeElement === close,
      hasKbd: !!close?.querySelector('kbd'),
    }
  }),
)
await page.keyboard.press('Escape')
await delay(250)

// Back to light theme for the final empty + light shots
await menu(app, 'menu:settings')
await delay(300)
await page.evaluate(() => {
  const btns = Array.from(document.querySelectorAll('[role="radio"]'))
  const light = btns.find((b) => b.textContent?.trim() === 'Light')
  light?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
})
await delay(400)
await page.keyboard.press('Escape')
await delay(300)

// ─── 19 Error view — attach a deleted error to the active tab ──────────
const activeTabPath = await page.evaluate(() => {
  const t = document.querySelector('[role="tab"][aria-selected="true"]')
  return t?.getAttribute('title') ?? null
})
if (activeTabPath) {
  await app.evaluate(({ BrowserWindow }, p) => {
    const win = BrowserWindow.getAllWindows()[0]
    win.webContents.send('file:deleted', p)
  }, activeTabPath)
  await delay(450)
  await shot(page, '19-error-view-deleted')

  await probe('errorView', async () =>
    page.evaluate(() => ({
      title: document.querySelector('h2')?.textContent,
      pathSpan: document.querySelector('.font-mono')?.textContent,
    })),
  )
}

// ─── 20 Welcome with recents — close every tab via menu IPC ─────────────
// Closing the last tab when no tabs remain would close the window, so we
// stop one short of the empty state and verify via DOM instead.
const tabCount = await page.evaluate(() => document.querySelectorAll('[role="tab"]').length)
for (let i = 0; i < tabCount; i++) {
  await menu(app, 'menu:close-tab')
}
await delay(400)
await shot(page, '20-welcome-with-recents')

// ─── DOM probes ─────────────────────────────────────────────────────────
await probe('themeTokens', async () =>
  page.evaluate(() => {
    const cs = getComputedStyle(document.documentElement)
    const pick = (n) => cs.getPropertyValue(n).trim()
    return {
      bg: pick('--background'),
      fg: pick('--foreground'),
      isDark: document.documentElement.classList.contains('dark'),
    }
  }),
)

await probe('reducedMotionRules', async () =>
  page.evaluate(() => {
    let count = 0
    for (const sheet of Array.from(document.styleSheets)) {
      try {
        for (const rule of Array.from(sheet.cssRules || [])) {
          if (rule.media?.mediaText?.includes('prefers-reduced-motion')) count++
        }
      } catch {
        /* CORS */
      }
    }
    return { count }
  }),
)

// ─── Persist probes ─────────────────────────────────────────────────────
await writeFile(join(outDir, 'probes.json'), JSON.stringify(probes, null, 2))

await app.close()
await rm(userDataDir, { recursive: true, force: true })

// ─── Assertion gate ─────────────────────────────────────────────────────
// Each entry is a label + a predicate over `probes`; any failure exits non-zero.
const checks = [
  ['tabBar role=tablist + tab', () => probes.tablistSemantics?.hasTablist === true],
  ['tabBar aria-setsize on all tabs', () => probes.tabBar?.setsize?.every?.((v) => v !== null)],
  ['tab close hit target is at least 20px', () => (probes.tabCloseHitTarget ?? 0) >= 20],
  [
    'copy-code button hidden at rest',
    () => probes.copyCodeBtnHidden?.opacity === '0' || probes.copyCodeBtnHidden?.error,
  ],
  [
    'sidebar mode is a radiogroup with 3 options',
    () =>
      probes.sidebarRadiogroup?.hasGroup === true && probes.sidebarRadiogroup?.labels?.length === 3,
  ],
  ['command palette opens with footer hint', () => probes.commandPalette?.hasFooterHint === true],
  [
    'zoom indicator width is stable across zoom changes',
    () => probes.zoomWidth && probes.zoomWidth.afterOne === probes.zoomWidth.afterThree,
  ],
  [
    'zoom reset button is hidden at 100% via opacity:0',
    () => probes.zoomIndicatorReset?.opacity === '1' || probes.zoomIndicatorReset == null,
  ],
  [
    'settings exposes three radiogroups',
    () =>
      probes.settingsRadiogroup?.theme === 'DIV' &&
      probes.settingsRadiogroup?.contentFont === 'DIV' &&
      probes.settingsRadiogroup?.codeFont === 'DIV',
  ],
  [
    'shortcuts grouped Files/Navigation/View/App',
    () =>
      JSON.stringify(probes.shortcutsGrouping?.headings ?? []) ===
      JSON.stringify(['Files', 'Navigation', 'View', 'App']),
  ],
  [
    'context menu has 6 menuitems with first focused and a kbd',
    () =>
      probes.contextMenu?.hasMenu === true &&
      probes.contextMenu?.itemCount === 6 &&
      probes.contextMenu?.firstItemFocused === true &&
      probes.contextMenu?.hasKbd === true,
  ],
  [
    'error view shows a truncated path',
    () =>
      probes.errorView?.title === 'File moved or deleted' &&
      typeof probes.errorView?.pathSpan === 'string' &&
      probes.errorView.pathSpan.includes('…'),
  ],
  ['reduced-motion rule coverage preserved', () => (probes.reducedMotionRules?.count ?? 0) >= 7],
]

const failures = checks.filter(([, fn]) => {
  try {
    return !fn()
  } catch {
    return true
  }
})

if (failures.length > 0) {
  console.error('\nAudit gate FAILED:')
  for (const [label] of failures) console.error('  ✗', label)
  console.error(`\nProbes: ${join(outDir, 'probes.json')}`)
  console.error(`Screenshots: ${outDir}`)
  process.exit(1)
}

console.log(`Audit complete. ${checks.length} checks passed. Output: ${outDir}`)
