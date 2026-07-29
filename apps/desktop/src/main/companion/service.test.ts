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

import { CitationStream, CompanionService } from './service'
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

  it('extracts a citation split across streamed deltas without exposing its raw path', () => {
    const stream = new CitationStream(['src:/docs/overview.md'])

    const first = stream.consume('The launch date is October 14 (src:/docs/over')
    const second = stream.consume('view.md).')
    const final = stream.flush()

    expect(first).toEqual({ text: 'The launch date is October 14 ', citationIds: [] })
    expect(second).toEqual({ text: '.', citationIds: ['src:/docs/overview.md'] })
    expect(final).toEqual({ text: '', citationIds: [] })
  })

  it('deduplicates repeated citations while preserving visible text', () => {
    const stream = new CitationStream(['src:/docs/overview.md'])

    const result = stream.consume(
      'See src:/docs/overview.md and src:/docs/overview.md for launch details.',
    )

    expect(result.text).toBe('See  and  for launch details.')
    expect(result.citationIds).toEqual(['src:/docs/overview.md'])
  })

  it('emits a cancelled terminal state without a later completed state', async () => {
    const sent: unknown[] = []
    const cancel = vi.fn().mockResolvedValue(undefined)
    const service = new CompanionService(
      () =>
        ({
          isDestroyed: () => false,
          webContents: {
            send: (_channel: string, update: unknown) => sent.push(update),
          },
        }) as never,
    )
    Object.assign(service, {
      client: { cancel },
      streamingMessageId: 'message-1',
    })

    await service.cancel()

    expect(cancel).toHaveBeenCalledOnce()
    expect(sent).toContainEqual({ kind: 'cancelled', messageId: 'message-1' })
    expect(sent).not.toContainEqual({ kind: 'done', messageId: 'message-1' })
  })

  it('rejects a second prompt while another request is being prepared', async () => {
    const sent: unknown[] = []
    const service = new CompanionService(
      () =>
        ({
          isDestroyed: () => false,
          webContents: {
            send: (_channel: string, update: unknown) => sent.push(update),
          },
        }) as never,
    )
    Object.assign(service, { activeRequestToken: Symbol('active-request') })
    vi.mocked(detectCompanionProviders).mockResolvedValueOnce([])

    await service.send({
      text: 'Second prompt',
      activePath: null,
      openFolderPath: null,
      tags: [],
    })

    expect(sent).toContainEqual({
      kind: 'warning',
      message: 'Wait for the current response or cancel it first.',
    })
  })
})
