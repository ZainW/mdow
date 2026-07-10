import { describe, expect, it } from 'vitest'

describe('install app dependencies environment', () => {
  it('removes the install command before Electron Builder probes pnpm', async () => {
    // The runtime helper is JavaScript because package lifecycle scripts execute it directly.
    const { withoutInstallCommand } = await import('../../scripts/install-app-deps.mjs')
    const original = {
      npm_command: 'install',
      npm_execpath: '/store/@pacquet/darwin-arm64/pacquet',
      PATH: '/usr/bin',
    }

    expect(withoutInstallCommand(original)).toEqual({
      npm_execpath: 'pnpm',
      PATH: '/usr/bin',
      pnpm_config_verify_deps_before_run: 'false',
    })
    expect(original).toEqual({
      npm_command: 'install',
      npm_execpath: '/store/@pacquet/darwin-arm64/pacquet',
      PATH: '/usr/bin',
    })
  })
})
