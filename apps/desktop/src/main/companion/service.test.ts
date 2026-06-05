import { describe, expect, it, vi } from 'vitest'
import type { CompanionProviderStatus, CompanionSendRequest } from '../../shared/types'
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
    const sendPrompt = vi.fn(async () => {})
    const start = vi.fn(async () => {})
    const createClient = vi.fn(() => ({ start, sendPrompt, cancel: vi.fn(), stop: vi.fn() }))
    const emitUpdate = vi.fn()
    const request: CompanionSendRequest = {
      messageId: 'msg_1',
      text: 'What is Mdow?',
      provider: 'opencode',
      activePath: '/docs/README.md',
      openFolderPath: '/docs',
    }
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient,
      buildContext: vi.fn(async () => ({
        sources: [
          {
            id: 'src_active',
            title: 'README.md',
            path: '/docs/README.md',
            text: '# Mdow',
            truncated: false,
            chars: 6,
          },
        ],
        summary: {
          activePath: '/docs/README.md',
          folderPath: '/docs',
          sourceCount: 1,
          truncated: false,
          warnings: [],
          sources: [{ id: 'src_active', title: 'README.md', path: '/docs/README.md' }],
        },
      })),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    await service.send(request)

    expect(createClient).toHaveBeenCalledWith({
      command: 'opencode',
      args: ['acp'],
      cwd: '/docs',
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
