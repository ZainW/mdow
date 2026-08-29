#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { createConnection, createServer } from 'node:net'
import {
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  rmSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { dirname, isAbsolute, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))

function findRepoRoot(start) {
  let dir = start
  for (;;) {
    if (
      existsSync(join(dir, 'apps/desktop/package.json')) &&
      existsSync(join(dir, 'pnpm-workspace.yaml'))
    ) {
      return dir
    }
    const parent = dirname(dir)
    if (parent === dir) throw new Error('Could not find the mdow repo root (missing apps/desktop)')
    dir = parent
  }
}

const repoRoot = findRepoRoot(__dirname)
const desktopDir = join(repoRoot, 'apps/desktop')
const outMain = join(desktopDir, 'out/main/index.js')
const verifyRoot = join(repoRoot, '.verify-mdow')
const runsDir = join(verifyRoot, 'runs')
const currentPath = join(verifyRoot, 'current')
const sharedDevProfile = 'Mdow Development'
const sharedProdProfileRe = /\/(?:Application Support|apphost)\/Mdow$/

const requireDesktop = createRequire(join(desktopDir, 'package.json'))

function usage() {
  return `Usage:
  verify-mdow launch [--file PATH] [--folder PATH] [--rebuild]
  verify-mdow doctor
  verify-mdow click --role ROLE --name NAME [--exact]
  verify-mdow fill --value TEXT (--role ROLE --name NAME | --placeholder TEXT) [--exact]
  verify-mdow press --key KEY
  verify-mdow wait (--role ROLE --name NAME | --selector CSS | --placeholder TEXT) [--timeout MS]
  verify-mdow text (--role ROLE --name NAME | --selector CSS)
  verify-mdow screenshot --path PATH
  verify-mdow snapshot --aria --path PATH
  verify-mdow status
  verify-mdow cleanup
`
}

function parseArgs(argv) {
  const out = { _: [] }
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    if (token === '--exact' || token === '--aria' || token === '--rebuild' || token === '--force') {
      out[token.slice(2)] = true
      continue
    }
    if (token.startsWith('--')) {
      const key = token.slice(2)
      const next = argv[i + 1]
      if (!next || next.startsWith('--')) {
        out[key] = true
        continue
      }
      out[key] = next
      i += 1
      continue
    }
    out._.push(token)
  }
  return out
}

