import { describe, expect, it } from 'vitest'
import { resolveFffMcp } from './fff'

describe('FFF MCP resolution', () => {
  it('returns a read-only stdio descriptor when fff-mcp exists', async () => {
    await expect(resolveFffMcp(async () => '/opt/homebrew/bin/fff-mcp')).resolves.toEqual({
      name: 'fff',
      command: '/opt/homebrew/bin/fff-mcp',
      args: [],
      env: [],
    })
  })

  it('stays optional when fff-mcp is unavailable', async () => {
    await expect(resolveFffMcp(async () => null)).resolves.toBeNull()
  })
})
