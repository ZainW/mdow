/* oxlint-disable eslint/no-await-in-loop -- executable candidates are checked in deterministic order. */
import { access } from 'fs/promises'
import { constants } from 'fs'
import { delimiter, join } from 'path'
import { homedir } from 'os'
import type { AcpMcpServer } from './acp-client'

export type FffExecutableFinder = () => Promise<string | null>

async function isExecutable(path: string): Promise<boolean> {
  try {
    await access(path, process.platform === 'win32' ? constants.F_OK : constants.X_OK)
    return true
  } catch {
    return false
  }
}

async function findFffExecutable(): Promise<string | null> {
  const extension = process.platform === 'win32' ? '.exe' : ''
  const candidates = [
    ...(process.env.PATH ?? '')
      .split(delimiter)
      .filter(Boolean)
      .map((directory) => join(directory, `fff-mcp${extension}`)),
    join(homedir(), '.local', 'bin', `fff-mcp${extension}`),
    `/opt/homebrew/bin/fff-mcp${extension}`,
    `/usr/local/bin/fff-mcp${extension}`,
  ]

  for (const candidate of [...new Set(candidates)]) {
    if (await isExecutable(candidate)) return candidate
  }
  return null
}

export async function resolveFffMcp(
  findExecutable: FffExecutableFinder = findFffExecutable,
): Promise<AcpMcpServer | null> {
  const command = await findExecutable()
  if (!command) return null
  return {
    name: 'fff',
    command,
    args: [],
    env: [],
  }
}