function electronEnv() {
  const env = { ...process.env }
  delete env.ELECTRON_RUN_AS_NODE
  delete env.ELECTRON_RENDERER_URL
  delete env.VITE_DEV_SERVER_URL
  return env
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true })
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`)
}

function readCurrent() {
  if (!existsSync(currentPath)) return null
  try {
    return readJson(currentPath)
  } catch {
    return null
  }
}

function runPaths(id) {
  const dir = join(runsDir, id)
  return {
    dir,
    userData: join(dir, 'user-data'),
    runJson: join(dir, 'run.json'),
    log: join(dir, 'daemon.log'),
    sock: join('/tmp', `mdow-verify-${id}.sock`),
  }
}

function pidAlive(pid) {
  if (!pid || !Number.isInteger(pid)) return false
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

function isSharedUserData(userData) {
  if (!userData) return true
  if (userData.includes(sharedDevProfile)) return true
  if (sharedProdProfileRe.test(userData)) return true
  return !userData.includes(`${join('.verify-mdow', 'runs')}`)
}

function sendRpc(sockPath, payload, timeoutMs = 20_000) {
  return new Promise((resolvePromise, reject) => {
    const socket = createConnection(sockPath)
    let buf = ''
    const timer = setTimeout(() => {
      socket.destroy()
      reject(new Error(`verify-mdow daemon did not answer within ${timeoutMs}ms`))
    }, timeoutMs)
    socket.on('connect', () => {
      socket.write(`${JSON.stringify(payload)}\n`)
    })
    socket.on('data', (chunk) => {
      buf += chunk.toString('utf8')
      const nl = buf.indexOf('\n')
      if (nl === -1) return
      clearTimeout(timer)
      socket.end()
      try {
        resolvePromise(JSON.parse(buf.slice(0, nl)))
      } catch (err) {
        reject(err)
      }
    })
    socket.on('error', (err) => {
      clearTimeout(timer)
      reject(err)
    })
  })
}

async function pingDaemon(sockPath) {
  try {
    const res = await sendRpc(sockPath, { cmd: 'ping' }, 2_000)
    return res?.ok === true
  } catch {
    return false
  }
}

function ensureBuild(rebuild) {
  if (!rebuild && existsSync(outMain)) return
  const result = spawnSync('pnpm', ['run', '--filter', 'desktop', 'build'], {
    cwd: repoRoot,
    env: electronEnv(),
    stdio: 'inherit',
  })
  if (result.status !== 0) {
    throw new Error(`desktop build failed with exit ${result.status}`)
  }
  if (!existsSync(outMain)) {
    throw new Error(`desktop build did not produce ${outMain}`)
  }
}

function seedFolderStore(userData, folderPath) {
  mkdirSync(userData, { recursive: true })
  writeJson(join(userData, 'config.json'), {
    lastFolder: folderPath,
    sidebarMode: 'folder',
    recents: [],
    sessionTabs: [],
    sessionActiveTabPath: null,
  })
}

async function waitForLive(paths, timeoutMs) {
  const started = Date.now()
  while (Date.now() - started < timeoutMs) {
    if (existsSync(paths.runJson)) {
      const run = readJson(paths.runJson)
      if (run.error) throw new Error(run.error)
      if (run.ready && (await pingDaemon(paths.sock))) return run
    }
    await new Promise((r) => setTimeout(r, 200))
  }
  let extra = ''
  if (existsSync(paths.log)) extra = `\n${readFileSync(paths.log, 'utf8').slice(-2_000)}`
  throw new Error(`verify-mdow launch timed out waiting for the daemon.${extra}`)
}

async function cleanupStale(current) {
  if (!current?.id) return
  const paths = runPaths(current.id)
  if (await pingDaemon(paths.sock)) return false
  if (pidAlive(current.daemonPid) || pidAlive(current.electronPid)) return false
  try {
    unlinkSync(paths.sock)
  } catch {
    // ignore missing socket
  }
  rmSync(paths.dir, { recursive: true, force: true })
  if (existsSync(currentPath)) unlinkSync(currentPath)
  return true
}

async function cmdLaunch(args) {
  const file = args.file ? resolve(args.file) : null
  const folder = args.folder ? resolve(args.folder) : null
  if (file && !existsSync(file)) throw new Error(`--file does not exist: ${file}`)
  if (folder && !existsSync(folder)) throw new Error(`--folder does not exist: ${folder}`)

  const existing = readCurrent()
  if (existing) {
    const existingPaths = runPaths(existing.id)
    if (await pingDaemon(existingPaths.sock)) {
      throw new Error(
        `A verify-mdow run is already live (${existing.id}). Run verify-mdow cleanup first. Do not drive a second instance.`,
      )
    }
    await cleanupStale(existing)
  }

  ensureBuild(Boolean(args.rebuild))

  const id = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
  const paths = runPaths(id)
  mkdirSync(paths.userData, { recursive: true })
  if (folder) seedFolderStore(paths.userData, folder)

  const daemonScript = fileURLToPath(import.meta.url)
  mkdirSync(paths.dir, { recursive: true })
  const logFd = openSync(paths.log, 'w')
  const child = spawn(process.execPath, [daemonScript, '_daemon', id, file ?? '', folder ?? ''], {
    cwd: repoRoot,
    env: electronEnv(),
    detached: true,
    stdio: ['ignore', logFd, logFd],
  })
  child.unref()

  writeJson(currentPath, { id, daemonPid: child.pid })
  try {
    const run = await waitForLive(paths, 60_000)
    writeJson(currentPath, {
      id: run.id,
      daemonPid: run.daemonPid,
      electronPid: run.electronPid,
    })
    process.stdout.write(
      `${JSON.stringify(
        {
          id: run.id,
          ready: run.ready,
          surface: run.surface,
          userData: run.userData,
          file: run.file,
          folder: run.folder,
        },
        null,
        2,
      )}\n`,
    )
  } catch (err) {
    await cmdCleanup()
    throw err
  }
}

async function withRun(fn, timeoutMs) {
  const current = readCurrent()
  if (!current?.id) throw new Error('No verify-mdow run. Launch first.')
  const paths = runPaths(current.id)
  if (!(await pingDaemon(paths.sock))) {
    throw new Error('verify-mdow daemon is not responding. Run doctor, then cleanup if it is dead.')
  }
  const res = await sendRpc(paths.sock, fn, timeoutMs)
  if (!res.ok) throw new Error(res.error || 'daemon command failed')
  return res.result
}

function printResult(result) {
  if (result === undefined) return
  if (typeof result === 'string') {
    process.stdout.write(`${result}\n`)
    return
  }
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`)
}

