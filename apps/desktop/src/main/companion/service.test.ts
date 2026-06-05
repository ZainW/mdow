import { afterEach, describe, expect, it, vi } from 'vitest'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { CompanionProviderStatus, CompanionSendRequest } from '../../shared/types'
import { clearAllowedPaths, registerAllowedPath } from '../allowed-paths'
import { createCompanionService } from './service'

const availableOpencode: CompanionProviderStatus = {
  id: 'opencode',
  label: 'opencode',
  command: 'opencode acp',
  status: 'available',
}

const availableCodex: CompanionProviderStatus = {
  id: 'codex',
  label: 'Codex ACP',
  command: 'npx --no-install @zed-industries/codex-acp',
  status: 'available',
}

describe('companion service', () => {
  afterEach(() => {
    clearAllowedPaths()
  })

  it('detects providers with the persisted custom command', async () => {
    const detectProviders = vi.fn(async () => [availableOpencode])
    const service = createCompanionService({
      detectProviders,
      createClient: vi.fn(),
      buildContext: vi.fn(),
      getSettings: () => ({ provider: 'auto', customCommand: 'custom acp' }),
      emitUpdate: vi.fn(),
    })

    await service.detectProviders()

    expect(detectProviders).toHaveBeenCalledWith('custom acp')
  })

  it('builds context, starts an ACP client, streams status, and sends prompt blocks', async () => {
    const folderPath = await mkdtemp(join(tmpdir(), 'mdow-companion-'))
    const activePath = join(folderPath, 'README.md')
    await writeFile(activePath, '# Mdow')
    registerAllowedPath(folderPath)

    const sendPrompt = vi.fn(async () => {})
    const start = vi.fn(async () => {})
    const createClient = vi.fn(() => ({ start, sendPrompt, cancel: vi.fn(), stop: vi.fn() }))
    const emitUpdate = vi.fn()
    const request: CompanionSendRequest = {
      messageId: 'msg_1',
      text: 'What is Mdow?',
      provider: 'opencode',
      activePath,
      openFolderPath: folderPath,
    }
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient,
      buildContext: vi.fn(async () => ({
        sources: [
          {
            id: 'src_active',
            title: 'README.md',
            path: activePath,
            text: '# Mdow',
            truncated: false,
            chars: 6,
          },
        ],
        summary: {
          activePath,
          folderPath,
          sourceCount: 1,
          truncated: false,
          warnings: [],
          sources: [{ id: 'src_active', title: 'README.md', path: activePath }],
        },
      })),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    await service.send(request)

    expect(createClient).toHaveBeenCalledWith({
      command: 'opencode',
      args: ['acp'],
      cwd: folderPath,
      onUpdate: expect.any(Function),
    })
    expect(start).toHaveBeenCalledOnce()
    expect(sendPrompt).toHaveBeenCalledWith(
      expect.arrayContaining([{ type: 'text', text: expect.stringContaining('What is Mdow?') }]),
    )
    expect(emitUpdate).toHaveBeenCalledWith({ type: 'status', status: 'starting' })
    expect(emitUpdate).toHaveBeenCalledWith(expect.objectContaining({ type: 'context' }))
    expect(emitUpdate).toHaveBeenCalledWith({ type: 'status', status: 'streaming' })
    expect(emitUpdate).toHaveBeenCalledWith({ type: 'status', status: 'complete' })

    await rm(folderPath, { recursive: true, force: true })
  })

  it('falls back to the process cwd when raw request paths are not allowed', async () => {
    const createClient = vi.fn(() => ({
      start: vi.fn(async () => {}),
      sendPrompt: vi.fn(async () => {}),
      cancel: vi.fn(),
      stop: vi.fn(),
    }))
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient,
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate: vi.fn(),
    })

    await service.send({
      messageId: 'msg_1',
      text: 'What can you see?',
      provider: 'opencode',
      activePath: '/private/docs/README.md',
      openFolderPath: '/private/docs',
    })

    expect(createClient).toHaveBeenCalledWith(expect.objectContaining({ cwd: process.cwd() }))
  })

  it('chooses the first available provider when provider is auto', async () => {
    const createClient = vi.fn(() => ({
      start: vi.fn(async () => {}),
      sendPrompt: vi.fn(async () => {}),
      cancel: vi.fn(),
      stop: vi.fn(),
    }))
    const service = createCompanionService({
      detectProviders: vi.fn(async () => [
        { ...availableOpencode, status: 'missing' as const },
        availableCodex,
      ]),
      createClient,
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate: vi.fn(),
    })

    await service.send({
      messageId: 'msg_1',
      text: 'What can you see?',
      provider: 'auto',
      activePath: null,
      openFolderPath: null,
    })

    expect(createClient).toHaveBeenCalledWith(
      expect.objectContaining({
        command: 'npx',
        args: ['--no-install', '@zed-industries/codex-acp'],
      }),
    )
  })

  it('forwards assistant deltas with the request message id', async () => {
    type CreateClient = Parameters<typeof createCompanionService>[0]['createClient']
    let onUpdate: Parameters<CreateClient>[0]['onUpdate']
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn((options) => {
        onUpdate = options.onUpdate
        return {
          start: vi.fn(async () => {}),
          sendPrompt: vi.fn(async () => {
            onUpdate({ type: 'assistant-delta', text: 'Hello' })
          }),
          cancel: vi.fn(),
          stop: vi.fn(),
        }
      }),
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    await service.send({
      messageId: 'assistant_1',
      text: 'Hi',
      provider: 'opencode',
      activePath: null,
      openFolderPath: null,
    })

    expect(emitUpdate).toHaveBeenCalledWith({
      type: 'assistant-delta',
      messageId: 'assistant_1',
      text: 'Hello',
    })
  })

  it('cancel and shutdown delegate to the active client', async () => {
    const cancel = vi.fn()
    const stop = vi.fn()
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({
        start: vi.fn(async () => {}),
        sendPrompt: vi.fn(async () => {}),
        cancel,
        stop,
      })),
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    await service.send({
      messageId: 'msg_1',
      text: 'Hi',
      provider: 'opencode',
      activePath: null,
      openFolderPath: null,
    })

    service.cancel()
    service.shutdown()

    expect(cancel).toHaveBeenCalledOnce()
    expect(emitUpdate).toHaveBeenCalledWith({ type: 'status', status: 'cancelled' })
    expect(stop).toHaveBeenCalledOnce()
  })

  it('rejects concurrent sends with an error update', async () => {
    let resolvePrompt: () => void = () => {}
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({
        start: vi.fn(async () => {}),
        sendPrompt: vi.fn(
          () =>
            new Promise<void>((resolve) => {
              resolvePrompt = resolve
            }),
        ),
        cancel: vi.fn(),
        stop: vi.fn(),
      })),
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    const firstSend = service.send({
      messageId: 'msg_1',
      text: 'First',
      provider: 'opencode',
      activePath: null,
      openFolderPath: null,
    })

    await expect(
      service.send({
        messageId: 'msg_2',
        text: 'Second',
        provider: 'opencode',
        activePath: null,
        openFolderPath: null,
      }),
    ).rejects.toThrow('A companion response is already in progress.')

    expect(emitUpdate).toHaveBeenCalledWith({
      type: 'error',
      message: 'A companion response is already in progress.',
    })

    resolvePrompt()
    await firstSend
  })

  it('suppresses complete status after cancellation', async () => {
    let resolvePrompt: () => void = () => {}
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({
        start: vi.fn(async () => {}),
        sendPrompt: vi.fn(
          () =>
            new Promise<void>((resolve) => {
              resolvePrompt = resolve
            }),
        ),
        cancel: vi.fn(),
        stop: vi.fn(),
      })),
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    const send = service.send({
      messageId: 'msg_1',
      text: 'Hi',
      provider: 'opencode',
      activePath: null,
      openFolderPath: null,
    })

    service.cancel()
    resolvePrompt()
    await send

    const cancelledCall = emitUpdate.mock.calls.find(
      ([update]) => update.type === 'status' && update.status === 'cancelled',
    )
    const completeAfterCancel = emitUpdate.mock.calls
      .slice(cancelledCall ? emitUpdate.mock.calls.indexOf(cancelledCall) + 1 : 0)
      .some(([update]) => update.type === 'status' && update.status === 'complete')

    expect(cancelledCall).toBeTruthy()
    expect(completeAfterCancel).toBe(false)
  })
})

function emptyContext() {
  return {
    sources: [],
    summary: {
      activePath: null,
      folderPath: null,
      sourceCount: 0,
      truncated: false,
      warnings: [],
      sources: [],
    },
  }
}
