import { afterEach, beforeEach, vi } from 'vitest'

export function createMinimalWindowApi(
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    saveAppState: vi.fn().mockResolvedValue(undefined),
    unwatchFile: vi.fn().mockResolvedValue(undefined),
    setTheme: vi.fn().mockResolvedValue(undefined),
    readFile: vi.fn(),
    showInFolder: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  }
}

// Install a partial window.api stub for the duration of the surrounding
// describe block, then restore whatever was there before (typically
// `undefined` in jsdom). Tests that mutate window.api should always pair
// the install with a restore so the stub doesn't leak across files.
export function stubWindowApi(getStub: () => Record<string, unknown>) {
  let original: unknown
  beforeEach(() => {
    original = (globalThis.window as { api?: unknown }).api
    // @ts-expect-error — minimal stub for unit tests
    globalThis.window.api = getStub()
  })
  afterEach(() => {
    if (original === undefined) {
      // @ts-expect-error — restoring jsdom's lack of window.api
      delete globalThis.window.api
    } else {
      ;(globalThis.window as { api?: unknown }).api = original
    }
  })
}
