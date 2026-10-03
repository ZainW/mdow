import { spawn, type ChildProcess } from 'child_process'
import { randomBytes } from 'crypto'
import { opencodeEnvPath } from './opencode-binary'

const START_TIMEOUT_MS = 20_000
const LISTENING_PATTERN = /listening on (https?:\/\/[^\s]+)/i

export interface OpencodeServerHandle {
  url: string
  /** Basic auth password, generated per launch and never written to disk. */
  password: string
  /** Resolves when the process exits, whether stopped or crashed. */
  exited: Promise<void>
  stop: () => void
}

export function parseListeningUrl(output: string): string | null {
  return LISTENING_PATTERN.exec(output)?.[1] ?? null
}

/**
 * Starts a private `opencode serve` bound to loopback on a free port. Mdow owns this server
 * instead of borrowing the user's background service, so it controls the credentials and the
 * server's lifetime matches the app's.
 */
export function startOpencodeServer(
  binaryPath: string,
  spawnImpl: typeof spawn = spawn,
): Promise<OpencodeServerHandle> {
  const password = randomBytes(24).toString('hex')
  const child: ChildProcess = spawnImpl(binaryPath, ['serve', '--hostname=127.0.0.1', '--port=0'], {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PATH: opencodeEnvPath(), OPENCODE_SERVER_PASSWORD: password },
  })

  const exited = new Promise<void>((resolve) => {
    child.once('exit', () => resolve())
    child.once('error', () => resolve())
  })
  const stop = () => {
    if (child.exitCode === null && !child.killed) child.kill()
  }

  return new Promise((resolve, reject) => {
    let output = ''
    let settled = false
    const settle = (fn: () => void) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      fn()
    }
    const timer = setTimeout(() => {
      stop()
      settle(() => reject(new Error('OpenCode did not start within 20 seconds.')))
    }, START_TIMEOUT_MS)

    const onData = (chunk: Buffer | string) => {
      output = (output + chunk.toString()).slice(-8_000)
      const url = parseListeningUrl(output)
      if (url) settle(() => resolve({ url, password, exited, stop }))
    }
    child.stdout?.on('data', onData)
    child.stderr?.on('data', onData)
    child.once('error', (error) => settle(() => reject(error)))
    child.once('exit', (code) => {
      const detail = output.trim().split('\n').slice(-3).join('\n')
      settle(() =>
        reject(new Error(`OpenCode exited (${code ?? 'signal'})${detail ? `: ${detail}` : ''}`)),
      )
    })
  })
}
