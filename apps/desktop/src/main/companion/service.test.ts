import { describe, expect, it, vi, beforeEach } from 'vitest'

vi.mock('../store', () => ({
  getCompanionSettings: vi.fn(() => ({ preferredProvider: null, customCommand: '' })),
  saveCompanionSettings: vi.fn(),
}))

vi.mock('./provider-detection', async () => {
  const actual =
    await vi.importActual<typeof import('./provider-detection')>('./provider-detection')
  return {
    ...actual,
    detectCompanionProviders: vi.fn(async () => [
      {
        id: 'opencode' as const,
        label: 'OpenCode',
        commandDisplay: 'opencode acp',
        availability: 'available' as const,
      },
    ]),
  }
})

import { CompanionService } from './service'
import { detectCompanionProviders } from './provider-detection'

describe('Companion service', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('reports detection via detectProviders', async () => {
    const service = new CompanionService(() => null)
    const providers = await service.detectProviders()
    expect(detectCompanionProviders).toHaveBeenCalled()
    expect(providers[0]?.id).toBe('opencode')
  })

  it('returns an error when no provider can start', async () => {
    vi.mocked(detectCompanionProviders).mockResolvedValueOnce([
      {
        id: 'opencode',
        label: 'OpenCode',
        commandDisplay: 'opencode acp',
        availability: 'missing',
      },
    ])
    const service = new CompanionService(() => null)
    const result = await service.startSession()
    expect(result.ok).toBe(false)
    expect(result.error).toMatch(/no companion provider/i)
  })
})
