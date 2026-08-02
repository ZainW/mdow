import { describe, expect, it, vi } from 'vitest'

const runtime = vi.hoisted(() => ({ configured: false }))

vi.mock('fs', () => ({
  existsSync: vi.fn(() => true),
}))

vi.mock('electron-store', () => ({
  default: class TestStore {
    private values = new Map<string, unknown>()

    constructor(options: { defaults: Record<string, unknown> }) {
      if (!runtime.configured) {
        throw new Error('store initialized before runtime paths were configured')
      }
      this.values = new Map(Object.entries(options.defaults))
    }

    get(key: string) {
      return this.values.get(key)
    }

    set(key: string, value: unknown) {
      this.values.set(key, value)
    }
  },
}))

describe('store initialization', () => {
  it('defers opening the store until after runtime paths can be configured', async () => {
    const storeModule = await import('./store')

    runtime.configured = true

    expect(storeModule.getAppState().theme).toBe('system')
  })
})
