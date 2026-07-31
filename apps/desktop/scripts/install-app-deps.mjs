/* oxlint-disable typescript/no-unsafe-assignment, typescript/no-unsafe-call, typescript/no-unsafe-member-access, typescript/no-unsafe-return -- React Doctor does not load Node's types for this typed ESM build script; install-app-deps.d.mts supplies its public type. */
import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { delimiter, dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptPath = fileURLToPath(import.meta.url)
const appDir = resolve(dirname(scriptPath), '..')
const require = createRequire(import.meta.url)

export function withoutInstallCommand(environment) {
  const executableNames = process.platform === 'win32' ? ['pnpm.cmd', 'pnpm.exe', 'pnpm'] : ['pnpm']
  const path = environment.PATH ?? environment.Path ?? ''
  const npmExecPath = path
    .split(delimiter)
    .flatMap((directory) => executableNames.map((name) => join(directory, name)))
    .find(existsSync)
  const sanitized = {
    ...environment,
    npm_execpath: npmExecPath ?? 'pnpm',
    pnpm_config_verify_deps_before_run: 'false',
  }
  delete sanitized.npm_command
  return sanitized
}

function installAppDeps() {
  const cliPath = require.resolve('electron-builder/cli.js')
  const result = spawnSync(process.execPath, [cliPath, 'install-app-deps'], {
    cwd: appDir,
    env: withoutInstallCommand(process.env),
    stdio: 'inherit',
  })

  if (result.error) throw result.error
  if (result.status !== 0) process.exit(result.status ?? 1)
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  installAppDeps()
}