async function cmdCleanup() {
  const current = readCurrent()
  if (!current?.id) {
    process.stdout.write('No verify-mdow run to clean up.\n')
    return
  }
  const paths = runPaths(current.id)
  if (await pingDaemon(paths.sock)) {
    try {
      await sendRpc(paths.sock, { cmd: 'close' }, 15_000)
    } catch {
      // fall through to pid kill
    }
  }
  const run = existsSync(paths.runJson) ? readJson(paths.runJson) : current
  const deadline = Date.now() + 8_000
  while (Date.now() < deadline && (pidAlive(run.electronPid) || pidAlive(run.daemonPid))) {
    await new Promise((r) => setTimeout(r, 150))
  }
  if (pidAlive(run.electronPid)) {
    try {
      process.kill(run.electronPid, 'SIGTERM')
    } catch {
      // already gone
    }
  }
  if (pidAlive(run.daemonPid)) {
    try {
      process.kill(run.daemonPid, 'SIGTERM')
    } catch {
      // already gone
    }
  }
  try {
    unlinkSync(paths.sock)
  } catch {
    // ignore
  }
  rmSync(paths.dir, { recursive: true, force: true })
  if (existsSync(currentPath)) unlinkSync(currentPath)
  process.stdout.write(
    'Cleaned up the verify-mdow instance. Evidence under .verify-mdow/evidence/ was left in place.\n',
  )
}

function locate(page, spec) {
  const exact = Boolean(spec.exact)
  if (spec.placeholder) return page.getByPlaceholder(spec.placeholder, { exact })
  if (spec.selector) {
    let loc = page.locator(spec.selector)
    if (spec.text) loc = loc.filter({ hasText: spec.text })
    return loc
  }
  if (spec.role && spec.name) return page.getByRole(spec.role, { name: spec.name, exact })
  if (spec.role) return page.getByRole(spec.role)
  throw new Error('Need --role/--name, --placeholder, or --selector')
}

function resolveOutPath(path, cwd) {
  if (!path) throw new Error('--path is required')
  return isAbsolute(path) ? path : resolve(cwd || repoRoot, path)
}

async function detectSurface(page) {
  if (
    await page
      .locator('.markdown-body h1')
      .first()
      .isVisible()
      .catch(() => false)
  )
    return 'reader'
  if (
    await page
      .getByRole('heading', { name: 'Mdow' })
      .isVisible()
      .catch(() => false)
  ) {
    return 'welcome'
  }
  return 'unknown'
}

