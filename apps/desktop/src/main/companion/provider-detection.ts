/* oxlint-disable eslint/no-await-in-loop -- PATH candidates are checked in deterministic shell order. */
import { access } from 'fs/promises'
import { constants } from 'fs'
import { delimiter, join } from 'path'
import { execFile } from 'child_process'
import { promisify } from 'util'
import type { CompanionProviderId, CompanionProviderStatus } from '../../shared/types'
import { getCompanionSettings } from '../store'

const execFileAsync = promisify(execFile)

async function pathExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath, constants.X_OK)
    return true
  } catch {
    try {
      await access(filePath, constants.F_OK)
      return true
    } catch {
      return false
    }
  }
}

async function findOnPath(command: string): Promise<string | null> {
  const pathEnv = process.env.PATH ?? ''
  const dirs = pathEnv.split(delimiter).filter(Boolean)
  const extensions = process.platform === 'win32' ? ['', '.exe', '.cmd', '.bat'] : ['']

  for (const dir of dirs) {
    for (const ext of extensions) {
      const candidate = join(dir, `${command}${ext}`)
      if (await pathExists(candidate)) return candidate
    }
  }
  return null
}

async function commandResolves(command: string): Promise<boolean> {
  if (await findOnPath(command)) return true
  try {
    const whichCmd = process.platform === 'win32' ? 'where' : 'command'
    const args = process.platform === 'win32' ? [command] : ['-v', command]
    await execFileAsync(whichCmd, args, { timeout: 2000 })
    return true
  } catch {
    return false
  }
}

export interface ProviderCommand {
  command: string
  args: string[]
  display: string
}

export function resolveProviderCommand(
  id: CompanionProviderId,
  customCommand: string,
): ProviderCommand | null {
  switch (id) {
    case 'opencode':
      return { command: 'opencode', args: ['acp'], display: 'opencode acp' }
    case 'codex-acp':
      return { command: 'codex-acp', args: [], display: 'codex-acp' }
    case 'custom': {
      const trimmed = customCommand.trim()
      if (!trimmed) return null
      const parts = trimmed.split(/\s+/).filter(Boolean)
      return {
        command: parts[0],
        args: parts.slice(1),
        display: trimmed,
      }
    }
    default: {
      const exhaustive: never = id
      void exhaustive
      return null
    }
  }
}

export async function detectCompanionProviders(): Promise<CompanionProviderStatus[]> {
  const settings = getCompanionSettings()
  const opencodeAvailable = await commandResolves('opencode')
  const codexAvailable = await commandResolves('codex-acp')
  const custom = settings.customCommand.trim()

  return [
    {
      id: 'opencode',
      label: 'OpenCode',
      commandDisplay: 'opencode acp',
      availability: opencodeAvailable ? 'available' : 'missing',
      detail: opencodeAvailable
        ? undefined
        : 'Install OpenCode, then retry. OpenCode Go models are configured inside OpenCode.',
    },
    {
      id: 'codex-acp',
      label: 'Codex ACP',
      commandDisplay: 'codex-acp',
      availability: codexAvailable ? 'available' : 'missing',
      detail: codexAvailable
        ? undefined
        : 'Install a Codex ACP adapter binary first. Mdow will not run npx install.',
    },
    {
      id: 'custom',
      label: 'Custom command',
      commandDisplay: custom || '(none)',
      availability: custom ? 'available' : 'missing',
      detail: custom
        ? 'Runs as a local subprocess from the main process.'
        : 'Enter a command that speaks ACP over stdio.',
    },
  ]
}
