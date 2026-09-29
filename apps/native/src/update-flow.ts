import type { AvailableUpdate } from '@gpuix/native'
import { openExternal } from './lib/platform'
import { findUpdate, relaunch, RELEASES_URL, updaterAvailable } from './lib/updater'
import { appStore, setUpdate, type UpdateStatus } from './store'

export const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000
export const LAUNCH_CHECK_DELAY_MS = 2000

let pending: AvailableUpdate | null = null

/** Background checks (`manual: false`) stay invisible unless an update exists. */
export async function checkForUpdates({ manual }: { manual: boolean }) {
  if (!updaterAvailable()) {
    setUpdate({ state: 'unavailable', manual })
    return
  }
  const state = appStore().getState().update.state
  if (state === 'checking' || state === 'downloading' || state === 'ready') return
  setUpdate({ state: 'checking', manual })
  try {
    pending = await findUpdate()
    setUpdate(
      pending ? { state: 'available', version: pending.version } : { state: 'up-to-date', manual },
    )
  } catch (error) {
    console.error('mdow: update check failed:', messageOf(error))
    pending = null
    setUpdate({ state: 'failed', manual, stage: 'check', message: messageOf(error) })
  }
}

export async function installUpdate() {
  if (!pending) return
  const version = pending.version
  setUpdate({ state: 'downloading', version })
  try {
    await pending.downloadAndInstall()
    setUpdate({ state: 'ready', version })
  } catch (error) {
    console.error('mdow: update install failed:', messageOf(error))
    setUpdate({ state: 'failed', manual: true, stage: 'install', message: messageOf(error) })
  }
}

export function restartToUpdate(beforeExit: () => void) {
  beforeExit()
  relaunch()
}

export function openReleases() {
  void openExternal(RELEASES_URL)
}

/** Banner copy, or null when this state shows no banner. */
export function bannerCopy(update: UpdateStatus): string | null {
  switch (update.state) {
    case 'checking':
      return update.manual ? 'Checking for updates…' : null
    case 'available':
      return `Mdow Native ${update.version} is available`
    case 'downloading':
      return 'Downloading update…'
    case 'ready':
      return 'Update ready. Restart to apply.'
    case 'up-to-date':
      return update.manual ? "You're on the latest version" : null
    case 'failed':
      if (update.stage === 'install')
        return "Couldn't install the update. Download it from Releases."
      return update.manual ? "Couldn't check for updates. Try again later." : null
    case 'unavailable':
      return update.manual
        ? 'This build has no automatic updates. Download the latest Native app from Releases.'
        : null
    case 'idle':
      return null
  }
}

function messageOf(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}
