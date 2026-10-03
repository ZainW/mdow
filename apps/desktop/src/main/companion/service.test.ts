import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { BrowserWindow } from 'electron'
import type { CompanionSendPayload, CompanionUpdate } from '../../shared/types'
import type { OpencodeEvent } from './opencode-client'

const settings = vi.hoisted(() => ({ lastModel: null as string | null }))
const fake = vi.hoisted(() => ({
  onEvent: null as ((event: OpencodeEvent) => void) | null,
  client: null as Record<string, ReturnType<typeof vi.fn>> | null,
}))

vi.mock('../store', () => ({
  getCompanionSettings: () => ({ ...settings }),
  saveCompanionSettings: (next: { lastModel?: string | null }) => {
    if (next.lastModel !== undefined) settings.lastModel = next.lastModel
  },
}))

vi.mock('./opencode-binary', () => ({
  detectOpencode: vi.fn(async () => ({
    binary: { path: '/bin/opencode', version: '2.0.16' },
    status: { availability: 'available', version: '2.0.16' },
  })),
}))

vi.mock('./opencode-server', () => ({
  startOpencodeServer: vi.fn(async () => ({
    url: 'http://127.0.0.1:1',
    password: 'pw',
    exited: new Promise(() => undefined),
    stop: vi.fn(),
  })),
}))

vi.mock('./opencode-client', () => ({
  OpencodeClient: vi.fn(function OpencodeClient() {
    const client = {
      listModels: vi.fn(async () => [
        { id: 'claude-sonnet-5-5', providerID: 'opencode', name: 'Claude Sonnet 5.5' },
        { id: 'inclusionai/ling', providerID: 'openrouter', name: 'Ling' },
      ]),
      listProviders: vi.fn(async () => [
        { id: 'opencode', name: 'OpenCode Zen' },
        { id: 'openrouter', name: 'OpenRouter' },
      ]),
      defaultModel: vi.fn(async () => null),
      createSession: vi.fn(async () => ({ id: 'ses_1' })),
      setInstructions: vi.fn(async () => undefined),
      switchModel: vi.fn(async () => undefined),
      prompt: vi.fn(async () => undefined),
      interrupt: vi.fn(async () => undefined),
      replyPermission: vi.fn(async () => undefined),
      subscribe: vi.fn(
        (_directory: string, onEvent: (event: OpencodeEvent) => void) =>
          new Promise<void>(() => {
            fake.onEvent = onEvent
          }),
      ),
    }
    fake.client = client
    return client
  }),
}))

const { CompanionService, composePrompt, parseModelValue, sessionDirectoryFor } =
  await import('./service')

function makeWindow() {
  const sent: CompanionUpdate[] = []
  const win = {
    isDestroyed: () => false,
    webContents: { send: (_channel: string, update: CompanionUpdate) => sent.push(update) },
  } as unknown as BrowserWindow
  return { win, sent }
}

const payload: CompanionSendPayload = {
  text: 'Tighten the intro',
  activePath: '/docs/guide/intro.md',
  openFolderPath: '/docs',
  tags: [{ kind: 'file', path: '/docs/notes.md', sourceId: 'tag:/docs/notes.md' }],
}

function emitEvent(type: string, data: Record<string, unknown>) {
  fake.onEvent?.({ type, data: { sessionID: 'ses_1', ...data } })
}

