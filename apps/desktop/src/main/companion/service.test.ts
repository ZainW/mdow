import { mkdtemp, rm, writeFile } from 'fs/promises'
import { join } from 'path'
import { tmpdir } from 'os'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../store', () => ({
  getCompanionSettings: vi.fn(() => ({
    preferredProvider: null,
    customCommand: '',
    lastModel: null,
  })),
  saveCompanionSettings: vi.fn(),
}))

vi.mock('./provider-detection', async () => {
  const actual =
    await vi.importActual<typeof import('./provider-detection')>('./provider-detection')
  return {
    ...actual,
    detectCompanionProviders: vi.fn(() =>
      Promise.resolve([
        {
          id: 'opencode' as const,
          label: 'OpenCode',
          commandDisplay: 'opencode acp',
          availability: 'available' as const,
        },
      ]),
    ),
  }
})

import { CitationStream, CompanionService } from './service'
import { detectCompanionProviders } from './provider-detection'
import { saveCompanionSettings } from '../store'

describe('Companion service', () => {
  let dir: string

  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(async () => {
    if (dir) await rm(dir, { recursive: true, force: true })
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
    const cancel = vi.fn().mockRejectedValue(new Error('process already exited'))
    const shutdown = vi.fn().mockResolvedValue(undefined)
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
      client: { cancel, shutdown },
      streamingMessageId: 'message-1',
      activeRequestToken: Symbol('active-request'),
    })

    await expect(service.cancel()).resolves.toBeUndefined()
    service.handleClientUpdate({
      kind: 'delta',
      text: 'late chunk',
    })

    expect(cancel).toHaveBeenCalledOnce()
    expect(shutdown).toHaveBeenCalledOnce()
    expect(sent).toContainEqual({ kind: 'cancelled', messageId: 'message-1' })
    expect(sent).not.toContainEqual({ kind: 'delta', text: 'late chunk' })
    expect(sent).not.toContainEqual({ kind: 'done', messageId: 'message-1' })
  })

  it('keeps a streaming response routed to the window that sent the prompt', () => {
    const firstWindowUpdates: unknown[] = []
    const focusedWindowUpdates: unknown[] = []
    const firstWindow = {
      isDestroyed: () => false,
      webContents: {
        send: (_channel: string, update: unknown) => firstWindowUpdates.push(update),
      },
    }
    const focusedWindow = {
      isDestroyed: () => false,
      webContents: {
        send: (_channel: string, update: unknown) => focusedWindowUpdates.push(update),
      },
    }
    const service = new CompanionService(() => focusedWindow as never)
    Object.assign(service, {
      activeWindow: firstWindow,
      activeRequestToken: Symbol('active-request'),
    })

    service.handleClientUpdate({
      kind: 'delta',
      text: 'same window',
    })

    expect(firstWindowUpdates).toContainEqual({ kind: 'delta', text: 'same window' })
    expect(focusedWindowUpdates).toHaveLength(0)
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

  it('reuses unchanged focused content by hash within the live provider session', async () => {
    dir = await mkdtemp(join(tmpdir(), 'mdow-service-'))
    const active = join(dir, 'active.md')
    await writeFile(active, `# Active\n${'important detail '.repeat(300)}`)
    const prompts: string[] = []
    const fakeClient = {
      getSessionId: () => 'session-1',
      prompt: async (prompt: string) => {
        prompts.push(prompt)
      },
    }
    const service = new CompanionService(() => null)
    Object.assign(service, {
      client: fakeClient,
      activeProvider: 'opencode',
      activeCwd: dir,
    })
    const payload = {
      text: 'Summarize this',
      activePath: active,
      openFolderPath: dir,
      tags: [],
      providerId: 'opencode' as const,
    }

    await service.send(payload)
    await service.send(payload)

    expect(prompts[0]).toContain('important detail')
    expect(prompts[1]).toContain('Content unchanged from earlier in this session')
    expect(prompts[1]).not.toContain('important detail')
  })

  it('exposes live models and persists only the confirmed selection', async () => {
    const selectedState = {
      options: [
        {
          value: 'openai/gpt-5.4',
          name: 'GPT-5.4',
          provider: 'openai' as const,
        },
      ],
      currentValue: 'openai/gpt-5.4',
      stale: false,
    }
    const setModel = vi.fn().mockResolvedValue(selectedState)
    const service = new CompanionService(() => null)
    Object.assign(service, {
      client: {
        getModelState: () => selectedState,
        setModel,
      },
    })

    expect(service.getModels()).toEqual(selectedState)
    await expect(service.setModel('openai/gpt-5.4')).resolves.toEqual(selectedState)
    expect(setModel).toHaveBeenCalledWith('openai/gpt-5.4')
    expect(saveCompanionSettings).toHaveBeenCalledWith({ lastModel: 'openai/gpt-5.4' })
  })
})
