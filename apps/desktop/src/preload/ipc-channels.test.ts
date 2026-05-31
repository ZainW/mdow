import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { IPC } from '../shared/types'

function extractPreloadInvokeChannels(source: string): string[] {
  const channels: string[] = []
  const pattern = /ipcRenderer\.invoke\(IPC\.([A-Z_]+)/g
  for (const match of source.matchAll(pattern)) {
    const key = match[1] as keyof typeof IPC
    if (key in IPC) channels.push(IPC[key])
  }
  return [...new Set(channels)].sort()
}

function extractMainHandleChannels(source: string): string[] {
  const channels: string[] = []
  const pattern = /ipcMain\.handle\(\s*['"]([^'"]+)['"]/g
  for (const match of source.matchAll(pattern)) {
    channels.push(match[1])
  }
  return [...new Set(channels)].sort()
}

const MAIN_ONLY_INVOKE_CHANNELS = ['file:stat']

describe('ipc channel parity', () => {
  it('ensures every preload invoke channel is handled in main', () => {
    const preloadDir = resolve(import.meta.dirname)
    const preloadSource = readFileSync(resolve(preloadDir, 'index.ts'), 'utf-8')
    const mainSource = readFileSync(resolve(preloadDir, '../main/ipc.ts'), 'utf-8')

    const preloadChannels = extractPreloadInvokeChannels(preloadSource)
    const mainChannels = extractMainHandleChannels(mainSource)

    const missingInMain = preloadChannels.filter((channel) => !mainChannels.includes(channel))
    expect(missingInMain, `missing ipcMain.handle for: ${missingInMain.join(', ')}`).toEqual([])
  })

  it('ensures every main handler is exposed through preload invoke', () => {
    const preloadDir = resolve(import.meta.dirname)
    const preloadSource = readFileSync(resolve(preloadDir, 'index.ts'), 'utf-8')
    const mainSource = readFileSync(resolve(preloadDir, '../main/ipc.ts'), 'utf-8')

    const preloadChannels = extractPreloadInvokeChannels(preloadSource)
    const mainChannels = extractMainHandleChannels(mainSource)

    const missingInPreload = mainChannels.filter(
      (channel) =>
        !preloadChannels.includes(channel) && !MAIN_ONLY_INVOKE_CHANNELS.includes(channel),
    )
    expect(missingInPreload, `missing preload invoke for: ${missingInPreload.join(', ')}`).toEqual(
      [],
    )
  })
})
