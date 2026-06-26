import { afterEach, describe, expect, it, vi } from 'vitest'
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises'
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

  it('falls back to the process cwd for symlinked folder roots', async () => {
    const basePath = await mkdtemp(join(tmpdir(), 'mdow-companion-cwd-'))
    const targetPath = await mkdtemp(join(tmpdir(), 'mdow-companion-target-'))
    const linkPath = join(basePath, 'linked-folder')
    await symlink(targetPath, linkPath, 'dir')
    registerAllowedPath(basePath)

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

    try {
      await service.send({
        messageId: 'msg_1',
        text: 'What can you see?',
        provider: 'opencode',
        activePath: null,
        openFolderPath: linkPath,
      })

      expect(createClient).toHaveBeenCalledWith(expect.objectContaining({ cwd: process.cwd() }))
    } finally {
      await rm(basePath, { recursive: true, force: true })
      await rm(targetPath, { recursive: true, force: true })
    }
  })

  it('falls back to the process cwd for active files under intermediate symlinks', async () => {
    const basePath = await mkdtemp(join(tmpdir(), 'mdow-companion-cwd-'))
    const targetPath = await mkdtemp(join(tmpdir(), 'mdow-companion-target-'))
    const linkPath = join(basePath, 'linked-folder')
    const activePath = join(linkPath, 'README.md')
    await mkdir(targetPath, { recursive: true })
    await writeFile(join(targetPath, 'README.md'), '# Secret')
    await symlink(targetPath, linkPath, 'dir')
    registerAllowedPath(basePath)

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

    try {
      await service.send({
        messageId: 'msg_1',
        text: 'What can you see?',
        provider: 'opencode',
        activePath,
        openFolderPath: null,
      })

      expect(createClient).toHaveBeenCalledWith(expect.objectContaining({ cwd: process.cwd() }))
    } finally {
      await rm(basePath, { recursive: true, force: true })
      await rm(targetPath, { recursive: true, force: true })
    }
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

  it('ignores assistant deltas that arrive after send completes', async () => {
    type CreateClient = Parameters<typeof createCompanionService>[0]['createClient']
    let onUpdate!: Parameters<CreateClient>[0]['onUpdate']
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn((options) => {
        onUpdate = options.onUpdate
        return {
          start: vi.fn(async () => {}),
          sendPrompt: vi.fn(async () => {}),
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

    const callCount = emitUpdate.mock.calls.length
    onUpdate({ type: 'assistant-delta', text: 'Late' })

    expect(emitUpdate.mock.calls).toHaveLength(callCount)
  })

  it('ignores assistant deltas that arrive after cancel settles', async () => {
    type CreateClient = Parameters<typeof createCompanionService>[0]['createClient']
    let onUpdate!: Parameters<CreateClient>[0]['onUpdate']
    let resolvePrompt: () => void = () => {}
    const promptStarted = deferred<void>()
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn((options) => {
        onUpdate = options.onUpdate
        return {
          start: vi.fn(async () => {}),
          sendPrompt: vi.fn(() => {
            promptStarted.resolve()
            return new Promise<void>((resolve) => {
              resolvePrompt = resolve
            })
          }),
          cancel: vi.fn(),
          stop: vi.fn(),
        }
      }),
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    const send = service.send({
      messageId: 'assistant_1',
      text: 'Hi',
      provider: 'opencode',
      activePath: null,
      openFolderPath: null,
    })

    await promptStarted.promise
    service.cancel()
    resolvePrompt()
    await send

    const callCount = emitUpdate.mock.calls.length
    onUpdate({ type: 'assistant-delta', text: 'Late' })

    expect(emitUpdate.mock.calls).toHaveLength(callCount)
  })

  it('does not attribute late updates from a previous client to the active message', async () => {
    type CreateClient = Parameters<typeof createCompanionService>[0]['createClient']
    const onUpdates: Array<Parameters<CreateClient>[0]['onUpdate']> = []
    const secondPromptStarted = deferred<void>()
    let resolveSecondPrompt: () => void = () => {}
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn((options) => {
        onUpdates.push(options.onUpdate)
        const sendPrompt = vi.fn(async () => {
          if (onUpdates.length === 2) {
            secondPromptStarted.resolve()
            await new Promise<void>((resolve) => {
              resolveSecondPrompt = resolve
            })
          }
        })
        return {
          start: vi.fn(async () => {}),
          sendPrompt,
          cancel: vi.fn(),
          stop: vi.fn(),
        }
      }),
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    await service.send({
      messageId: 'msg_1',
      text: 'First',
      provider: 'opencode',
      activePath: null,
      openFolderPath: null,
    })

    const secondSend = service.send({
      messageId: 'msg_2',
      text: 'Second',
      provider: 'opencode',
      activePath: null,
      openFolderPath: null,
    })
    await secondPromptStarted.promise

    onUpdates[0]({ type: 'assistant-delta', text: 'late' })
    resolveSecondPrompt()
    await secondSend

    expect(emitUpdate).not.toHaveBeenCalledWith({
      type: 'assistant-delta',
      messageId: 'msg_2',
      text: 'late',
    })
  })

  it('cancel and shutdown delegate to the active client', async () => {
    let resolvePrompt: () => void = () => {}
    const promptStarted = deferred<void>()
    const cancel = vi.fn()
    const stop = vi.fn()
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({
        start: vi.fn(async () => {}),
        sendPrompt: vi.fn(() => {
          promptStarted.resolve()
          return new Promise<void>((resolve) => {
            resolvePrompt = resolve
          })
        }),
        cancel,
        stop,
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

    await promptStarted.promise
    service.cancel()
    resolvePrompt()
    await send
    service.shutdown()

    expect(cancel).toHaveBeenCalledOnce()
    expect(emitUpdate).toHaveBeenCalledWith({ type: 'status', status: 'cancelled' })
    expect(stop).toHaveBeenCalledOnce()
  })

  it('idle cancel emits nothing and does not call the last client', async () => {
    const cancel = vi.fn()
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({
        start: vi.fn(async () => {}),
        sendPrompt: vi.fn(async () => {}),
        cancel,
        stop: vi.fn(),
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
    emitUpdate.mockClear()

    service.cancel()

    expect(cancel).not.toHaveBeenCalled()
    expect(emitUpdate).not.toHaveBeenCalled()
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

  it('allows a new send after cancellation stops an unresponsive prompt', async () => {
    let rejectPrompt: (reason?: unknown) => void = () => {}
    const sendPrompt = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<void>((_resolve, reject) => {
            rejectPrompt = reject
          }),
      )
      .mockResolvedValueOnce(undefined)
    const stop = vi.fn(() => rejectPrompt(new Error('ACP client stopped')))
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({
        start: vi.fn(async () => {}),
        sendPrompt,
        cancel: vi.fn(),
        stop,
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
    await vi.waitFor(() => expect(sendPrompt).toHaveBeenCalledOnce())

    service.cancel()
    await firstSend

    await expect(
      service.send({
        messageId: 'msg_2',
        text: 'Second',
        provider: 'opencode',
        activePath: null,
        openFolderPath: null,
      }),
    ).resolves.toBeUndefined()

    expect(stop).toHaveBeenCalled()
    expect(sendPrompt).toHaveBeenCalledTimes(2)
  })

  it('allows a new send after cancellation stops a client before start completes', async () => {
    let rejectStart: (reason?: unknown) => void = () => {}
    const start = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<void>((_resolve, reject) => {
            rejectStart = reject
          }),
      )
      .mockResolvedValueOnce(undefined)
    const sendPrompt = vi.fn(async () => {})
    const stop = vi.fn(() => rejectStart(new Error('ACP client stopped')))
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({
        start,
        sendPrompt,
        cancel: vi.fn(),
        stop,
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
    await vi.waitFor(() => expect(start).toHaveBeenCalledOnce())

    service.cancel()
    await firstSend

    await expect(
      service.send({
        messageId: 'msg_2',
        text: 'Second',
        provider: 'opencode',
        activePath: null,
        openFolderPath: null,
      }),
    ).resolves.toBeUndefined()

    expect(stop).toHaveBeenCalled()
    expect(start).toHaveBeenCalledTimes(2)
    expect(sendPrompt).toHaveBeenCalledOnce()
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

  it('does not create a client when shutdown happens while context is building', async () => {
    const context = deferred<ReturnType<typeof emptyContext>>()
    const createClient = vi.fn(() => ({
      start: vi.fn(async () => {}),
      sendPrompt: vi.fn(async () => {}),
      cancel: vi.fn(),
      stop: vi.fn(),
    }))
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient,
      buildContext: vi.fn(() => context.promise),
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

    service.shutdown()
    context.resolve(emptyContext())
    await send

    expect(createClient).not.toHaveBeenCalled()
    expect(hasStatusAfter(emitUpdate, 'complete', 0)).toBe(false)
  })

  it('does not create a client when cancellation happens during provider detection', async () => {
    const detected = deferred<CompanionProviderStatus[]>()
    const detectStarted = deferred<void>()
    const createClient = vi.fn(() => ({
      start: vi.fn(async () => {}),
      sendPrompt: vi.fn(async () => {}),
      cancel: vi.fn(),
      stop: vi.fn(),
    }))
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(() => {
        detectStarted.resolve()
        return detected.promise
      }),
      createClient,
      buildContext: vi.fn(async () => emptyContext()),
      getSettings: () => ({ provider: 'auto', customCommand: '' }),
      emitUpdate,
    })

    const send = service.send({
      messageId: 'msg_1',
      text: 'Hi',
      provider: 'auto',
      activePath: null,
      openFolderPath: null,
    })
    await detectStarted.promise

    service.cancel()
    detected.resolve([availableOpencode])
    await send

    expect(createClient).not.toHaveBeenCalled()
    expect(hasStatusAfter(emitUpdate, 'complete', 0)).toBe(false)
  })

  it('stops a created client when cancellation happens while the client is starting', async () => {
    const start = deferred<void>()
    const startStarted = deferred<void>()
    const stop = vi.fn()
    const sendPrompt = vi.fn(async () => {})
    const emitUpdate = vi.fn()
    const service = createCompanionService({
      detectProviders: vi.fn(async () => []),
      createClient: vi.fn(() => ({
        start: vi.fn(() => {
          startStarted.resolve()
          return start.promise
        }),
        sendPrompt,
        cancel: vi.fn(),
        stop,
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
    await startStarted.promise

    service.cancel()
    start.resolve()
    await send

    expect(stop).toHaveBeenCalledOnce()
    expect(sendPrompt).not.toHaveBeenCalled()
    expect(hasStatusAfter(emitUpdate, 'complete', 0)).toBe(false)
  })
})

function deferred<T>() {
  let resolve: (value: T) => void = () => {}
  let reject: (reason?: unknown) => void = () => {}
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve
    reject = promiseReject
  })
  return { promise, resolve, reject }
}

function hasStatusAfter(
  emitUpdate: ReturnType<typeof vi.fn>,
  status: 'starting' | 'ready' | 'streaming' | 'complete' | 'cancelled',
  index: number,
) {
  return emitUpdate.mock.calls
    .slice(index)
    .some(([update]) => update.type === 'status' && update.status === status)
}

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
