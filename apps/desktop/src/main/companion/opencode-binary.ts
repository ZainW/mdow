/* oxlint-disable eslint/no-await-in-loop -- candidates are checked in deterministic priority order. */
import { execFile } from 'child_process'
import { constants } from 'fs'
import { access } from 'fs/promises'
import { homedir } from 'os'
import { delimiter, join } from 'path'
import { promisify } from 'util'
import type { CompanionRuntimeStatus } from '../../shared/types'

const execFileAsync = promisify(execFile)

/** Mdow talks to the v2 HTTP API (`opencode serve`), which shipped in OpenCode 2.0. */
export const MIN_OPENCODE_MAJOR = 2

export interface ResolvedOpencode {
  path: string
  version: string
}

// Apps launched from Finder or a desktop entry get a minimal PATH, so the usual install
// locations are searched as well as whatever PATH the app inherited.
export function opencodeSearchDirs(
  env: NodeJS.ProcessEnv = process.env,
  home: string = homedir(),
): string[] {
  const fromPath = (env.PATH ?? '').split(delimiter).filter(Boolean)
  const wellKnown = [
    join(home, '.opencode', 'bin'),
    join(home, '.local', 'bin'),
    join(home, '.bun', 'bin'),
    join(home, '.npm-global', 'bin'),
    '/opt/homebrew/bin',
    '/usr/local/bin',
    '/usr/bin',
  ]
  return [...new Set([...fromPath, ...wellKnown])]
}

/** PATH for the server process, so OpenCode can find git and its own helpers. */
export function opencodeEnvPath(env: NodeJS.ProcessEnv = process.env): string {
  return opencodeSearchDirs(env).join(delimiter)
}

async function isExecutable(path: string): Promise<boolean> {
  try {
    await access(path, constants.X_OK)
    return true
  } catch {
    return false
  }
}

export function parseOpencodeVersion(output: string): string | null {
  const match = /(\d+)\.(\d+)\.(\d+)/.exec(output)
  return match ? match[0] : null
}

export function isSupportedOpencodeVersion(version: string): boolean {
  const major = Number(version.split('.')[0])
  return Number.isFinite(major) && major >= MIN_OPENCODE_MAJOR
}

export async function findOpencodeBinary(): Promise<string | null> {
  const names = process.platform === 'win32' ? ['opencode.exe', 'opencode.cmd'] : ['opencode']
  for (const dir of opencodeSearchDirs()) {
    for (const name of names) {
      const candidate = join(dir, name)
      if (await isExecutable(candidate)) return candidate
    }
  }
  return null
}

export async function detectOpencode(): Promise<{
  status: CompanionRuntimeStatus
  binary: ResolvedOpencode | null
}> {
  const path = await findOpencodeBinary()
  if (!path) {
    return {
      binary: null,
      status: {
        availability: 'missing',
        detail: 'Install OpenCode 2 from opencode.ai, sign in to a provider, then try again.',
      },
    }
  }

  let version: string | null = null
  try {
    const { stdout } = await execFileAsync(path, ['--version'], {
      timeout: 5_000,
      env: { ...process.env, PATH: opencodeEnvPath() },
    })
    version = parseOpencodeVersion(stdout)
  } catch {
    // Reported below as a failed detection.
  }

  if (!version) {
    return {
      binary: null,
      status: { availability: 'failed', detail: `Could not run ${path} --version.` },
    }
  }
  if (!isSupportedOpencodeVersion(version)) {
    return {
      binary: null,
      status: {
        availability: 'outdated',
        version,
        detail: `Mdow needs OpenCode ${MIN_OPENCODE_MAJOR} or newer. Run \`opencode upgrade\`.`,
      },
    }
  }
  return { binary: { path, version }, status: { availability: 'available', version } }
}
