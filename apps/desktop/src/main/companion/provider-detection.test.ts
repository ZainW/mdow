import { describe, expect, it, vi } from 'vitest'
import { resolveProviderCommand } from './provider-detection'

vi.mock('../store', () => ({
  getCompanionSettings: () => ({ preferredProvider: null, customCommand: 'my-agent --acp' }),
}))

describe('Companion provider detection', () => {
  it('resolves built-in and custom commands', () => {
    expect(resolveProviderCommand('opencode', '')).toEqual({
      command: 'opencode',
      args: ['acp'],
      display: 'opencode acp',
    })
    expect(resolveProviderCommand('codex-acp', '')).toEqual({
      command: 'codex-acp',
      args: [],
      display: 'codex-acp',
    })
    expect(resolveProviderCommand('custom', 'my-agent --acp')).toEqual({
      command: 'my-agent',
      args: ['--acp'],
      display: 'my-agent --acp',
    })
    expect(resolveProviderCommand('custom', '   ')).toBeNull()
  })
})