async function runDaemon(id, file, folder) {
  const paths = runPaths(id)
  mkdirSync(paths.dir, { recursive: true })
  const log = (msg) => {
    writeFileSync(paths.log, `${new Date().toISOString()} ${msg}\n`, { flag: 'a' })
  }

  const fail = (error) => {
    writeJson(paths.runJson, { id, error, daemonPid: process.pid })
    log(error)
    process.exit(1)
  }

  let app
  let page
  try {
    const { _electron: electron } = requireDesktop('playwright')
    const electronPath = requireDesktop('electron')
    const launchArgs = [`--user-data-dir=${paths.userData}`, desktopDir]
    if (file) launchArgs.push(file)

    log(`launch ${electronPath} ${launchArgs.join(' ')}`)
    app = await electron.launch({
      executablePath: electronPath,
      cwd: desktopDir,
      args: launchArgs,
      env: electronEnv(),
    })
    page = await app.firstWindow()
    if (file) {
      await page.waitForSelector('.markdown-body h1', { timeout: 20_000 })
    } else {
      await page.getByRole('heading', { name: 'Mdow' }).waitFor({ timeout: 20_000 })
    }
    const surface = await detectSurface(page)
    const userData = await app.evaluate(({ app: electronApp }) => electronApp.getPath('userData'))
    if (isSharedUserData(userData)) {
      await app.close()
      fail(`Refusing to drive a shared profile: ${userData}`)
    }
    const electronPid = app.process()?.pid ?? null
    const run = {
      id,
      ready: true,
      surface,
      userData,
      file,
      folder,
      daemonPid: process.pid,
      electronPid,
      desktopVersion: readJson(join(desktopDir, 'package.json')).version,
    }
    writeJson(paths.runJson, run)
    log(`ready surface=${surface} userData=${userData}`)

    const server = createServer((socket) => {
      let buf = ''
      socket.on('data', (chunk) => {
        buf += chunk.toString('utf8')
        const nl = buf.indexOf('\n')
        if (nl === -1) return
        const req = JSON.parse(buf.slice(0, nl))
        void handleCommand(req)
          .then((result) => {
            socket.write(`${JSON.stringify({ ok: true, result })}\n`)
            if (req.cmd === 'close') {
              server.close()
              void app.close().finally(() => process.exit(0))
            }
          })
          .catch((err) => {
            socket.write(`${JSON.stringify({ ok: false, error: err.message })}\n`)
          })
      })
    })

    try {
      unlinkSync(paths.sock)
    } catch {
      // ignore
    }
    server.listen(paths.sock)

    async function handleCommand(req) {
      const spec = req
      if (req.cmd === 'ping') return { pong: true }
      if (req.cmd === 'doctor' || req.cmd === 'status') {
        const liveUserData = await app.evaluate(({ app: electronApp }) =>
          electronApp.getPath('userData'),
        )
        const title = await page.title()
        const surfaceNow = await detectSurface(page)
        const heading =
          (await page
            .locator('.markdown-body h1')
            .first()
            .textContent()
            .catch(() => null)) ??
          (await page
            .getByRole('heading', { name: 'Mdow' })
            .textContent()
            .catch(() => null))
        const shared = isSharedUserData(liveUserData)
        if (shared) {
          throw new Error(`Instance is using a shared profile: ${liveUserData}`)
        }
        return {
          id,
          version: run.desktopVersion,
          userData: liveUserData,
          isolated: true,
          windowTitle: title,
          surface: surfaceNow,
          heading: heading?.trim() ?? null,
          electronPid,
          daemonPid: process.pid,
          file,
          folder,
        }
      }
      if (req.cmd === 'click') {
        await locate(page, spec).click({ timeout: spec.timeout ?? 8_000 })
        return { clicked: spec.name || spec.placeholder || spec.selector }
      }
      if (req.cmd === 'fill') {
        if (typeof spec.value !== 'string') throw new Error('--value is required')
        await locate(page, spec).fill(spec.value, { timeout: spec.timeout ?? 8_000 })
        return { filled: spec.value }
      }
      if (req.cmd === 'press') {
        if (!spec.key) throw new Error('--key is required')
        await page.keyboard.press(spec.key)
        return { pressed: spec.key }
      }
      if (req.cmd === 'wait') {
        await locate(page, spec)
          .first()
          .waitFor({
            state: 'visible',
            timeout: Number(spec.timeout ?? 10_000),
          })
        return { visible: spec.name || spec.placeholder || spec.selector }
      }
      if (req.cmd === 'text') {
        const text = (await locate(page, spec).first().innerText()).trim()
        return text
      }
      if (req.cmd === 'screenshot') {
        const out = resolveOutPath(spec.path, spec.cwd)
        mkdirSync(dirname(out), { recursive: true })
        await page.screenshot({ path: out })
        return { path: out }
      }
      if (req.cmd === 'snapshot') {
        const out = resolveOutPath(spec.path, spec.cwd)
        mkdirSync(dirname(out), { recursive: true })
        const aria = await page.locator('body').ariaSnapshot()
        writeFileSync(out, `${aria}\n`)
        return { path: out }
      }
      if (req.cmd === 'close') return { closing: true }
      throw new Error(`Unknown command: ${req.cmd}`)
    }
  } catch (err) {
    if (app) {
      try {
        await app.close()
      } catch {
        // ignore
      }
    }
    fail(err instanceof Error ? err.message : String(err))
  }
}

