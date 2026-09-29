/**
 * End-to-end update test on macOS, with a throwaway signing key:
 *   1. package the app with that key's public half embedded,
 *   2. serve a GitHub-shaped release announcing 9.9.9 from localhost,
 *   3. launch the packaged app, click Download, and check the bundle on disk was replaced,
 *   4. repeat with an update signed by a different key and check it is refused.
 *
 * Usage: bun scripts/test-updater.ts
 */
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { launch } from '@gpuix/react/automation'

const NATIVE = resolve(import.meta.dir, '..')
const APP = 'Mdow Native.app'
// The updater refuses to replace an app whose path runs through a symlink, and macOS
// temp dirs sit under /var -> /private/var.
const work = realpathSync(mkdtempSync(join(tmpdir(), 'mdow-updater-test-')))

function run(cmd: string[], env: Record<string, string> = {}) {
  const result = Bun.spawnSync(cmd, {
    cwd: NATIVE,
    env: { ...process.env, ...env },
    stdout: 'inherit',
    stderr: 'inherit',
  })
  if (result.exitCode !== 0) throw new Error(`${cmd.join(' ')} exited ${result.exitCode}`)
}

function generateKey(name: string) {
  const path = join(work, name)
  run(['bun', 'x', 'packager', 'signer', 'generate', '--ci', '--path', path], {
    CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD: '',
  })
  return { path, pub: readFileSync(`${path}.pub`, 'utf8').trim() }
}

function plistVersion(app: string) {
  const result = Bun.spawnSync([
    '/usr/libexec/PlistBuddy',
    '-c',
    'Print :CFBundleShortVersionString',
    join(app, 'Contents/Info.plist'),
  ])
  return result.stdout.toString().trim()
}

/** A copy of the packaged app claiming `version`, tarred and signed like a release asset. */
function makeRelease(built: string, version: string, keyPath: string) {
  const dir = join(work, `feed-${version}-${keyPath.split('/').pop()}`)
  mkdirSync(dir, { recursive: true })
  const staged = join(dir, 'stage', APP)
  cpSync(built, staged, { recursive: true })
  run([
    '/usr/libexec/PlistBuddy',
    '-c',
    `Set :CFBundleShortVersionString ${version}`,
    join(staged, 'Contents/Info.plist'),
  ])
  const tarball = join(dir, `${APP}.tar.gz`)
  // Same as package-mac.sh: AppleDouble `._` entries break the updater's unpack.
  run(['tar', '-czf', tarball, '-C', join(dir, 'stage'), APP], { COPYFILE_DISABLE: '1' })
  run(['bun', 'x', 'packager', 'signer', 'sign', '-k', keyPath, tarball], {
    CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD: '',
  })
  return dir
}

function serve(dir: string, version: string) {
  const name = `${APP}.tar.gz`
  return Bun.serve({
    hostname: '127.0.0.1',
    port: 0,
    fetch(request) {
      const url = new URL(request.url)
      const base = `http://127.0.0.1:${url.port}`
      if (url.pathname === '/release') {
        return Response.json({
          tag_name: `v${version}`,
          assets: [
            { name, browser_download_url: `${base}/asset` },
            { name: `${name}.sig`, browser_download_url: `${base}/asset.sig` },
          ],
        })
      }
      if (url.pathname === '/asset') return new Response(Bun.file(join(dir, name)))
      if (url.pathname === '/asset.sig') return new Response(Bun.file(join(dir, `${name}.sig`)))
      return new Response('not found', { status: 404 })
    },
  })
}

async function attemptUpdate(built: string, feedDir: string, expectSuccess: boolean) {
  const install = join(work, `install-${expectSuccess ? 'good' : 'bad'}`, APP)
  cpSync(built, install, { recursive: true })
  const server = serve(feedDir, '9.9.9')
  const log = join(work, `app-${expectSuccess}.log`)
  // launch() pipes stderr and never reads it; keep the app's own error output.
  const app = await launch({
    command: '/bin/sh',
    args: ['-c', 'exec "$0" 2>>"$1"', join(install, 'Contents/MacOS/MdowNative'), log],
    env: {
      GPUIX_BACKGROUND: '1',
      MDOW_STATE_PATH: join(work, `state-${expectSuccess}.json`),
      MDOW_UPDATE_FEED_URL: `http://127.0.0.1:${server.port}/release`,
    },
  })
  try {
    await app.getByText('Mdow Native 9.9.9 is available').waitFor({ timeoutMs: 20_000 })
    await app.getByTestId('update-action').click()
    const outcome = expectSuccess
      ? 'Update ready. Restart to apply.'
      : "Couldn't install the update. Download it from Releases."
    await app.getByText(outcome).waitFor({ timeoutMs: 60_000 })
  } catch (error) {
    await app.screenshot({ path: join(tmpdir(), 'mdow-updater-failure.png') })
    console.error(readFileSync(log, 'utf8'))
    throw error
  } finally {
    await app.close()
    await server.stop(true)
  }
  const version = plistVersion(install)
  const expected = expectSuccess ? '9.9.9' : plistVersion(built)
  if (version !== expected) throw new Error(`installed version is ${version}, expected ${expected}`)
  console.log(
    expectSuccess ? '✓ signed update installed' : '✓ update with a foreign signature refused',
  )
}

const keyFile = join(NATIVE, 'updater/public-key.txt')
const originalKey = readFileSync(keyFile, 'utf8')
try {
  const good = generateKey('good.key')
  const bad = generateKey('bad.key')
  writeFileSync(keyFile, `${good.pub}\n`)
  const dist = join(work, 'dist')
  run(['bash', join(NATIVE, 'scripts/package-mac.sh')], { DIST_DIR: dist, CI: '' })
  const built = join(dist, APP)
  await attemptUpdate(built, makeRelease(built, '9.9.9', good.path), true)
  await attemptUpdate(built, makeRelease(built, '9.9.9', bad.path), false)
} finally {
  writeFileSync(keyFile, originalKey)
  rmSync(work, { recursive: true, force: true })
}
