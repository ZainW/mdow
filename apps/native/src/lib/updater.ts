import { dirname } from 'node:path'
import { checkUpdate, type AvailableUpdate } from '@gpuix/native'
import pkg from '../../package.json'
import publicKey from '../../updater/public-key.txt' with { type: 'text' }
import { IS_MAC } from './platform'

export const APP_VERSION: string = pkg.version
export const RELEASES_URL = 'https://github.com/ZainW/mdow/releases/latest'
const FEED = 'https://github.com/ZainW/mdow/releases/latest'

/**
 * Updates are signed `.app.tar.gz` / `.AppImage` assets on the GitHub release, verified with
 * the cargo-packager key below. Only packaged builds update: a `bun src/main.tsx` dev run
 * has nothing to replace.
 */
export function updaterAvailable() {
  if (process.env.MDOW_DISABLE_UPDATES === '1') return false
  if (!publicKey.trim()) return false
  if (IS_MAC) return /\.app\/Contents\/MacOS\//.test(process.execPath)
  return !!process.env.APPIMAGE
}

export async function findUpdate(): Promise<AvailableUpdate | null> {
  return checkUpdate(APP_VERSION, {
    endpoints: [process.env.MDOW_UPDATE_FEED_URL || FEED],
    pubkey: publicKey.trim(),
    timeoutMs: 30_000,
  })
}

/** Start the freshly installed app, then quit this one. */
export function relaunch() {
  if (IS_MAC) {
    const bundle = dirname(dirname(dirname(process.execPath)))
    Bun.spawn(['/bin/sh', '-c', `sleep 1; open "${bundle.replace(/"/g, '\\"')}"`], {
      stdio: ['ignore', 'ignore', 'ignore'],
    }).unref()
  } else if (process.env.APPIMAGE) {
    Bun.spawn([process.env.APPIMAGE], { stdio: ['ignore', 'ignore', 'ignore'] }).unref()
  }
  process.exit(0)
}