const argv = process.argv.slice(2)
if (argv[0] === '_daemon') {
  const id = argv[1]
  const file = argv[2] || null
  const folder = argv[3] || null
  await runDaemon(id, file, folder)
  // keep event loop alive via the socket server
} else {
  const args = parseArgs(argv)
  const command = args._[0]
  try {
    if (!command || command === 'help' || args.help) {
      process.stdout.write(usage())
      process.exit(command ? 0 : 1)
    }
    if (command === 'launch') await cmdLaunch(args)
    else if (command === 'cleanup') await cmdCleanup()
    else if (command === 'doctor' || command === 'status') {
      printResult(await withRun({ cmd: command }, 10_000))
    } else if (command === 'click') {
      printResult(
        await withRun({
          cmd: 'click',
          role: args.role,
          name: args.name,
          placeholder: args.placeholder,
          selector: args.selector,
          exact: Boolean(args.exact),
        }),
      )
    } else if (command === 'fill') {
      printResult(
        await withRun({
          cmd: 'fill',
          role: args.role,
          name: args.name,
          placeholder: args.placeholder,
          selector: args.selector,
          value: args.value,
          exact: Boolean(args.exact),
        }),
      )
    } else if (command === 'press') {
      printResult(await withRun({ cmd: 'press', key: args.key }))
    } else if (command === 'wait') {
      printResult(
        await withRun(
          {
            cmd: 'wait',
            role: args.role,
            name: args.name,
            placeholder: args.placeholder,
            selector: args.selector,
            text: args.text,
            exact: Boolean(args.exact),
            timeout: args.timeout,
          },
          Number(args.timeout ?? 15_000) + 2_000,
        ),
      )
    } else if (command === 'text') {
      printResult(
        await withRun({
          cmd: 'text',
          role: args.role,
          name: args.name,
          placeholder: args.placeholder,
          selector: args.selector,
          exact: Boolean(args.exact),
        }),
      )
    } else if (command === 'screenshot') {
      printResult(await withRun({ cmd: 'screenshot', path: args.path, cwd: process.cwd() }))
    } else if (command === 'snapshot') {
      if (!args.aria) throw new Error('snapshot requires --aria')
      printResult(await withRun({ cmd: 'snapshot', path: args.path, cwd: process.cwd() }))
    } else {
      throw new Error(`Unknown command: ${command}\n${usage()}`)
    }
  } catch (err) {
    process.stderr.write(`${err instanceof Error ? err.message : err}\n`)
    process.exit(1)
  }
}
