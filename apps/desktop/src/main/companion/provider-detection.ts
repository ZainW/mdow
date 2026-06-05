import { spawn } from 'node:child_process'
import type { CompanionProviderStatus } from '../../shared/types'

interface ProviderCommand {
  command: string
  args: string[]
}

interface ProviderCandidate extends ProviderCommand {
  id: CompanionProviderStatus['id']
  label: string
  displayCommand: string
  installHint?: string
}

const DETECTION_TIMEOUT_MS = 750

const BUILT_IN_CANDIDATES: ProviderCandidate[] = [
  {
    id: 'opencode',
    label: 'opencode',
    command: 'opencode',
    args: ['acp'],
    displayCommand: 'opencode acp',
    installHint: 'Install opencode, then make sure the opencode command is on PATH.',
  },
  {
    id: 'codex',
    label: 'Codex ACP',
    command: 'npx',
    args: ['--no-install', '@zed-industries/codex-acp'],
    displayCommand: 'npx --no-install @zed-industries/codex-acp',
    installHint: 'Install @zed-industries/codex-acp before using the Codex ACP adapter.',
  },
]

export function parseProviderCommand(input: string): ProviderCommand {
  const parts: string[] = []
  const pattern = /"([^"]+)"|'([^']+)'|\S+/g
  for (const match of input.matchAll(pattern)) {
    parts.push(match[1] ?? match[2] ?? match[0])
  }
  const [command = '', ...args] = parts
  return { command, args }
}

async function detectCandidate(candidate: ProviderCandidate): Promise<CompanionProviderStatus> {
  return new Promise((resolve) => {
    let settled = false
    let child: ReturnType<typeof spawn> | null = null

    const finish = (status: CompanionProviderStatus['status'], error?: string) => {
      if (settled) return
      settled = true
      if (child && status === 'available') child.kill()
      resolve({
        id: candidate.id,
        label: candidate.label,
        command: candidate.displayCommand,
        installHint: candidate.installHint,
        status,
        error,
      })
    }

    try {
      child = spawn(candidate.command, candidate.args, {
        stdio: ['ignore', 'ignore', 'ignore'],
        env: { ...process.env, npm_config_yes: 'false' },
      })
    } catch (err) {
      finish('failed', err instanceof Error ? err.message : 'Failed to start provider')
      return
    }

    child.once('error', (err) => finish('missing', err.message))
    child.once('exit', (code) => finish(code === 0 ? 'available' : 'missing', `Exited with ${code}`))
    setTimeout(() => finish('available'), DETECTION_TIMEOUT_MS)
  })
}

export async function detectCompanionProviders(
  customCommand: string,
): Promise<CompanionProviderStatus[]> {
  const candidates = [...BUILT_IN_CANDIDATES]
  const trimmed = customCommand.trim()
  if (trimmed) {
    const parsed = parseProviderCommand(trimmed)
    candidates.push({
      id: 'custom',
      label: 'Custom',
      command: parsed.command,
      args: parsed.args,
      displayCommand: trimmed,
    })
  }
  return Promise.all(candidates.map((candidate) => detectCandidate(candidate)))
}