describe('CompanionService', () => {
  beforeEach(() => {
    settings.lastModel = null
    fake.onEvent = null
    fake.client = null
  })

  it('opens a locked-down session in the folder and prompts with what the user is viewing', async () => {
    const { win } = makeWindow()
    const service = new CompanionService(() => win)
    await service.send(payload, win)

    const client = fake.client!
    expect(client.createSession).toHaveBeenCalledWith({
      directory: '/docs',
      model: null,
      permissions: expect.arrayContaining([
        { action: 'shell', resource: '*', effect: 'deny' },
        { action: 'edit', resource: '*', effect: 'ask' },
      ]),
    })
    expect(client.setInstructions).toHaveBeenCalledWith('ses_1', 'mdow', expect.any(String))
    expect(client.prompt).toHaveBeenCalledWith(
      'ses_1',
      '<mdow-context>\nViewing: guide/intro.md\nAttached: notes.md\n</mdow-context>\n\nTighten the intro',
    )
  })

  it('streams updates, waits for review, and finishes the turn', async () => {
    const { win, sent } = makeWindow()
    const service = new CompanionService(() => win)
    await service.send(payload, win)

    emitEvent('session.text.delta', { delta: 'Here is a tighter intro.' })
    emitEvent('session.tool.input.started', { id: 'call_1', name: 'edit' })
    emitEvent('permission.asked', { id: 'per_1', action: 'edit', source: { id: 'call_1' } })
    expect(sent).toContainEqual({ kind: 'delta', text: 'Here is a tighter intro.' })
    expect(sent).toContainEqual(
      expect.objectContaining({ kind: 'change', permissionId: 'per_1', status: 'pending' }),
    )

    await service.replyPermission('per_1', 'approve')
    expect(fake.client!.replyPermission).toHaveBeenCalledWith('ses_1', 'per_1', 'once', undefined)
    await expect(service.replyPermission('per_1', 'approve')).rejects.toThrow('no longer waiting')

    emitEvent('session.execution.succeeded', {})
    expect(sent.at(-1)).toMatchObject({ kind: 'done' })
  })

  it('declines shell requests without asking the user', async () => {
    const { win, sent } = makeWindow()
    const service = new CompanionService(() => win)
    await service.send(payload, win)
    emitEvent('permission.asked', { id: 'per_2', action: 'shell', resources: ['ls'] })
    expect(fake.client!.replyPermission).toHaveBeenCalledWith(
      'ses_1',
      'per_2',
      'reject',
      expect.stringContaining('shell'),
    )
    expect(sent).toEqual([])
  })

  it('reports provider failures as errors', async () => {
    const { win, sent } = makeWindow()
    const service = new CompanionService(() => win)
    await service.send(payload, win)
    emitEvent('session.execution.failed', { error: { message: 'Sign in to OpenCode Go' } })
    expect(sent.at(-1)).toEqual({ kind: 'error', message: 'Sign in to OpenCode Go' })
  })

  it('refuses a second message while one is running', async () => {
    const { win, sent } = makeWindow()
    const service = new CompanionService(() => win)
    await service.send(payload, win)
    await service.send(payload, win)
    expect(fake.client!.prompt).toHaveBeenCalledTimes(1)
    expect(sent.at(-1)).toMatchObject({ kind: 'warning' })
  })

  it('cancel interrupts the session and rejects pending reviews', async () => {
    const { win, sent } = makeWindow()
    const service = new CompanionService(() => win)
    await service.send(payload, win)
    emitEvent('session.tool.input.started', { id: 'call_1', name: 'edit' })
    emitEvent('permission.asked', { id: 'per_1', action: 'edit', source: { id: 'call_1' } })

    await service.cancel()
    expect(fake.client!.interrupt).toHaveBeenCalledWith('ses_1')
    expect(fake.client!.replyPermission).toHaveBeenCalledWith(
      'ses_1',
      'per_1',
      'reject',
      expect.any(String),
    )
    expect(sent.at(-1)).toMatchObject({ kind: 'cancelled' })
  })

  it('lists models grouped by provider and remembers the choice', async () => {
    const { win } = makeWindow()
    const service = new CompanionService(() => win)
    const state = await service.getModels('/docs')
    expect(state.options.map((option) => option.value)).toEqual([
      'opencode/claude-sonnet-5-5',
      'openrouter/inclusionai/ling',
    ])
    expect(state.currentValue).toBeNull()

    await service.setModel('openrouter/inclusionai/ling')
    expect(settings.lastModel).toBe('openrouter/inclusionai/ling')
    expect((await service.getModels('/docs')).currentValue).toBe('openrouter/inclusionai/ling')
    await expect(service.setModel('nope/model')).rejects.toThrow('not available')
  })

  it('starts a new OpenCode session after reset', async () => {
    const { win } = makeWindow()
    const service = new CompanionService(() => win)
    await service.send(payload, win)
    emitEvent('session.execution.succeeded', {})
    await service.reset()
    await service.send(payload, win)
    expect(fake.client!.createSession).toHaveBeenCalledTimes(2)
  })
})

describe('companion helpers', () => {
  it('splits model values on the first slash', () => {
    expect(parseModelValue('openrouter/inclusionai/ling')).toEqual({
      providerID: 'openrouter',
      id: 'inclusionai/ling',
    })
    expect(parseModelValue('no-slash')).toBeNull()
  })

  it('uses the open folder, else the document folder', () => {
    expect(sessionDirectoryFor(payload)).toBe('/docs')
    expect(sessionDirectoryFor({ ...payload, openFolderPath: null })).toBe('/docs/guide')
  })

  it('leaves the prompt alone when there is no context', () => {
    expect(composePrompt({ ...payload, activePath: null, tags: [] }, '/docs')).toBe(
      'Tighten the intro',
    )
  })
})
